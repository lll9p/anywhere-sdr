use std::{
    fs::File,
    io::{BufWriter, Write},
    path::PathBuf,
};

use anyhow::Result;

pub use super::utils::{DataFormat, MotionMode};
use crate::{
    channel::Channel,
    constants::*,
    datetime::{DateTime, GpsTime},
    eph::Ephemeris,
    ionoutc::IonoUtc,
    table::{ANT_PAT_DB, COS_TABLE512, SIN_TABLE512},
    utils::{allocate_channel, compute_range},
};
#[derive(Debug)]
pub struct SignalGenerator {
    pub ephemerides: Box<[[Ephemeris; MAX_SAT]; EPHEM_ARRAY_SIZE]>,
    pub valid_ephemerides_index: usize,
    pub channels: [Channel; MAX_CHAN],
    pub ionoutc: IonoUtc,
    pub allocated_satellite: [i32; MAX_SAT],
    /// posisions of receiver, per 100ms
    pub positions: Vec<[f64; 3]>,
    pub user_motion_count: usize,
    pub receiver_gps_time: GpsTime,
    pub antenna_gains: [i32; MAX_CHAN],
    pub antenna_pattern: [f64; 37],
    pub mode: MotionMode,
    pub elvmask: f64,          // set to 0.0
    pub sample_frequency: f64, // 26000000
    pub sample_rate: f64,      // samples per 0.1sec
    pub data_format: DataFormat,
    // when is some , disable path loss
    pub fixed_gain: Option<i32>,
    pub iq_buffer_size: usize,
    pub iq_buffer: Vec<i16>,
    pub out_file: Option<PathBuf>,
    pub initialized: bool,
    pub verbose: bool,
}
impl Default for SignalGenerator {
    fn default() -> Self {
        Self {
            ephemerides: Box::default(),
            valid_ephemerides_index: usize::default(),
            channels: std::array::from_fn(|_| Channel::default()),
            ionoutc: IonoUtc::default(),
            allocated_satellite: [0; MAX_SAT],
            positions: Vec::new(),
            user_motion_count: usize::default(),
            receiver_gps_time: GpsTime::default(),
            antenna_gains: [0; MAX_CHAN],
            antenna_pattern: [0.0; 37],
            mode: MotionMode::Static,
            elvmask: f64::default(),
            sample_frequency: 0.0,
            sample_rate: 0.0,
            data_format: DataFormat::Bits8,
            fixed_gain: None,
            iq_buffer_size: 0,
            iq_buffer: Vec::new(),
            out_file: None,
            initialized: false,
            verbose: true,
        }
    }
}
impl SignalGenerator {
    pub fn initiallize(&mut self) {
        // Initialize channels
        let chan = &mut self.channels;
        let allocated_sat = &mut self.allocated_satellite;
        let ephemerides = &mut self.ephemerides;
        let ieph = self.valid_ephemerides_index;
        let ionoutc = &mut self.ionoutc;
        match self.mode {
            MotionMode::Static => eprintln!("Using static location mode."),
            MotionMode::Dynamic => eprintln!("Using dynamic location mode."),
        }

        eprintln!(
            "xyz = {}, {}, {}",
            self.positions[0][0], self.positions[0][1], self.positions[0][2],
        );
        let g0 = self.receiver_gps_time.clone();
        let t0 = DateTime::from(&g0);
        eprintln!(
            "Start time = {:4}/{:02}/{:02},{:02}:{:02}:{:0>2.0} ({}:{:.0})",
            t0.y, t0.m, t0.d, t0.hh, t0.mm, t0.sec, g0.week, g0.sec,
        );
        // Clear all channels
        chan.iter_mut().take(MAX_CHAN).for_each(|ch| ch.prn = 0);
        // Clear satellite allocation flag
        allocated_sat.iter_mut().take(MAX_SAT).for_each(|s| *s = -1);
        // Initial reception time
        self.receiver_gps_time = self.receiver_gps_time.add_secs(0.0);
        // Allocate visible satellites
        allocate_channel(
            chan,
            &mut ephemerides[ieph],
            ionoutc,
            &self.receiver_gps_time,
            &self.positions[0],
            self.elvmask,
            allocated_sat,
        );

        for ichan in chan.iter().take(MAX_CHAN) {
            if ichan.prn > 0 {
                eprintln!(
                    "{:02} {:6.1} {:5.1} {:11.1} {:5.1}",
                    ichan.prn,
                    ichan.azel[0] * R2D,
                    ichan.azel[1] * R2D,
                    ichan.rho0.distance,
                    ichan.rho0.iono_delay,
                );
            }
        }

        ////////////////////////////////////////////////////////////
        // Receiver antenna gain pattern
        ////////////////////////////////////////////////////////////
        // for i in 0..37 {
        for (i, item) in self.antenna_pattern.iter_mut().take(37).enumerate() {
            *item = 10.0f64.powf(-ANT_PAT_DB[i] / 20.0);
        }

        self.iq_buffer_size =
            (self.sample_frequency * self.sample_rate).floor() as usize;
        self.iq_buffer = vec![0; 2 * self.iq_buffer_size];
        self.initialized = true;
    }

    #[allow(clippy::too_many_lines)]
    pub fn generate(&mut self) -> Result<()> {
        if !self.initialized {
            anyhow::bail!("Not initialized!");
        }
        let file = File::create(self.out_file.as_ref().unwrap())?;
        let mut file = BufWriter::new(file);
        // Generate baseband signals
        const INTERVAL: f64 = 0.1;
        self.receiver_gps_time = self.receiver_gps_time.add_secs(INTERVAL);
        let channels = &mut self.channels;
        let ephemerides = &mut self.ephemerides;
        let ieph = &mut self.valid_ephemerides_index;
        let receiver_gps_time = &mut self.receiver_gps_time;
        let ionoutc = &mut self.ionoutc;
        let antenna_gains = &mut self.antenna_gains;

        let iq_buff_size = self.iq_buffer_size;
        let sampling_period = self.sample_frequency.recip();

        let time_start = std::time::Instant::now();
        // 主循环：遍历每个时间间隔（0.1秒）
        for user_motion_index in 1..self.user_motion_count {
            // 根据静态/动态模式选择接收机位置
            let current_location = match self.mode {
                MotionMode::Static => &self.positions[0],
                MotionMode::Dynamic => &self.positions[user_motion_index],
            };
            // 第一步：更新所有通道的伪距、相位和增益参数
            for i in 0..MAX_CHAN {
                // 仅处理已分配卫星的通道
                if channels[i].prn > 0 {
                    // 卫星PRN号转索引
                    let sv = channels[i].prn - 1;
                    // 计算当前时刻的伪距（传播时延）
                    // Refresh code phase and data bit counters

                    // Current pseudorange
                    let rho = compute_range(
                        &ephemerides[*ieph][sv],
                        ionoutc,
                        receiver_gps_time,
                        current_location,
                    );

                    // 更新方位角/仰角信息
                    // Update code phase and data bit counters
                    channels[i].azel.copy_from_slice(&rho.azel);
                    // 计算码相位（C/A码偏移）
                    channels[i].compute_code_phase(&rho, INTERVAL);
                    channels[i].carr_phasestep = (512.0
                        * 65536.0
                        * channels[i].f_carr
                        * sampling_period)
                        .round()
                        as i32;

                    // Path loss
                    let path_loss = 20_200_000.0 / rho.distance;
                    // Receiver antenna gain
                    let ibs = ((90.0 - rho.azel[1] * R2D) / 5.0) as usize; // covert elevation to boresight
                    let ant_gain = self.antenna_pattern[ibs];
                    // 计算信号增益（考虑路径损耗和天线方向图）
                    // Signal gain
                    // 应用增益模式选择
                    if let Some(fixed_gain) = self.fixed_gain {
                        // 固定增益模式
                        antenna_gains[i] = fixed_gain; // hold the power level constant
                    } else {
                        // 带路径损耗补偿
                        antenna_gains[i] =
                            (path_loss * ant_gain * 128.0) as i32; // scaled by 2^7
                    }
                }
            }
            // 第二步：生成基带I/Q采样数据
            for isamp in 0..iq_buff_size {
                let mut i_acc: i32 = 0;
                let mut q_acc: i32 = 0;
                // 第三步：累加所有通道的信号分量
                for i in 0..MAX_CHAN {
                    if channels[i].prn > 0 {
                        // 仅处理有效通道
                        // #ifdef FLOAT_CARR_PHASE
                        //                     iTable =
                        // (int)floor(chan[i].carr_phase*512.0);
                        // #else
                        // 使用预计算的正弦/余弦表生成载波
                        let i_table =
                            (channels[i].carr_phase >> 16 & 0x1ff) as usize; // 9-bit index
                        // 生成I/Q分量（考虑导航数据位和C/A码）
                        let ip = channels[i].dataBit
                            * channels[i].codeCA
                            * COS_TABLE512[i_table]
                            * antenna_gains[i];
                        let qp = channels[i].dataBit
                            * channels[i].codeCA
                            * SIN_TABLE512[i_table]
                            * antenna_gains[i];
                        // Accumulate for all visible satellites
                        // 累加到总信号
                        i_acc += ip;
                        q_acc += qp;
                        // Update code phase
                        // 第四步：更新码相位（C/A码序列控制）
                        channels[i].code_phase +=
                            channels[i].f_code * sampling_period;
                        if channels[i].code_phase >= CA_SEQ_LEN as f64 {
                            channels[i].code_phase -= CA_SEQ_LEN as f64;
                            channels[i].icode += 1;
                            if channels[i].icode >= 20 {
                                // 20 C/A codes = 1 navigation data bit
                                // 处理导航数据位（每20个C/A码周期）
                                channels[i].icode = 0;
                                channels[i].ibit += 1;
                                // 处理导航字（每30个数据位）
                                if channels[i].ibit >= 30 {
                                    // 30 navigation data bits = 1 word
                                    channels[i].ibit = 0;
                                    channels[i].iword += 1;

                                    /*
                                                                        if (chan[i].iword>=N_DWRD)
                                                                            fprintf(stderr, "\nWARNING: Subframe word buffer overflow.\n");
                                    */
                                }
                                // 提取当前导航数据位
                                // Set new navigation data bit
                                channels[i].dataBit = (channels[i].dwrd
                                    [channels[i].iword as usize]
                                    >> (29 - channels[i].ibit)
                                    & 0x1)
                                    as i32
                                    * 2
                                    - 1;
                            }
                        }
                        // 更新当前C/A码片
                        // Set current code chip
                        channels[i].codeCA = channels[i].ca
                            [channels[i].code_phase as i32 as usize]
                            * 2_i32
                            - 1_i32;
                        // Update carrier phase
                        // #ifdef FLOAT_CARR_PHASE
                        //                     chan[i].carr_phase +=
                        // chan[i].f_carr
                        // * sampling_period;
                        //
                        //                     if (chan[i].carr_phase >= 1.0)
                        //                         chan[i].carr_phase -= 1.0;
                        //                     else if (chan[i].carr_phase<0.0)
                        //                         chan[i].carr_phase += 1.0;
                        // #else
                        // 第五步：更新载波相位（使用相位累加器）
                        channels[i].carr_phase = (channels[i].carr_phase)
                            .wrapping_add(channels[i].carr_phasestep as u32);
                    }
                }
                // 第六步：量化并存储I/Q采样
                // Scaled by 2^7
                // i_acc = (i_acc + 64) >> 7;
                // q_acc = (q_acc + 64) >> 7;
                // Store I/Q samples into buffer
                self.iq_buffer[isamp * 2] = ((i_acc + 64) >> 7) as i16; // 8位量化（带舍入）
                self.iq_buffer[isamp * 2 + 1] = ((q_acc + 64) >> 7) as i16;
            }

            // 第七步：将I/Q数据写入输出文件（不同格式处理）

            match self.data_format {
                DataFormat::Bits1 => {
                    let mut iq8_buff = vec![0; iq_buff_size / 4];
                    for isamp in 0..2 * iq_buff_size {
                        if isamp % 8 == 0 {
                            iq8_buff[isamp / 8] = 0;
                        }
                        let curr_bit = &mut iq8_buff[isamp / 8];

                        *curr_bit = (i32::from(*curr_bit)
                            | i32::from(i32::from(self.iq_buffer[isamp]) > 0)
                                << (7 - isamp as i32 % 8))
                            as i8;
                    }

                    unsafe {
                        file.write_all(std::slice::from_raw_parts(
                            iq8_buff.as_ptr().cast::<u8>(),
                            iq_buff_size / 4,
                        ))?;
                    }
                }
                DataFormat::Bits8 => {
                    let mut iq8_buff = vec![0; 2 * iq_buff_size];
                    for (isamp, buff) in iq8_buff.iter_mut().enumerate() {
                        *buff = (i32::from(self.iq_buffer[isamp]) >> 4) as i8;
                        // 12-bit bladeRF -> 8-bit HackRF
                        //iq8_buff[isamp] = iq_buff[isamp] >> 8; // for
                        // PocketSDR
                    }

                    unsafe {
                        file.write_all(std::slice::from_raw_parts(
                            iq8_buff.as_ptr().cast::<u8>(),
                            2 * iq_buff_size,
                        ))?;
                    }
                }
                DataFormat::Bits16 => {
                    // data_format==SC16
                    let byte_slice = unsafe {
                        std::slice::from_raw_parts(
                            self.iq_buffer.as_ptr().cast::<u8>(),
                            2 * iq_buff_size * 2, // 2 bytes per sample
                        )
                    };
                    file.write_all(byte_slice)?;
                }
            }

            //
            // Update navigation message and channel allocation every 30 seconds
            //
            // 第八步：定期更新导航信息（每30秒）
            let igrx = (receiver_gps_time.sec * 10.0 + 0.5) as i32;
            if igrx % 300 == 0 {
                // Every 30 seconds
                for ichan in channels.iter_mut().take(MAX_CHAN) {
                    if ichan.prn > 0 {
                        ichan.generate_nav_msg(receiver_gps_time, false);
                    }
                }
                // Refresh ephemeris and subframes
                // Quick and dirty fix. Need more elegant way.
                for sv in 0..MAX_SAT {
                    if ephemerides[*ieph + 1][sv].vflg {
                        let dt = ephemerides[*ieph + 1][sv]
                            .toc
                            .diff_secs(receiver_gps_time);
                        if dt < SECONDS_IN_HOUR {
                            // move next set of ephemeris
                            *ieph += 1;
                            for ichan in channels.iter_mut().take(MAX_CHAN) {
                                // Generate new subframes if allocated
                                if ichan.prn != 0 {
                                    ephemerides[*ieph][ichan.prn - 1]
                                        .generate_navigation_subframes(
                                            ionoutc,
                                            &mut ichan.sbf,
                                        );
                                }
                            }
                        }
                        break;
                    }
                }
                // Update channel allocation
                allocate_channel(
                    channels,
                    &mut ephemerides[*ieph],
                    ionoutc,
                    receiver_gps_time,
                    current_location,
                    self.elvmask,
                    &mut self.allocated_satellite,
                );

                // Show details about simulated channels
                if self.verbose {
                    eprintln!();
                    for ichan in channels.iter().take(MAX_CHAN) {
                        if ichan.prn > 0 {
                            eprintln!(
                                "{:02} {:6.1} {:5.1} {:11.1} {:5.1}",
                                ichan.prn,
                                ichan.azel[0] * R2D,
                                ichan.azel[1] * R2D,
                                ichan.rho0.distance,
                                ichan.rho0.iono_delay,
                            );
                        }
                    }
                }
            }
            // 第九步：更新时间并显示进度
            // Update receiver time
            *receiver_gps_time = receiver_gps_time.add_secs(INTERVAL);
            eprint!(
                "\rTime into run = {:4.1}\0",
                (user_motion_index + 1) as f64 / 10.0
            );
        }

        eprintln!("\nDone!");
        eprintln!(
            "Process time = {:.1} [sec]",
            time_start.elapsed().as_secs_f32()
        );
        Ok(())
    }
}

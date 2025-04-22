use std::path::PathBuf;

use anyhow::Result;

pub use super::utils::MotionMode;
use crate::{
    channel::Channel,
    constants::*,
    datetime::{DateTime, GpsTime},
    eph::Ephemeris,
    geometry::Ecef,
    io::{DataFormat, IQWriter},
    ionoutc::IonoUtc,
    propagation::compute_range,
    table::ANT_PAT_DB,
};
#[derive(Debug)]
pub struct SignalGenerator {
    pub ephemerides: Box<[[Ephemeris; MAX_SAT]; EPHEM_ARRAY_SIZE]>,
    pub valid_ephemerides_index: usize,
    pub channels: [Channel; MAX_CHAN],
    pub ionoutc: IonoUtc,
    pub allocated_satellite: [i32; MAX_SAT],
    /// posisions of receiver, per 100ms
    pub positions: Vec<Ecef>,
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
    // pub iq_buffer: Vec<i16>,
    pub out_file: Option<PathBuf>,
    pub writer: Option<IQWriter>,
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
            // iq_buffer: Vec::new(),
            out_file: None,
            writer: None,
            initialized: false,
            verbose: true,
        }
    }
}
impl SignalGenerator {
    pub fn initiallize(&mut self) -> Result<()> {
        // Initialize channels
        match self.mode {
            MotionMode::Static => eprintln!("Using static location mode."),
            MotionMode::Dynamic => eprintln!("Using dynamic location mode."),
        }

        eprintln!(
            "xyz = {}, {}, {}",
            self.positions[0].x, self.positions[0].y, self.positions[0].z,
        );
        let gps_time_start = self.receiver_gps_time.clone();
        let date_time_start = DateTime::from(&gps_time_start);
        eprintln!(
            "Start time = {:4}/{:02}/{:02},{:02}:{:02}:{:0>2.0} ({}:{:.0})",
            date_time_start.y,
            date_time_start.m,
            date_time_start.d,
            date_time_start.hh,
            date_time_start.mm,
            date_time_start.sec,
            gps_time_start.week,
            gps_time_start.sec,
        );
        // Clear all channels
        self.channels
            .iter_mut()
            .take(MAX_CHAN)
            .for_each(|ch| ch.prn = 0);
        // Clear satellite allocation flag
        self.allocated_satellite
            .iter_mut()
            .take(MAX_SAT)
            .for_each(|s| *s = -1);
        // Initial reception time
        self.receiver_gps_time = self.receiver_gps_time.add_secs(0.0);
        // Allocate visible satellites
        self.allocate_channel(self.positions[0]);

        for ichan in self.channels.iter().take(MAX_CHAN) {
            if ichan.prn != 0 {
                eprintln!(
                    "{:02} {:6.1} {:5.1} {:11.1} {:5.1}",
                    ichan.prn,
                    ichan.azel.az * R2D,
                    ichan.azel.el * R2D,
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
        // self.iq_buffer = vec![0; 2 * self.iq_buffer_size];
        let writer = IQWriter::new(
            self.out_file.as_ref().unwrap(),
            self.data_format,
            self.iq_buffer_size,
        )?;
        self.writer = Some(writer);
        self.initialized = true;
        Ok(())
    }

    pub fn allocate_channel(&mut self, xyz: Ecef) -> i32 {
        let mut nsat: i32 = 0;
        // let ref_0: [f64; 3] = [0., 0., 0.];
        // #[allow(unused_variables)]
        // let mut r_ref: f64 = 0.;
        // #[allow(unused_variables)]
        // let mut r_xyz: f64;
        for (sv, eph) in self.ephemerides[self.valid_ephemerides_index]
            .iter()
            .enumerate()
            .take(MAX_SAT)
        {
            if let Some((azel, true)) =
                eph.check_visibility(&self.receiver_gps_time, &xyz, 0.0)
            {
                nsat += 1; // Number of visible satellites
                if self.allocated_satellite[sv] == -1 {
                    // Visible but not allocated
                    //
                    // Allocated new satellite
                    let mut channel_index = 0;
                    for (i, ichan) in
                        self.channels.iter_mut().take(MAX_CHAN).enumerate()
                    {
                        if ichan.prn == 0 {
                            // Initialize channel
                            ichan.prn = sv + 1;
                            ichan.azel = azel;
                            // C/A code generation
                            ichan.codegen();
                            // Generate subframe
                            eph.generate_navigation_subframes(
                                &self.ionoutc,
                                &mut ichan.sbf,
                            );
                            // Generate navigation message
                            ichan.generate_nav_msg(
                                &self.receiver_gps_time,
                                true,
                            );
                            // Initialize pseudorange
                            let rho = compute_range(
                                eph,
                                &self.ionoutc,
                                &self.receiver_gps_time,
                                &xyz,
                            );
                            ichan.rho0 = rho;
                            // Initialize carrier phase
                            // r_xyz = rho.range;
                            // below line does nothing
                            // let _rho =
                            //     compute_range(&eph[sv], ionoutc, grx,
                            // &ref_0); r_ref = rho.
                            // range;
                            let mut phase_ini: f64 = 0.0; // TODO: Must initialize properly
                            //phase_ini = (2.0*r_ref - r_xyz)/LAMBDA_L1;
                            // #ifdef FLOAT_CARR_PHASE
                            //                         ichan.carr_phase =
                            // phase_ini - floor(phase_ini);
                            // #else
                            phase_ini -= phase_ini.floor();
                            ichan.carr_phase =
                                (512.0 * 65536.0 * phase_ini) as u32;
                            break;
                        }
                        channel_index = i + 1;
                    }
                    // Set satellite allocation channel
                    if channel_index < MAX_CHAN {
                        self.allocated_satellite[sv] = channel_index as i32;
                    }
                }
            } else if self.allocated_satellite[sv] >= 0 {
                // Not visible but allocated
                // Clear channel
                self.channels[self.allocated_satellite[sv] as usize].prn = 0;
                // Clear satellite allocation flag
                self.allocated_satellite[sv] = -1;
            }
        }
        nsat
    }

    /// Generate baseband signals
    /// # Errors
    /// Returns `anyhow::Error`
    /// TODO: unroll `for i in 0..MAX_CHAN`, to make it faster around 12%
    #[allow(clippy::too_many_lines)]
    pub fn generate(&mut self) -> Result<()> {
        if !self.initialized {
            anyhow::bail!("Not initialized!");
        }
        // let mut writer = self.writer.as_mut().unwrap();
        // let file = File::create(self.out_file.as_ref().unwrap())?;
        // let mut file = BufWriter::new(file);
        // Generate baseband signals
        // const INTERVAL: f64 = 0.1;
        self.receiver_gps_time =
            self.receiver_gps_time.add_secs(self.sample_rate);
        let mut ieph = self.valid_ephemerides_index;

        // let iq_buff_size = self.iq_buffer_size;
        let sampling_period = self.sample_frequency.recip();

        let time_start = std::time::Instant::now();
        // 主循环：遍历每个时间间隔（0.1秒）
        for user_motion_index in 1..self.user_motion_count {
            // 根据静态/动态模式选择接收机位置
            let current_location = match self.mode {
                MotionMode::Static => self.positions[0],
                MotionMode::Dynamic => self.positions[user_motion_index],
            };
            // 第一步：更新所有通道的伪距、相位和增益参数
            for i in 0..MAX_CHAN {
                println!("{}", self.channels[i].codeCA);
                // 仅处理已分配卫星的通道
                if self.channels[i].prn != 0 {
                    // 卫星PRN号转索引
                    let sv = self.channels[i].prn - 1;
                    // 计算当前时刻的伪距（传播时延）
                    // Refresh code phase and data bit counters

                    // Current pseudorange
                    let rho = compute_range(
                        &self.ephemerides[ieph][sv],
                        &self.ionoutc,
                        &self.receiver_gps_time,
                        &current_location,
                    );

                    // 更新方位角/仰角信息
                    // Update code phase and data bit counters
                    self.channels[i].azel = rho.azel;
                    // 计算码相位（C/A码偏移）
                    self.channels[i].compute_code_phase(&rho, self.sample_rate);
                    self.channels[i].carr_phasestep = (512.0
                        * 65536.0
                        * self.channels[i].f_carr
                        * sampling_period)
                        .round()
                        as i32;

                    // Path loss
                    let path_loss = 20_200_000.0 / rho.distance;
                    // Receiver antenna gain
                    let ibs = ((90.0 - rho.azel.el * R2D) / 5.0) as usize; // covert elevation to boresight
                    let ant_gain = self.antenna_pattern[ibs];
                    // 计算信号增益（考虑路径损耗和天线方向图）
                    // Signal gain
                    // 应用增益模式选择
                    if let Some(fixed_gain) = self.fixed_gain {
                        // 固定增益模式
                        self.antenna_gains[i] = fixed_gain; // hold the power level constant
                    } else {
                        // 带路径损耗补偿
                        self.antenna_gains[i] =
                            (path_loss * ant_gain * 128.0) as i32; // scaled by 2^7
                    }
                }
            }
            // 第二步：生成基带I/Q采样数据
            if let Some(w) = self.writer.as_mut() {
                for isamp in 0..w.buffer_size {
                    let mut i_acc: i32 = 0;
                    let mut q_acc: i32 = 0;
                    // 第三步：累加所有通道的信号分量
                    // let (i_acc, q_acc) = self
                    //     .channels
                    //     .iter_mut()
                    //     .zip(self.antenna_gains.iter())
                    //     .filter(|(ch, _)| ch.prn != 0)
                    //     .fold((0, 0), |(i_acc, q_acc), (ch, gain)| {
                    //         let (ip, qp) =
                    // ch.generate_iq_contribution(*gain);
                    //         // Update code phase
                    //         // 第四步：更新码相位（C/A码序列控制）
                    //         ch.update_navigation_bits(sampling_period);
                    //
                    //         // Accumulate for all visible satellites
                    //         // 累加到总信号
                    //         (i_acc + ip, q_acc + qp)
                    //     });
                    for i in 0..MAX_CHAN {
                        if self.channels[i].prn != 0 {
                            let (ip, qp) = self.channels[i]
                                .generate_iq_contribution(
                                    self.antenna_gains[i],
                                );
                            // Accumulate for all visible satellites
                            // 累加到总信号
                            i_acc += ip;
                            q_acc += qp;
                            // Update code phase
                            // 第四步：更新码相位（C/A码序列控制）
                            self.channels[i]
                                .update_navigation_bits(sampling_period);
                        }
                    }

                    // 第六步：量化并存储I/Q采样
                    // Scaled by 2^7
                    // i_acc = (i_acc + 64) >> 7;
                    // q_acc = (q_acc + 64) >> 7;
                    // Store I/Q samples into buffer
                    w.buffer[isamp * 2] = ((i_acc + 64) >> 7) as i16; // 8位量化（带舍入）
                    w.buffer[isamp * 2 + 1] = ((q_acc + 64) >> 7) as i16;
                }

                // 第七步：将I/Q数据写入输出文件（不同格式处理）
                w.write_samples()?;
            }

            // Update navigation message and channel allocation every 30 seconds
            //
            // 第八步：定期更新导航信息（每30秒）
            let igrx = (self.receiver_gps_time.sec * 10.0 + 0.5) as i32;
            if igrx % 300 == 0 {
                // Every 30 seconds
                for ichan in self.channels.iter_mut().take(MAX_CHAN) {
                    if ichan.prn != 0 {
                        ichan.generate_nav_msg(&self.receiver_gps_time, false);
                    }
                }
                // Refresh ephemeris and subframes
                // Quick and dirty fix. Need more elegant way.
                for sv in 0..MAX_SAT {
                    if self.ephemerides[ieph + 1][sv].vflg {
                        let dt = self.ephemerides[ieph + 1][sv]
                            .toc
                            .diff_secs(&self.receiver_gps_time);
                        if dt < SECONDS_IN_HOUR {
                            // move next set of ephemeris
                            ieph += 1;
                            self.valid_ephemerides_index = ieph;
                            for ichan in self.channels.iter_mut().take(MAX_CHAN)
                            {
                                // Generate new subframes if allocated
                                if ichan.prn != 0 {
                                    self.ephemerides[ieph][ichan.prn - 1]
                                        .generate_navigation_subframes(
                                            &self.ionoutc,
                                            &mut ichan.sbf,
                                        );
                                }
                            }
                        }
                        break;
                    }
                }
                // Update channel allocation
                self.allocate_channel(current_location);

                // Show details about simulated channels
                if self.verbose {
                    eprintln!();
                    for ichan in self.channels.iter().take(MAX_CHAN) {
                        if ichan.prn != 0 {
                            eprintln!(
                                "{:02} {:6.1} {:5.1} {:11.1} {:5.1}",
                                ichan.prn,
                                ichan.azel.az * R2D,
                                ichan.azel.el * R2D,
                                ichan.rho0.distance,
                                ichan.rho0.iono_delay,
                            );
                        }
                    }
                }
            }
            // 第九步：更新时间并显示进度
            // Update receiver time
            self.receiver_gps_time =
                self.receiver_gps_time.add_secs(self.sample_rate);
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

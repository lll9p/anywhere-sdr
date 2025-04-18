#![allow(dead_code, unused_imports)]
use std::{io::Write, path::PathBuf};

use anyhow::{Context, Error, Result, bail};

use crate::{
    channel::Channel,
    constants::*,
    datetime::{DateTime, GpsTime},
    eph::Ephemeris,
    ionoutc::IonoUtc,
    llh2xyz,
    read_nmea_gga::read_nmea_gga_any_size,
    read_rinex::read_rinex_nav_all,
    read_user_motion::{
        read_user_motion_any_size, read_user_motion_llh_any_size,
    },
    table::{ANT_PAT_DB, COS_TABLE512, SIN_TABLE512},
    utils::{allocate_channel, compute_range, eph2sbf},
};
#[derive(Debug)]
pub enum MotionMode {
    Static,
    Dynamic,
    // UserControl
}
#[derive(Debug)]
pub enum DataFormat {
    Bits1 = 1,
    Bits8 = 8,
    Bits16 = 16,
}
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
    initialized: bool,
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
    pub fn generate(&mut self) {
        let mut file =
            std::fs::File::create(self.out_file.as_ref().unwrap()).unwrap();
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
        let delt = self.sample_frequency.recip();

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
                    let sv = (channels[i].prn - 1) as usize;
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
                    channels[i].carr_phasestep =
                        (512.0 * 65536.0 * channels[i].f_carr * delt).round()
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
                        channels[i].code_phase += channels[i].f_code * delt;
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
                        // * delt;
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
                        ))
                        .ok();
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
                        ))
                        .ok();
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
                    file.write_all(byte_slice).ok();
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
                                if ichan.prn != 0_i32 {
                                    eph2sbf(
                                        &ephemerides[*ieph]
                                            [(ichan.prn - 1) as usize],
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
    }
}

#[allow(clippy::type_complexity)]
#[derive(Debug, Default)]
pub struct SignalGeneratorBuilder {
    output_file: Option<PathBuf>,
    ephemerides_data: Option<(
        usize,
        IonoUtc,
        Box<[[Ephemeris; MAX_SAT]; EPHEM_ARRAY_SIZE]>,
    )>,
    leap: Option<Vec<i32>>,
    positions: Option<Vec<[f64; 3]>>,
    sample_rate: Option<f64>,
    mode: Option<MotionMode>,
    duration: Option<f64>,
    frequency: Option<f64>,
    time_override: Option<bool>,
    receiver_gps_time: Option<GpsTime>,
    data_format: Option<DataFormat>,
    path_loss: Option<i32>,
    ionospheric_disable: Option<bool>,
    verbose: Option<bool>,
}
impl SignalGeneratorBuilder {
    fn parse_datetime(
        value: &str,
    ) -> Result<jiff::civil::DateTime, jiff::Error> {
        let time: jiff::civil::DateTime = value.parse()?;
        Ok(time)
    }

    pub fn navigation_file(
        mut self, navigation_file: Option<PathBuf>,
    ) -> Result<Self, Error> {
        use std::array;
        // Read ephemeris
        if let Some(file) = navigation_file {
            let mut ephemerides: Box<[[Ephemeris; MAX_SAT]; EPHEM_ARRAY_SIZE]> =
                std::array::from_fn(|_| {
                    std::array::from_fn(|_| Ephemeris::default())
                })
                .into();
            let mut iono_utc = IonoUtc::default();
            let count =
                read_rinex_nav_all(&mut ephemerides, &mut iono_utc, &file)
                    .map_err(|_| {
                        Error::msg("ERROR: ephemeris file not found or error.")
                    })?;
            if count == 0 {
                bail!("");
            }
            self.ephemerides_data = Some((count, iono_utc, ephemerides));
        }
        Ok(self)
    }

    pub fn time_override(mut self, time_override: Option<bool>) -> Self {
        self.time_override = time_override;
        self
    }

    pub fn time(mut self, time: Option<&String>) -> Result<Self> {
        if let Some(time) = time {
            let time_parsed = match time.to_lowercase().as_str() {
                "now" => jiff::Timestamp::now().in_tz("UTC"),
                time => Self::parse_datetime(time)?.in_tz("UTC"),
            }?;
            let time = DateTime {
                y: i32::from(time_parsed.year()),
                m: i32::from(time_parsed.month()),
                d: i32::from(time_parsed.day()),
                hh: i32::from(time_parsed.hour()),
                mm: i32::from(time_parsed.minute()),
                sec: f64::from(time_parsed.second()), // TODO: add floor?
            };
            self.receiver_gps_time = Some(GpsTime::from(&time));
        }
        Ok(self)
    }

    pub fn duration(mut self, duration: Option<f64>) -> Self {
        self.duration = duration;
        self
    }

    pub fn ionospheric_disable(mut self, disable: Option<bool>) -> Self {
        self.ionospheric_disable = disable;
        self
    }

    pub fn leap(mut self, leap: Option<Vec<i32>>) -> Self {
        self.leap = leap;
        self
    }

    pub fn data_format(mut self, data_format: Option<usize>) -> Result<Self> {
        match data_format {
            Some(1) => self.data_format = Some(DataFormat::Bits1),
            Some(8) => self.data_format = Some(DataFormat::Bits8),
            Some(16) => self.data_format = Some(DataFormat::Bits16),
            None => {}
            _ => {
                bail!("ERROR: Invalid I/Q data format.")
            }
        }
        Ok(self)
    }

    pub fn output_file(mut self, file: Option<PathBuf>) -> Self {
        self.output_file = file;
        self
    }

    pub fn frequency(mut self, frequency: Option<usize>) -> Result<Self> {
        match frequency {
            Some(freq) if freq < 1_000_000 => {
                self.frequency = Some(freq as f64);
            }
            None => {}
            _ => bail!("ERROR: Invalid sampling frequency."),
        }
        Ok(self)
    }

    pub fn location_ecef(mut self, location: Option<Vec<f64>>) -> Result<Self> {
        if self.positions.is_some() && location.is_some() {
            bail!("Cannot set position(s) more than once");
        }
        if let Some(location) = location {
            self.mode = Some(MotionMode::Static);
            let location = [location[0], location[1], location[2]];
            self.positions = Some(vec![location]);
        }
        Ok(self)
    }

    pub fn location(mut self, location: Option<Vec<f64>>) -> Result<Self> {
        if self.positions.is_some() && location.is_some() {
            bail!("Cannot set position(s) more than once");
        }
        if let Some(location) = location {
            self.mode = Some(MotionMode::Static);
            let mut location = [location[0], location[1], location[2]];
            location[0] /= R2D;
            location[1] /= R2D;
            let mut xyz = [0.0, 0.0, 0.0];
            llh2xyz(&location, &mut xyz);
            self.positions = Some(vec![xyz]);
        }
        Ok(self)
    }

    pub fn verbose(mut self, verbose: Option<bool>) -> Self {
        self.verbose = verbose;
        self
    }

    pub fn path_loss(mut self, loss: Option<i32>) -> Self {
        self.path_loss = loss;
        self
    }

    pub fn user_mothon_file(mut self, file: Option<PathBuf>) -> Result<Self> {
        if self.positions.is_some() && file.is_some() {
            bail!("Cannot set position(s) more than once");
        }
        if let Some(file) = file {
            self.mode = Some(MotionMode::Dynamic);
            self.positions = Some(read_user_motion_any_size(&file)?);
        }
        Ok(self)
    }

    pub fn user_mothon_llh_file(
        mut self, file: Option<PathBuf>,
    ) -> Result<Self> {
        if self.positions.is_some() && file.is_some() {
            bail!("Cannot set position(s) more than once");
        }
        if let Some(file) = file {
            self.mode = Some(MotionMode::Dynamic);
            self.positions = Some(read_user_motion_llh_any_size(&file)?);
        }
        Ok(self)
    }

    pub fn user_mothon_nmea_gga_file(
        mut self, file: Option<PathBuf>,
    ) -> Result<Self> {
        if self.positions.is_some() && file.is_some() {
            bail!("Cannot set position(s) more than once");
        }
        if let Some(file) = file {
            self.mode = Some(MotionMode::Dynamic);
            self.positions = Some(read_nmea_gga_any_size(&file)?);
        }
        Ok(self)
    }

    pub fn sample_rate(mut self, rate: Option<f64>) -> Self {
        self.sample_rate = rate;
        self
    }

    #[allow(unused_mut, unused_variables, clippy::too_many_lines)]
    pub fn build(mut self) -> Result<SignalGenerator> {
        // ensure navigation data is read
        let Some((mut count, mut ionoutc, mut ephemerides)) =
            self.ephemerides_data
        else {
            bail!("You must set navigation!");
        };
        // check and set defaults
        // leap setting
        if let Some(leap) = self.leap {
            ionoutc.leapen = 1;
            ionoutc.wnlsf = leap[0];
            ionoutc.dn = leap[1];
            ionoutc.dtlsf = leap[2];
            #[allow(clippy::impossible_comparisons)]
            if ionoutc.dn < 1 && ionoutc.dn > 7 {
                bail!("ERROR: Invalid GPS day number");
            }
            if ionoutc.wnlsf < 0 {
                bail!("ERROR: Invalid GPS week number")
            }
            #[allow(clippy::impossible_comparisons)]
            if ionoutc.dtlsf < -128 && ionoutc.dtlsf > 127 {
                bail!("ERROR: Invalid delta leap second");
            }
        }
        // positions
        let positions = if let Some(positions) = self.positions {
            if positions.len() == 1 {
                self.mode = Some(MotionMode::Static);
            } else if positions.is_empty() {
                bail!("Wrong positions!");
            }
            positions
        } else {
            // Default static location; Tokyo
            self.mode = Some(MotionMode::Static);
            let llh = [35.681_298 / R2D, 139.766_247 / R2D, 10.0];
            let mut xyz = [0.0, 0.0, 0.0];
            llh2xyz(&llh, &mut xyz);
            vec![xyz]
        };
        // sample_rate, default is 0.1/10HZ
        let sample_rate = self.sample_rate.unwrap_or(0.1);
        // mode
        let mut mode = self.mode.unwrap_or(MotionMode::Static);
        // check duration
        if self.duration.is_some_and(|d| d < 0.0) {
            bail!("ERROR: Invalid duration.");
        }
        let user_motion_count = if let Some(duration) = self.duration {
            let duration_count = (duration * 10.0 + 0.5) as usize;
            if matches!(mode, MotionMode::Static) {
                // if is static mode just return it
                duration_count
            } else {
                // if not static mode need to set to min of them
                positions.len().min(duration_count)
            }
        } else {
            // not set, it is positions' len
            positions.len()
        };
        // frequency
        let sample_frequency = self.frequency.unwrap_or(2_600_000.0);
        // is override time?

        let mut antenna_gains: [i32; MAX_CHAN] = [0; MAX_CHAN];
        let mut antenna_pattern: [f64; 37] = [0.; 37];
        let mut datetime_min = DateTime::default();
        let mut datetime_max = DateTime::default();
        let mut gpstime_min = GpsTime::default();
        let mut gpstime_max = GpsTime::default();
        // get min time of ephemerides
        for sv in 0..MAX_SAT {
            if ephemerides[0][sv].vflg {
                gpstime_min = ephemerides[0][sv].toc.clone();
                break;
            }
        }
        // get max time of ephemerides
        for sv in 0..MAX_SAT {
            if ephemerides[count - 1][sv].vflg {
                gpstime_max = ephemerides[count - 1][sv].toc.clone();
                break;
            }
        }
        let time_override = self.time_override.unwrap_or(false);
        let receiver_gps_time = if let Some(gps_time_0) = self.receiver_gps_time
        {
            // Scenario start time has been set.
            if time_override {
                let mut gtmp = GpsTime {
                    week: gps_time_0.week,
                    sec: f64::from(
                        gps_time_0.sec as i32 / SECONDS_IN_HOUR as i32 * 2,
                    ) * SECONDS_IN_HOUR
                        * 2.0,
                };
                // let mut gtmp: GpsTime = GpsTime::default();
                // gtmp.week = g0.week;
                // gtmp.sec = f64::from(g0.sec as i32 / 7200) * 7200.0;
                // Overwrite the UTC reference week number
                let dsec = gtmp.diff_secs(&gpstime_min);
                ionoutc.wnt = gtmp.week;
                ionoutc.tot = gtmp.sec as i32;
                // Iono/UTC parameters may no longer valid
                //ionoutc.vflg = FALSE;
                for sv in 0..MAX_SAT {
                    for i_eph in ephemerides.iter_mut().take(count) {
                        if i_eph[sv].vflg {
                            gtmp = i_eph[sv].toc.add_secs(dsec);
                            let ttmp = DateTime::from(&gtmp);
                            i_eph[sv].toc = gtmp;
                            i_eph[sv].t = ttmp;
                            gtmp = i_eph[sv].toe.add_secs(dsec);
                            i_eph[sv].toe = gtmp;
                        }
                    }
                }
            } else if gps_time_0.diff_secs(&gpstime_min) < 0.0
                || gpstime_max.diff_secs(&gps_time_0) < 0.0f64
            {
                bail!("ERROR: Invalid start time.");
            }
            gps_time_0
        } else {
            gpstime_min
        };
        let mut valid_ephemerides_index = None;

        // Select the current set of ephemerides
        for (i, eph_item) in ephemerides.iter().enumerate().take(count) {
            for e in eph_item.iter().take(MAX_SAT) {
                if e.vflg {
                    let dt = receiver_gps_time.diff_secs(&e.toc);
                    if (-SECONDS_IN_HOUR..SECONDS_IN_HOUR).contains(&dt) {
                        valid_ephemerides_index = Some(i);
                        break;
                    }
                }
            }
            if valid_ephemerides_index.is_some() {
                // ieph has been set
                break;
            }
        }

        let Some(valid_ephemerides_index) = valid_ephemerides_index else {
            bail!("ERROR: No current set of ephemerides has been found.");
        };
        // Disable ionospheric correction
        ionoutc.enable = self.ionospheric_disable.unwrap_or(true);
        let Some(data_format) = self.data_format else {
            bail!("data format is not set");
        };

        let generator = SignalGenerator {
            ephemerides,
            valid_ephemerides_index,
            ionoutc,
            positions,
            user_motion_count,
            receiver_gps_time,
            antenna_gains,
            antenna_pattern,
            mode,
            sample_frequency,
            sample_rate,
            data_format,
            fixed_gain: self.path_loss,
            out_file: self.output_file,
            verbose: false,
            ..Default::default()
        };
        Ok(generator)
    }
}

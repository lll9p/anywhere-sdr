use std::path::PathBuf;

use geometry::{Azel, Ecef};
use gps::{
    BroadcastEphemeris, Error, GpsTime, IonoUtc, SignalGenerator,
    SignalGeneratorBuilder, compute_range,
};

#[derive(Clone, Debug)]
pub struct VisibleSatellite {
    pub prn: usize,
    pub azel: Azel,
}

pub struct FixedScenario {
    generator: SignalGenerator,
    navigation_path: PathBuf,
    start_time: GpsTime,
}

impl FixedScenario {
    pub fn new_custom(
        start_time_text: &str, duration_seconds: f64, location_llh: [f64; 3],
        sample_frequency_hz: usize, ionosphere_enabled: bool,
        fixed_gain: Option<i32>,
    ) -> Result<Self, Error> {
        let navigation_path = PathBuf::from(env!("CARGO_WORKSPACE_DIR"))
            .join("resources")
            .join("brdc0010.22n");
        let location = location_llh.to_vec();
        let builder = SignalGeneratorBuilder::default()
            .navigation_file(Some(navigation_path.clone()))?
            .location(Some(location))?
            .time(Some(start_time_text.to_owned()))?
            .duration(Some(duration_seconds))
            .frequency(Some(sample_frequency_hz))?
            .data_format(Some(8))?
            .ionospheric_disable(Some(!ionosphere_enabled))
            .path_loss(fixed_gain)
            .output_file(None)
            .verbose(Some(false));
        let mut generator = builder.build()?;
        generator.initialize()?;
        let start_time = generator.receiver_gps_time.clone();

        Ok(Self {
            generator,
            navigation_path,
            start_time,
        })
    }

    pub fn navigation_path(&self) -> &PathBuf {
        &self.navigation_path
    }

    pub fn start_time(&self) -> &GpsTime {
        &self.start_time
    }

    pub fn sample_frequency_hz(&self) -> f64 {
        self.generator.sample_frequency
    }

    pub fn sample_rate_seconds(&self) -> f64 {
        self.generator.sample_rate
    }

    pub fn receiver_position(&self) -> Ecef {
        self.generator.positions[0]
    }

    pub fn ionoutc(&self) -> &IonoUtc {
        &self.generator.ionoutc
    }

    pub fn expected_ephemeris(
        &self, prn: usize,
    ) -> Option<&BroadcastEphemeris> {
        self.generator.ephemerides[self.generator.valid_ephemerides_index]
            .get(prn.checked_sub(1)?)
    }

    pub fn visible_satellites(&self) -> Vec<VisibleSatellite> {
        let receiver_position = self.receiver_position();
        let mut satellites = Vec::new();

        for (sv_index, ephemeris) in self.generator.ephemerides
            [self.generator.valid_ephemerides_index]
            .iter()
            .enumerate()
        {
            let Some((azel, visible)) = ephemeris.check_visibility(
                &self.start_time,
                &receiver_position,
                self.generator.elevation_mask,
            ) else {
                continue;
            };
            if !visible {
                continue;
            }

            let _range = compute_range(
                ephemeris,
                &self.generator.ionoutc,
                &self.start_time,
                &receiver_position,
            );
            satellites.push(VisibleSatellite {
                prn: sv_index + 1,
                azel,
            });
        }

        satellites.sort_by(|left, right| {
            right
                .azel
                .el
                .total_cmp(&left.azel.el)
                .then_with(|| left.prn.cmp(&right.prn))
        });
        satellites
    }

    pub fn run_streaming<F, E>(&mut self, mut on_block: F) -> Result<(), E>
    where
        F: FnMut(usize, &GpsTime, &[i16]) -> Result<(), E>,
        E: From<Error>,
    {
        let stream_start = self.start_time.clone();
        let sample_frequency_hz = self.generator.sample_frequency;
        let mut block_index = 0usize;
        let mut emitted_samples = 0usize;

        self.generator.run_streaming::<_, E>(|iq| {
            let block_time = stream_start
                .add_secs(emitted_samples as f64 / sample_frequency_hz);
            on_block(block_index, &block_time, iq)?;
            emitted_samples += iq.len() / 2;
            block_index += 1;
            Ok(())
        })
    }
}

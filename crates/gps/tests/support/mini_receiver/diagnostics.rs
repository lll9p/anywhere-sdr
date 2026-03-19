use geometry::Ecef;

#[derive(Clone, Debug)]
pub struct AcquisitionMetric {
    pub prn: usize,
    pub predicted_carrier_hz: f64,
    pub predicted_code_phase_chips: f64,
    pub acquired_carrier_hz: f64,
    pub acquired_code_phase_chips: f64,
    pub peak_ratio: f64,
}

#[derive(Clone, Debug)]
pub struct WordDiagnostics {
    pub valid_word_count: usize,
}

#[derive(Clone, Debug)]
pub struct EphemerisDiagnostics {
    pub differences: Vec<&'static str>,
}

#[derive(Clone, Debug)]
pub struct PvtSolution {
    pub position_ecef: Ecef,
    pub clock_bias_m: f64,
    pub residual_rms_m: f64,
    pub used_satellites: usize,
}

impl PvtSolution {
    pub fn position_error_m(&self, truth: &Ecef) -> f64 {
        let dx = self.position_ecef.x - truth.x;
        let dy = self.position_ecef.y - truth.y;
        let dz = self.position_ecef.z - truth.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}

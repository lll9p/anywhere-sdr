use geometry::{Ecef, Neu};

/// Limits a signed delta to the configured magnitude.
pub(super) fn apply_limited_delta(delta: f64, max_delta: f64) -> f64 {
    if max_delta == 0.0 {
        delta
    } else {
        delta.clamp(-max_delta, max_delta)
    }
}

/// Computes horizontal speed magnitude from a NEU velocity vector.
pub(super) fn horizontal_speed_mps(velocity_neu: Neu) -> f64 {
    (velocity_neu.north * velocity_neu.north
        + velocity_neu.east * velocity_neu.east)
        .sqrt()
}

/// Normalizes a heading into the range `[0, 360)` degrees.
pub(super) fn normalize_heading_deg(heading_deg: f64) -> f64 {
    let degrees = heading_deg % 360.0;
    if degrees < 0.0 {
        degrees + 360.0
    } else {
        degrees
    }
}

/// Computes the signed smallest-angle heading delta in degrees.
pub(super) fn shortest_heading_delta_deg(
    current_degrees: f64, target_degrees: f64,
) -> f64 {
    let current = normalize_heading_deg(current_degrees);
    let target = normalize_heading_deg(target_degrees);
    let mut delta = target - current;
    if delta > 180.0 {
        delta -= 360.0;
    } else if delta < -180.0 {
        delta += 360.0;
    }
    delta
}

/// Converts a NEU displacement into ECEF with the tangent matrix transpose.
pub(super) fn ecef_from_neu(neu: Neu, ltcmat: [[f64; 3]; 3]) -> Ecef {
    Ecef {
        x: ltcmat[0][0] * neu.north
            + ltcmat[1][0] * neu.east
            + ltcmat[2][0] * neu.up,
        y: ltcmat[0][1] * neu.north
            + ltcmat[1][1] * neu.east
            + ltcmat[2][1] * neu.up,
        z: ltcmat[0][2] * neu.north
            + ltcmat[1][2] * neu.east
            + ltcmat[2][2] * neu.up,
    }
}

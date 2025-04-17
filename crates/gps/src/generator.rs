#![allow(dead_code, unused_imports)]
use crate::{channel::Channel, constants::*, eph::Ephemeris};
pub struct SignalGenerator {
    ephemerides: [[Ephemeris; MAX_SAT]; EPHEM_ARRAY_SIZE],
    channels: [Channel; MAX_CHAN],
    current_position: [f64; 3],
    antenna_gains: [i32; MAX_CHAN],
    antenna_pattern: [f64; 37],
}

pub struct SignalGeneratorBuilder {}

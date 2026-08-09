#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RustOutputFingerprint {
    pub length: u64,
    pub hash_a: u64,
    pub hash_b: u64,
}

pub const RELEASE_C_CASE_COUNT: usize = 26;

pub const CARRIER_PHASE_GOLDENS: &[(&str, RustOutputFingerprint)] = &[
    ("output/c_format_1bit.bin", RustOutputFingerprint {
        length: 20150000,
        hash_a: 16699627181149938887,
        hash_b: 13004944171280445264,
    }),
    ("output/c_format_8bit.bin", RustOutputFingerprint {
        length: 161200000,
        hash_a: 13376893141707610490,
        hash_b: 18213302653108518768,
    }),
    ("output/c_format_16bit.bin", RustOutputFingerprint {
        length: 322400000,
        hash_a: 1857662601395714297,
        hash_b: 13517579297592505192,
    }),
    ("output/c_freq_2mhz_1bit.bin", RustOutputFingerprint {
        length: 15500000,
        hash_a: 16111228178893168729,
        hash_b: 5960667788641430220,
    }),
    ("output/c_freq_1mhz_1bit.bin", RustOutputFingerprint {
        length: 7750000,
        hash_a: 7834021266702496295,
        hash_b: 9127116181213503469,
    }),
    ("output/c_freq_5mhz_1bit.bin", RustOutputFingerprint {
        length: 38750000,
        hash_a: 8359737025626618885,
        hash_b: 18337515348242299900,
    }),
    ("output/c_freq_2mhz_8bit.bin", RustOutputFingerprint {
        length: 124000000,
        hash_a: 10849144093354244855,
        hash_b: 8854989223209366075,
    }),
    ("output/c_freq_2mhz_16bit.bin", RustOutputFingerprint {
        length: 248000000,
        hash_a: 2986158941808919390,
        hash_b: 3885083875863370908,
    }),
    ("output/c_motion_nmea_gga.bin", RustOutputFingerprint {
        length: 20150000,
        hash_a: 13816873819418977956,
        hash_b: 8190918528410717045,
    }),
    ("output/c_motion_ecef_circle.bin", RustOutputFingerprint {
        length: 20150000,
        hash_a: 2389149853035533663,
        hash_b: 6650203124685383507,
    }),
    ("output/c_motion_llh_circle.bin", RustOutputFingerprint {
        length: 20150000,
        hash_a: 13719995127941476444,
        hash_b: 2919016144449783322,
    }),
    ("output/c_static_llh_hangzhou.bin", RustOutputFingerprint {
        length: 20150000,
        hash_a: 7386818456144869981,
        hash_b: 14918511689342801757,
    }),
    ("output/c_static_llh_tokyo.bin", RustOutputFingerprint {
        length: 20150000,
        hash_a: 714368880548141152,
        hash_b: 1858636153916600407,
    }),
    ("output/c_static_ecef_coords.bin", RustOutputFingerprint {
        length: 20150000,
        hash_a: 1546998309450255657,
        hash_b: 10514766435355109325,
    }),
    ("output/c_gain_fixed_63.bin", RustOutputFingerprint {
        length: 20150000,
        hash_a: 7053638587732309228,
        hash_b: 11608553500456091630,
    }),
    ("output/c_gain_fixed_128.bin", RustOutputFingerprint {
        length: 20150000,
        hash_a: 10212285009648814074,
        hash_b: 16651849631267634539,
    }),
    ("output/c_time_custom_start.bin", RustOutputFingerprint {
        length: 20150000,
        hash_a: 2515816439198051809,
        hash_b: 17508227426682676705,
    }),
    (
        "output/c_time_override_toc_toe.bin",
        RustOutputFingerprint {
            length: 20150000,
            hash_a: 16464720429845065507,
            hash_b: 15511078146958699818,
        },
    ),
    ("output/c_time_leap_second.bin", RustOutputFingerprint {
        length: 20150000,
        hash_a: 14214448803945130099,
        hash_b: 7760797370402593292,
    }),
    ("output/c_iono_disabled.bin", RustOutputFingerprint {
        length: 20150000,
        hash_a: 3388203205990534598,
        hash_b: 11335645964721003779,
    }),
    ("output/c_verbose_output.bin", RustOutputFingerprint {
        length: 20150000,
        hash_a: 16699627181149938887,
        hash_b: 13004944171280445264,
    }),
    ("output/c_duration_10sec.bin", RustOutputFingerprint {
        length: 6500000,
        hash_a: 12793178671506346229,
        hash_b: 6256654551756989989,
    }),
    ("output/c_duration_60sec.bin", RustOutputFingerprint {
        length: 39000000,
        hash_a: 12340324366459325978,
        hash_b: 2262915027121636291,
    }),
    (
        "output/c_combo_tokyo_2mhz_8bit.bin",
        RustOutputFingerprint {
            length: 124000000,
            hash_a: 350252044444983224,
            hash_b: 12172990329967648078,
        },
    ),
    (
        "output/c_combo_hangzhou_gain100_noiono.bin",
        RustOutputFingerprint {
            length: 20150000,
            hash_a: 18209232620164334439,
            hash_b: 8716271142787653705,
        },
    ),
    (
        "output/c_combo_ecef_3mhz_16bit.bin",
        RustOutputFingerprint {
            length: 372000000,
            hash_a: 6090387216957061591,
            hash_b: 15442539461993611647,
        },
    ),
];

pub fn expected_fingerprint(
    c_output_path: &str,
) -> Option<RustOutputFingerprint> {
    CARRIER_PHASE_GOLDENS
        .iter()
        .find_map(|(path, fingerprint)| {
            (*path == c_output_path).then_some(*fingerprint)
        })
}

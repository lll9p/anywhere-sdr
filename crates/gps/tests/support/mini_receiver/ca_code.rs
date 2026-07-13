use constants::CA_SEQ_LEN;

pub fn ca_code(prn: usize) -> Result<[i8; CA_SEQ_LEN], String> {
    const DELAYS: [usize; 32] = [
        5, 6, 7, 8, 17, 18, 139, 140, 141, 251, 252, 254, 255, 256, 257, 258,
        469, 470, 471, 472, 473, 474, 509, 512, 513, 514, 515, 516, 859, 860,
        861, 862,
    ];

    if !(1..=32).contains(&prn) {
        return Err(format!("invalid PRN {prn}"));
    }

    let mut g1 = [0i32; CA_SEQ_LEN];
    let mut g2 = [0i32; CA_SEQ_LEN];
    let mut register_1 = [-1i32; 10];
    let mut register_2 = [-1i32; 10];

    for chip_index in 0..CA_SEQ_LEN {
        g1[chip_index] = register_1[9];
        g2[chip_index] = register_2[9];

        let feedback_1 = register_1[2] * register_1[9];
        let feedback_2 = register_2[1]
            * register_2[2]
            * register_2[5]
            * register_2[7]
            * register_2[8]
            * register_2[9];

        for shift_index in (1..10).rev() {
            register_1[shift_index] = register_1[shift_index - 1];
            register_2[shift_index] = register_2[shift_index - 1];
        }

        register_1[0] = feedback_1;
        register_2[0] = feedback_2;
    }

    let mut code = [0i8; CA_SEQ_LEN];
    let sequence_start = CA_SEQ_LEN - DELAYS[prn - 1];
    for (sequence_index, (chip, g1_value)) in
        (sequence_start..).zip(code.iter_mut().zip(g1))
    {
        *chip = (-(g1_value * g2[sequence_index % CA_SEQ_LEN])) as i8;
    }

    Ok(code)
}

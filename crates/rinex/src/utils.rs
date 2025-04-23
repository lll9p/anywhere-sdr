pub fn parse_rinex_f64(
    num_string: &str,
) -> Result<f64, std::num::ParseFloatError> {
    num_string.replace('D', "E").parse()
}

pub fn parse_i32(num_string: &str) -> Result<i32, std::num::ParseIntError> {
    num_string.parse()
}

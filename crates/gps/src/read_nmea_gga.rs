use std::{fs, path::PathBuf};

use crate::{
    constants::{USER_MOTION_SIZE, *},
    utils::llh2xyz,
};
pub fn parse_f64(num_string: &str) -> Result<f64, std::num::ParseFloatError> {
    num_string.parse()
}

pub fn read_nmea_gga(
    xyz: &mut [[f64; 3]; USER_MOTION_SIZE], filename: &PathBuf,
) -> anyhow::Result<usize> {
    let mut numd: usize = 0;

    let content = fs::read_to_string(filename)?;

    let lines = content.lines();

    for (i, line) in lines.enumerate().take(USER_MOTION_SIZE) {
        let line_vec = line.split(',').collect::<Vec<&str>>();
        let _header = line_vec[0];
        let _utc = line_vec[1];
        let lat = line_vec[2];
        let lat_dir = line_vec[3];
        let lon = line_vec[4];
        let lon_dir = line_vec[5];
        let _quality = line_vec[6];
        let _nsats = line_vec[7];
        let _hdop = line_vec[8];
        let alt = line_vec[9];
        let _a_units = line_vec[10];
        let undulation = line_vec[11];
        let _u_units = line_vec[12];
        let _age_or_stn_id = line_vec[13];
        let _checksum = line_vec[14];
        let mut llh = [0.0f64; 3];
        let mut pos = [0.0f64; 3];
        llh[0] = parse_f64(&lat[..2])? + parse_f64(&lat[2..])? / 60.0;
        // println!(
        //     "{},{}",
        //     parse_f64(&lat[..2].to_string())?,
        //     parse_f64(&lat[2..].to_string())?
        // );
        if lat_dir == "S" {
            llh[0] *= -1.0;
        }
        llh[0] /= R2D;
        llh[1] = parse_f64(&lon[..3])? + parse_f64(&lon[3..])? / 60.0;
        if lon_dir == "W" {
            llh[1] *= -1.0;
        }
        llh[1] /= R2D;
        llh[2] = parse_f64(alt)? + parse_f64(undulation)?;
        llh2xyz(&llh, &mut pos);

        // println!("{llh:?}");
        xyz[i][0] = pos[0];
        xyz[i][1] = pos[1];
        xyz[i][2] = pos[2];
        numd = i;
        // if i >= USER_MOTION_SIZE - 1 {
        //     break;
        // }
    }
    Ok(numd + 1)
}

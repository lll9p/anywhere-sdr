use crate::{constants::*, process::llh2xyz};
use std::{fs, path::PathBuf};

///  \brief Read the list of user motions from the input file
///  \param[out] xyz Output array of ECEF vectors for user motion
///  \param[[in] filename File name of the text input file
///  \returns Number of user data motion records read, -1 on error
pub fn read_user_motion(
    xyz: &mut [[f64; 3]; USER_MOTION_SIZE],
    filename: &PathBuf,
) -> anyhow::Result<i32> {
    let mut umd = 0;
    let content = fs::read_to_string(filename)?;
    let lines = content.lines();
    for (i, line) in lines.enumerate().take(USER_MOTION_SIZE) {
        let line_vec = line.split(',').collect::<Vec<&str>>();
        xyz[i][0] = line_vec[1].trim().parse()?;
        xyz[i][1] = line_vec[2].trim().parse()?;
        xyz[i][2] = line_vec[3].trim().parse()?;
        umd = i as i32;
    }
    Ok(umd + 1)
}
///  \brief Read the list of user motions from the input file
///  \param[out] xyz Output array of LatLonHei coordinates for user motion
///  \param[[in] filename File name of the text input file with format Lat,Lon,Hei
///  \returns Number of user data motion records read, -1 on error
///
/// Added by romalvarezllorens@gmail.com
pub fn read_user_motion_llh(
    xyz: &mut [[f64; 3]; USER_MOTION_SIZE],
    filename: &PathBuf,
) -> anyhow::Result<i32> {
    let mut umd = 0;
    let content = fs::read_to_string(filename)?;
    let lines = content.lines();
    for (i, line) in lines.enumerate().take(USER_MOTION_SIZE) {
        let mut llh = [0.0; 3];
        let line_vec = line.split(',').collect::<Vec<&str>>();
        llh[0] = line_vec[1].trim().parse()?;
        llh[1] = line_vec[2].trim().parse()?;
        llh[2] = line_vec[3].trim().parse()?;
        if llh[0] > 90.0 || llh[0] < -90.0 || llh[1] > 180.0 || llh[1] < -180.0 {
            panic!(
                "ERROR: Invalid file format (time[s], latitude[deg], longitude[deg], height [m].\n"
            );
        }
        llh[0] /= R2D; // convert to RAD
        llh[1] /= R2D; // convert to RAD
        llh2xyz(&llh, &mut xyz[i]);
        umd = i as i32;
    }
    Ok(umd + 1)
}

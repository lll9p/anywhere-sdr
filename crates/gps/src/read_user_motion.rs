use std::{fs, path::PathBuf};

use crate::{constants::*, utils::llh2xyz};

///  \brief Read the list of user motions from the input file
///  \param[out] xyz Output array of ECEF vectors for user motion
///  \param[[in] filename File name of the text input file
///  \returns Number of user data motion records read, -1 on error
pub fn read_user_motion(
    xyz: &mut [[f64; 3]; USER_MOTION_SIZE], filename: &PathBuf,
) -> anyhow::Result<usize> {
    let mut umd = 0;
    let content = fs::read_to_string(filename)?;
    let lines = content.lines();
    for (i, line) in lines.enumerate().take(USER_MOTION_SIZE) {
        let line_vec = line.split(',').collect::<Vec<&str>>();
        xyz[i][0] = line_vec[1].trim().parse()?;
        xyz[i][1] = line_vec[2].trim().parse()?;
        xyz[i][2] = line_vec[3].trim().parse()?;
        umd = i;
    }
    Ok(umd + 1)
}
///  \brief Read the list of user motions from the input file
///  \param[out] xyz Output array of `LatLonHei` coordinates for user motion
///  \param[[in] filename File name of the text input file with format
/// Lat,Lon,Hei  \returns Number of user data motion records read, -1 on error
///
/// Added by romalvarezllorens@gmail.com
pub fn read_user_motion_llh(
    xyz: &mut [[f64; 3]; USER_MOTION_SIZE], filename: &PathBuf,
) -> anyhow::Result<usize> {
    let mut umd = 0;
    let content = fs::read_to_string(filename)?;
    let lines = content.lines();
    for (i, line) in lines.enumerate().take(USER_MOTION_SIZE) {
        let mut llh = [0.0; 3];
        let line_vec = line.split(',').collect::<Vec<&str>>();
        llh[0] = line_vec[1].trim().parse()?;
        llh[1] = line_vec[2].trim().parse()?;
        llh[2] = line_vec[3].trim().parse()?;
        assert!(
            !(llh[0] > 90.0
                || llh[0] < -90.0
                || llh[1] > 180.0
                || llh[1] < -180.0),
            "ERROR: Invalid file format (time[s], latitude[deg], \
             longitude[deg], height [m].\n"
        );
        llh[0] /= R2D; // convert to RAD
        llh[1] /= R2D; // convert to RAD
        llh2xyz(&llh, &mut xyz[i]);
        umd = i;
    }
    Ok(umd + 1)
}
///  \brief Read the list of user motions from the input file
///  \param[out] xyz Output array of ECEF vectors for user motion
///  \param[[in] filename File name of the text input file
///  \returns Number of user data motion records read, -1 on error
#[allow(dead_code)]
pub fn read_user_motion_any_size(
    filename: &PathBuf,
) -> anyhow::Result<Vec<[f64; 3]>> {
    let mut xyz = Vec::<[f64; 3]>::new();
    let content = fs::read_to_string(filename)?;
    let lines = content.lines();
    for line in lines {
        let line_vec = line.split(',').collect::<Vec<&str>>();
        xyz.push([
            line_vec[1].trim().parse()?,
            line_vec[2].trim().parse()?,
            line_vec[3].trim().parse()?,
        ]);
    }
    Ok(xyz)
}
///  \brief Read the list of user motions from the input file
///  \param[out] xyz Output array of `LatLonHei` coordinates for user motion
///  \param[[in] filename File name of the text input file with format
/// Lat,Lon,Hei  \returns Number of user data motion records read, -1 on error
///
/// Added by romalvarezllorens@gmail.com
#[allow(dead_code)]
pub fn read_user_motion_llh_any_size(
    filename: &PathBuf,
) -> anyhow::Result<Vec<[f64; 3]>> {
    let mut xyz = Vec::<[f64; 3]>::new();
    let content = fs::read_to_string(filename)?;
    let lines = content.lines();
    for line in lines {
        let line_vec = line.split(',').collect::<Vec<&str>>();
        let mut llh = [
            line_vec[1].trim().parse()?,
            line_vec[2].trim().parse()?,
            line_vec[3].trim().parse()?,
        ];
        if llh[0] > 90.0 || llh[0] < -90.0 || llh[1] > 180.0 || llh[1] < -180.0
        {
            anyhow::bail!(
                "ERROR: Invalid file format (time[s], latitude[deg], \
                 longitude[deg], height [m].\n"
            );
        }
        llh[0] /= R2D; // convert to RAD
        llh[1] /= R2D; // convert to RAD
        xyz.push(llh);
        llh2xyz(&llh, &mut xyz[0]);
    }
    Ok(xyz)
}

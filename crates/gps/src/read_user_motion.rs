use std::{fs, path::PathBuf};

use crate::{
    constants::*,
    geometry::{Ecef, Location},
};
///  \brief Read the list of user motions from the input file
///  \param[out] xyz Output array of ECEF vectors for user motion
///  \param[[in] filename File name of the text input file
///  \returns Number of user data motion records read, -1 on error
pub fn read_user_motion(filename: &PathBuf) -> anyhow::Result<Vec<Ecef>> {
    let mut xyz = Vec::new();
    let content = fs::read_to_string(filename)?;
    let lines = content.lines();
    for line in lines {
        let line_vec = line.split(',').collect::<Vec<&str>>();
        xyz.push(Ecef::from(&[
            line_vec[1].trim().parse()?,
            line_vec[2].trim().parse()?,
            line_vec[3].trim().parse()?,
        ]));
    }
    Ok(xyz)
}
///  \brief Read the list of user motions from the input file
///  \param[out] xyz Output array of `LatLonHei` coordinates for user motion
///  \param[[in] filename File name of the text input file with format
/// Lat,Lon,Hei  \returns Number of user data motion records read, -1 on error
///
/// Added by romalvarezllorens@gmail.com
pub fn read_user_motion_llh(filename: &PathBuf) -> anyhow::Result<Vec<Ecef>> {
    let mut xyz = Vec::new();
    let content = fs::read_to_string(filename)?;
    let lines = content.lines();
    for line in lines {
        let line_vec = line.split(',').collect::<Vec<&str>>();
        let mut llh = Location::from(&[
            line_vec[1].trim().parse()?,
            line_vec[2].trim().parse()?,
            line_vec[3].trim().parse()?,
        ]);
        if llh.latitude > 90.0
            || llh.latitude < -90.0
            || llh.longitude > 180.0
            || llh.longitude < -180.0
        {
            anyhow::bail!(
                "ERROR: Invalid file format (time[s], latitude[deg], \
                 longitude[deg], height [m].\n"
            );
        }
        llh.latitude /= R2D; // convert to RAD
        llh.longitude /= R2D; // convert to RAD
        // let mut xyz_item = [0.0; 3];
        // llh2xyz(&llh, &mut xyz_item);
        xyz.push(Ecef::from(&llh));
    }
    Ok(xyz)
}

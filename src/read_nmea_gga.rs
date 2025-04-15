use crate::{
    FILE, atof,
    constants::{USER_MOTION_SIZE, *},
    fclose, fgets, fopen,
    process::llh2xyz,
    strncmp, strncpy, strtok,
};
use std::{fs, path::PathBuf};

pub fn readNmeaGGA(xyz: &mut [[f64; 3]; USER_MOTION_SIZE], filename: *const libc::c_char) -> i32 {
    unsafe {
        let mut numd: i32 = 0_i32;
        let mut str: [libc::c_char; 100] = [0; 100];
        let mut llh: [f64; 3] = [0.; 3];
        let mut pos: [f64; 3] = [0.; 3];
        let mut tmp: [libc::c_char; 8] = [0; 8];
        let fp: *mut FILE = fopen(filename, b"rt\0" as *const u8 as *const libc::c_char);
        if fp.is_null() {
            return -1_i32;
        }
        while !(fgets(str.as_mut_ptr(), 100_i32, fp)).is_null() {
            let token = strtok(str.as_mut_ptr(), b",\0" as *const u8 as *const libc::c_char);
            if strncmp(
                token.offset(3),
                b"GGA\0" as *const u8 as *const libc::c_char,
                3_i32 as u32,
            ) != 0_i32
            {
                continue;
            }
            let _token = strtok(
                std::ptr::null_mut::<libc::c_char>(),
                b",\0" as *const u8 as *const libc::c_char,
            );
            let token = strtok(
                std::ptr::null_mut::<libc::c_char>(),
                b",\0" as *const u8 as *const libc::c_char,
            );
            strncpy(tmp.as_mut_ptr(), token, 2_i32 as u32);
            tmp[2] = 0_i32 as libc::c_char;
            println!("{},{}", atof(tmp.as_mut_ptr()), atof(token.offset(2)));
            llh[0] = atof(tmp.as_mut_ptr()) + atof(token.offset(2)) / 60.0f64;

            let token = strtok(
                std::ptr::null_mut::<libc::c_char>(),
                b",\0" as *const u8 as *const libc::c_char,
            );
            if *token.offset(0) as i32 == 'S' as i32 {
                llh[0] *= -1.0f64;
            }
            llh[0] /= 57.2957795131f64;
            let token = strtok(
                std::ptr::null_mut::<libc::c_char>(),
                b",\0" as *const u8 as *const libc::c_char,
            );
            strncpy(tmp.as_mut_ptr(), token, 3_i32 as u32);
            tmp[3] = 0_i32 as libc::c_char;
            llh[1] = atof(tmp.as_mut_ptr()) + atof(token.offset(3)) / 60.0f64;
            let token = strtok(
                std::ptr::null_mut::<libc::c_char>(),
                b",\0" as *const u8 as *const libc::c_char,
            );
            if *token.offset(0) as i32 == 'W' as i32 {
                llh[1] *= -1.0f64;
            }
            llh[1] /= 57.2957795131f64;
            let _token = strtok(
                std::ptr::null_mut::<libc::c_char>(),
                b",\0" as *const u8 as *const libc::c_char,
            );
            let _token = strtok(
                std::ptr::null_mut::<libc::c_char>(),
                b",\0" as *const u8 as *const libc::c_char,
            );
            let _token = strtok(
                std::ptr::null_mut::<libc::c_char>(),
                b",\0" as *const u8 as *const libc::c_char,
            );
            let token = strtok(
                std::ptr::null_mut::<libc::c_char>(),
                b",\0" as *const u8 as *const libc::c_char,
            );
            llh[2] = atof(token);
            let _token = strtok(
                std::ptr::null_mut::<libc::c_char>(),
                b",\0" as *const u8 as *const libc::c_char,
            );
            let token = strtok(
                std::ptr::null_mut::<libc::c_char>(),
                b",\0" as *const u8 as *const libc::c_char,
            );
            llh[2] += atof(token);

            llh2xyz(&llh, &mut pos);
            println!("{llh:?}");
            (xyz[numd as usize])[0] = pos[0];
            (xyz[numd as usize])[1] = pos[1];
            (xyz[numd as usize])[2] = pos[2];
            numd += 1;
            if numd >= USER_MOTION_SIZE as i32 {
                break;
            }
        }
        fclose(fp);
        numd
    }
}

pub fn parse_f64(num_string: &str) -> Result<f64, std::num::ParseFloatError> {
    num_string.parse()
}

pub fn parse_i32(num_string: &str) -> Result<i32, std::num::ParseIntError> {
    num_string.parse()
}
pub fn read_Nmea_GGA(
    xyz: &mut [[f64; 3]; USER_MOTION_SIZE],
    filename: &PathBuf,
) -> anyhow::Result<i32> {
    let mut numd: i32 = 0;

    let content = fs::read_to_string(filename).unwrap();

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
        let _age_or_stn_Id = line_vec[13];
        let _checksum = line_vec[14];
        let mut llh = [0.0f64; 3];
        let mut pos = [0.0f64; 3];
        llh[0] = parse_f64(&lat[..2].to_string())? + parse_f64(&lat[2..].to_string())? / 60.0;
        // println!(
        //     "{},{}",
        //     parse_f64(&lat[..2].to_string())?,
        //     parse_f64(&lat[2..].to_string())?
        // );
        if lat_dir == "S" {
            llh[0] *= -1.0;
        }
        llh[0] /= R2D;
        llh[1] = parse_f64(&lon[..3].to_string())? + parse_f64(&lon[3..].to_string())? / 60.0;
        if lon_dir == "W" {
            llh[1] *= -1.0;
        }
        llh[1] /= R2D;
        llh[2] = parse_f64(&alt.to_string())? + parse_f64(&undulation.to_string())?;
        llh2xyz(&llh, &mut pos);

        // println!("{llh:?}");
        xyz[i][0] = pos[0];
        xyz[i][1] = pos[1];
        xyz[i][2] = pos[2];
        numd = i as i32;
        // if i >= USER_MOTION_SIZE - 1 {
        //     break;
        // }
    }
    Ok(numd + 1)
}

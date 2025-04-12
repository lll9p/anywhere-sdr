use crate::{FILE, constants::USER_MOTION_SIZE, fclose, fgets, fopen, llh2xyz, sscanf};
pub unsafe fn readUserMotion(
    xyz_full: &mut [[f64; 3]; USER_MOTION_SIZE],
    filename: *const libc::c_char,
) -> i32 {
    unsafe {
        
        let mut str: [libc::c_char; 100] = [0; 100];
        let mut t: f64 = 0.;
        let mut x: f64 = 0.;
        let mut y: f64 = 0.;
        let mut z: f64 = 0.;
        let fp: *mut FILE = fopen(filename, b"rt\0" as *const u8 as *const libc::c_char);
        if fp.is_null() {
            return -1_i32;
        }
        let mut numd = 0i32;
        while numd < USER_MOTION_SIZE as i32 {
            if (fgets(str.as_mut_ptr(), 100_i32, fp)).is_null() {
                break;
            }
            if -1_i32
                == sscanf(
                    str.as_mut_ptr(),
                    b"%lf,%lf,%lf,%lf\0" as *const u8 as *const libc::c_char,
                    &mut t as *mut f64,
                    &mut x as *mut f64,
                    &mut y as *mut f64,
                    &mut z as *mut f64,
                )
            {
                break;
            }
            (xyz_full[numd as usize])[0] = x;
            (xyz_full[numd as usize])[1] = y;
            (xyz_full[numd as usize])[2] = z;
            numd += 1;
        }
        fclose(fp);
        numd
    }
}
pub unsafe fn readUserMotionLLH(
    xyz_full: &mut [[f64; 3]; USER_MOTION_SIZE],
    filename: *const libc::c_char,
) -> i32 {
    unsafe {
        
        let mut t: f64 = 0.;
        let mut llh: [f64; 3] = [0.; 3];
        let mut str: [libc::c_char; 100] = [0; 100];
        let fp: *mut FILE = fopen(filename, b"rt\0" as *const u8 as *const libc::c_char);
        if fp.is_null() {
            return -1_i32;
        }
        let mut numd = 0_i32;
        while numd < USER_MOTION_SIZE as i32 {
            if (fgets(str.as_mut_ptr(), 100_i32, fp)).is_null() {
                break;
            }
            if -1_i32
                == sscanf(
                    str.as_mut_ptr(),
                    b"%lf,%lf,%lf,%lf\0" as *const u8 as *const libc::c_char,
                    &mut t as *mut f64,
                    &mut *llh.as_mut_ptr().offset(0) as *mut f64,
                    &mut *llh.as_mut_ptr().offset(1) as *mut f64,
                    &mut *llh.as_mut_ptr().offset(2) as *mut f64,
                )
            {
                break;
            }
            if llh[0] > 90.0f64 || llh[0] < -90.0f64 || llh[1] > 180.0f64 || llh[1] < -180.0f64 {
                eprintln!(
                    "ERROR: Invalid file format (time[s], latitude[deg], longitude[deg], height [m].\n"
                );
                numd = 0_i32;
                break;
            } else {
                llh[0] /= 57.2957795131f64;
                llh[1] /= 57.2957795131f64;
                llh2xyz(&llh, &mut xyz_full[numd as usize]);
                numd += 1;
            }
        }
        fclose(fp);
        numd
    }
}

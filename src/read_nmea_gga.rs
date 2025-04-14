use crate::{
    FILE, atof, constants::USER_MOTION_SIZE, fclose, fgets, fopen, process::llh2xyz, strncmp,
    strncpy, strtok,
};

pub fn readNmeaGGA(xyz_0: *mut [f64; 3], filename: *const libc::c_char) -> i32 {
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
                token.offset(3_i32 as isize),
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
            tmp[2_i32 as usize] = 0_i32 as libc::c_char;
            llh[0_i32 as usize] =
                atof(tmp.as_mut_ptr()) + atof(token.offset(2_i32 as isize)) / 60.0f64;
            let token = strtok(
                std::ptr::null_mut::<libc::c_char>(),
                b",\0" as *const u8 as *const libc::c_char,
            );
            if *token.offset(0_i32 as isize) as i32 == 'S' as i32 {
                llh[0_i32 as usize] *= -1.0f64;
            }
            llh[0_i32 as usize] /= 57.2957795131f64;
            let token = strtok(
                std::ptr::null_mut::<libc::c_char>(),
                b",\0" as *const u8 as *const libc::c_char,
            );
            strncpy(tmp.as_mut_ptr(), token, 3_i32 as u32);
            tmp[3_i32 as usize] = 0_i32 as libc::c_char;
            llh[1_i32 as usize] =
                atof(tmp.as_mut_ptr()) + atof(token.offset(3_i32 as isize)) / 60.0f64;
            let token = strtok(
                std::ptr::null_mut::<libc::c_char>(),
                b",\0" as *const u8 as *const libc::c_char,
            );
            if *token.offset(0_i32 as isize) as i32 == 'W' as i32 {
                llh[1_i32 as usize] *= -1.0f64;
            }
            llh[1_i32 as usize] /= 57.2957795131f64;
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
            llh[2_i32 as usize] = atof(token);
            let _token = strtok(
                std::ptr::null_mut::<libc::c_char>(),
                b",\0" as *const u8 as *const libc::c_char,
            );
            let token = strtok(
                std::ptr::null_mut::<libc::c_char>(),
                b",\0" as *const u8 as *const libc::c_char,
            );
            llh[2_i32 as usize] += atof(token);
            llh2xyz(&llh, &mut pos);
            (*xyz_0.offset(numd as isize))[0_i32 as usize] = pos[0_i32 as usize];
            (*xyz_0.offset(numd as isize))[1_i32 as usize] = pos[1_i32 as usize];
            (*xyz_0.offset(numd as isize))[2_i32 as usize] = pos[2_i32 as usize];
            numd += 1;
            if numd >= USER_MOTION_SIZE as i32 {
                break;
            }
        }
        fclose(fp);
        numd
    }
}

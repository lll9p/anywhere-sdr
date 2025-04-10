use crate::{
    FILE, atof, constants::USER_MOTION_SIZE, fclose, fgets, fopen, llh2xyz, strncmp, strncpy,
    strtok,
};

pub fn readNmeaGGA(mut xyz_0: *mut [f64; 3], mut filename: *const libc::c_char) -> i32 {
    unsafe {
        let mut fp: *mut FILE = std::ptr::null_mut::<FILE>();
        let mut numd: i32 = 0 as i32;
        let mut str: [libc::c_char; 100] = [0; 100];
        let mut token: *mut libc::c_char = std::ptr::null_mut::<libc::c_char>();
        let mut llh: [f64; 3] = [0.; 3];
        let mut pos: [f64; 3] = [0.; 3];
        let mut tmp: [libc::c_char; 8] = [0; 8];
        fp = fopen(filename, b"rt\0" as *const u8 as *const libc::c_char);
        if fp.is_null() {
            return -(1 as i32);
        }
        while !(fgets(str.as_mut_ptr(), 100 as i32, fp)).is_null() {
            token = strtok(str.as_mut_ptr(), b",\0" as *const u8 as *const libc::c_char);
            if strncmp(
                token.offset(3 as i32 as isize),
                b"GGA\0" as *const u8 as *const libc::c_char,
                3 as i32 as u32,
            ) != 0 as i32
            {
                continue;
            }
            token = strtok(
                std::ptr::null_mut::<libc::c_char>(),
                b",\0" as *const u8 as *const libc::c_char,
            );
            token = strtok(
                std::ptr::null_mut::<libc::c_char>(),
                b",\0" as *const u8 as *const libc::c_char,
            );
            strncpy(tmp.as_mut_ptr(), token, 2 as i32 as u32);
            tmp[2 as i32 as usize] = 0 as i32 as libc::c_char;
            llh[0 as i32 as usize] =
                atof(tmp.as_mut_ptr()) + atof(token.offset(2 as i32 as isize)) / 60.0f64;
            token = strtok(
                std::ptr::null_mut::<libc::c_char>(),
                b",\0" as *const u8 as *const libc::c_char,
            );
            if *token.offset(0 as i32 as isize) as i32 == 'S' as i32 {
                llh[0 as i32 as usize] *= -1.0f64;
            }
            llh[0 as i32 as usize] /= 57.2957795131f64;
            token = strtok(
                std::ptr::null_mut::<libc::c_char>(),
                b",\0" as *const u8 as *const libc::c_char,
            );
            strncpy(tmp.as_mut_ptr(), token, 3 as i32 as u32);
            tmp[3 as i32 as usize] = 0 as i32 as libc::c_char;
            llh[1 as i32 as usize] =
                atof(tmp.as_mut_ptr()) + atof(token.offset(3 as i32 as isize)) / 60.0f64;
            token = strtok(
                std::ptr::null_mut::<libc::c_char>(),
                b",\0" as *const u8 as *const libc::c_char,
            );
            if *token.offset(0 as i32 as isize) as i32 == 'W' as i32 {
                llh[1 as i32 as usize] *= -1.0f64;
            }
            llh[1 as i32 as usize] /= 57.2957795131f64;
            token = strtok(
                std::ptr::null_mut::<libc::c_char>(),
                b",\0" as *const u8 as *const libc::c_char,
            );
            token = strtok(
                std::ptr::null_mut::<libc::c_char>(),
                b",\0" as *const u8 as *const libc::c_char,
            );
            token = strtok(
                std::ptr::null_mut::<libc::c_char>(),
                b",\0" as *const u8 as *const libc::c_char,
            );
            token = strtok(
                std::ptr::null_mut::<libc::c_char>(),
                b",\0" as *const u8 as *const libc::c_char,
            );
            llh[2 as i32 as usize] = atof(token);
            token = strtok(
                std::ptr::null_mut::<libc::c_char>(),
                b",\0" as *const u8 as *const libc::c_char,
            );
            token = strtok(
                std::ptr::null_mut::<libc::c_char>(),
                b",\0" as *const u8 as *const libc::c_char,
            );
            llh[2 as i32 as usize] += atof(token);
            llh2xyz(llh.as_mut_ptr(), pos.as_mut_ptr());
            (*xyz_0.offset(numd as isize))[0 as i32 as usize] = pos[0 as i32 as usize];
            (*xyz_0.offset(numd as isize))[1 as i32 as usize] = pos[1 as i32 as usize];
            (*xyz_0.offset(numd as isize))[2 as i32 as usize] = pos[2 as i32 as usize];
            numd += 1;
            if numd >= USER_MOTION_SIZE as i32 {
                break;
            }
        }
        fclose(fp);
        numd
    }
}

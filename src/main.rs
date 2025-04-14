#![allow(
    dead_code,
    mutable_transmutes,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    clippy::missing_safety_doc
)]
#![feature(extern_types)]
unsafe extern "C" {
    pub type _IO_wide_data;
    pub type _IO_codecvt;
    pub type _IO_marker;

    fn fclose(__stream: *mut FILE) -> i32;
    fn fopen(_: *const libc::c_char, _: *const libc::c_char) -> *mut FILE;

    fn sscanf(_: *const libc::c_char, _: *const libc::c_char, _: ...) -> i32;
    fn fgets(__s: *mut libc::c_char, __n: i32, __stream: *mut FILE) -> *mut libc::c_char;
    fn atof(__nptr: *const libc::c_char) -> f64;
    fn atoi(__nptr: *const libc::c_char) -> i32;
    fn strcpy(_: *mut libc::c_char, _: *const libc::c_char) -> *mut libc::c_char;
    fn strncpy(_: *mut libc::c_char, _: *const libc::c_char, _: u32) -> *mut libc::c_char;
    fn strcmp(_: *const libc::c_char, _: *const libc::c_char) -> i32;
    fn strncmp(_: *const libc::c_char, _: *const libc::c_char, _: u32) -> i32;
    fn strchr(_: *const libc::c_char, _: i32) -> *mut libc::c_char;
    fn strtok(_: *mut libc::c_char, _: *const libc::c_char) -> *mut libc::c_char;
    fn time(__timer: *mut time_t) -> time_t;
    fn gmtime(__timer: *const time_t) -> *mut tm;
}

pub fn tracing_init() -> tracing_appender::non_blocking::WorkerGuard {
    let file_appender = tracing_appender::rolling::daily("./", "app.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_file(true)
        .with_line_number(true)
        .with_writer(non_blocking)
        .with_ansi(false)
        .init();
    guard
}

mod cli;
mod constants;
mod datetime;
mod eph;
mod getopt;
mod ionoutc;
mod process;
mod read_nmea_gga;
mod read_rinex;
mod read_user_motion;
mod table;
mod utils;

use clap::Parser;
use datetime::{datetime_t, gpstime_t, tm};
use eph::ephem_t;
use getopt::usage;
use ionoutc::ionoutc_t;
use process::process;
use utils::*;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct _IO_FILE {
    pub _flags: i32,
    pub _IO_read_ptr: *mut libc::c_char,
    pub _IO_read_end: *mut libc::c_char,
    pub _IO_read_base: *mut libc::c_char,
    pub _IO_write_base: *mut libc::c_char,
    pub _IO_write_ptr: *mut libc::c_char,
    pub _IO_write_end: *mut libc::c_char,
    pub _IO_buf_base: *mut libc::c_char,
    pub _IO_buf_end: *mut libc::c_char,
    pub _IO_save_base: *mut libc::c_char,
    pub _IO_backup_base: *mut libc::c_char,
    pub _IO_save_end: *mut libc::c_char,
    pub _markers: *mut _IO_marker,
    pub _chain: *mut _IO_FILE,
    pub _fileno: i32,
    pub _flags2: i32,
    pub _old_offset: libc::c_long,
    pub _cur_column: libc::c_ushort,
    pub _vtable_offset: libc::c_schar,
    pub _shortbuf: [libc::c_char; 1],
    pub _lock: *mut libc::c_void,
    pub _offset: libc::c_long,
    pub _codecvt: *mut _IO_codecvt,
    pub _wide_data: *mut _IO_wide_data,
    pub _freeres_list: *mut _IO_FILE,
    pub _freeres_buf: *mut libc::c_void,
    pub __pad5: u32,
    pub _mode: i32,
    pub _unused2: [libc::c_char; 20],
}
pub type _IO_lock_t = ();
pub type FILE = _IO_FILE;
type clock_t = libc::c_long;
type time_t = libc::c_long;

pub fn main() -> anyhow::Result<()> {
    let _guard = tracing_init();
    // let cli = Cli::parse();
    // match cli.commands {
    //     Commands::Report => {}
    //     Commands::XRD(cmds) => {
    //         cmds.run()?;
    //     }
    //     Commands::Sizer(cmds) => {
    //         cmds.run()?;
    //     }
    // }
    // let mut args: Vec<String> = std::env::args().collect();
    let mut args: Vec<*mut libc::c_char> = Vec::new();
    for arg in ::std::env::args() {
        args.push(
            (::std::ffi::CString::new(arg))
                .expect("Failed to convert argument into CString.")
                .into_raw(),
        );
    }
    args.push(::core::ptr::null_mut());

    if args.len() - 1 < 3 {
        usage();
        panic!();
    }
    unsafe { ::std::process::exit(process((args.len() - 1) as i32, args.as_mut_ptr())) };

    // Ok(())
}

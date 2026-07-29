use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const PROCESS_TIMEOUT: Duration = Duration::from_secs(20);
const MAX_DIAGNOSTIC_BYTES: usize = 128 * 1024;

fn gpssim_binary() -> &'static str {
    env!("CARGO_BIN_EXE_gpssim")
}

fn navigation_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../resources/brdc0010.22n")
}

fn common_arguments() -> Vec<OsString> {
    vec![
        OsString::from("--ephemerides"),
        navigation_path().into_os_string(),
        OsString::from("--location"),
        OsString::from("35.681298,139.766247,10.0"),
        OsString::from("--duration"),
        OsString::from("0.000001"),
        OsString::from("--frequency"),
        OsString::from("1000000"),
        OsString::from("--bits"),
        OsString::from("8"),
    ]
}

fn run_with_deadline(
    program: &Path, arguments: &[OsString], timeout: Duration,
) -> Result<Output, String> {
    let mut child = Command::new(program)
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            format!("failed to spawn {}: {error}", program.display())
        })?;
    let deadline = Instant::now() + timeout;

    loop {
        if child
            .try_wait()
            .map_err(|error| format!("failed to poll child process: {error}"))?
            .is_some()
        {
            return child.wait_with_output().map_err(|error| {
                format!("failed to collect child output: {error}")
            });
        }
        if Instant::now() >= deadline {
            child.kill().map_err(|error| {
                format!("failed to kill timed-out child: {error}")
            })?;
            let output = child.wait_with_output().map_err(|error| {
                format!("failed to reap timed-out child: {error}")
            })?;
            return Err(format!(
                "process timed out; stdout={} stderr={}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
            ));
        }
        thread::sleep(Duration::from_millis(5));
    }
}

fn run_gpssim(arguments: &[OsString]) -> Result<Output, String> {
    run_with_deadline(Path::new(gpssim_binary()), arguments, PROCESS_TIMEOUT)
}

fn diagnostics(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    )
}

fn assert_bounded(output: &Output) {
    assert!(
        output.stdout.len() + output.stderr.len() <= MAX_DIAGNOSTIC_BYTES,
        "process diagnostics exceeded {MAX_DIAGNOSTIC_BYTES} bytes"
    );
}

fn unique_output_path() -> Result<PathBuf, String> {
    let timestamp =
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| {
                format!("system clock is before the Unix epoch: {error}")
            })?;
    Ok(std::env::temp_dir().join(format!(
        "gpssim-run-outcome-{}-{}.bin",
        std::process::id(),
        timestamp.as_nanos(),
    )))
}

#[test]
fn cli_null_and_file_runs_exit_successfully() -> Result<(), String> {
    let mut null_arguments = common_arguments();
    null_arguments.extend([OsString::from("--tx"), OsString::from("null")]);
    let null_output = run_gpssim(&null_arguments)?;
    assert_bounded(&null_output);
    assert!(
        null_output.status.success(),
        "null run failed: {}",
        diagnostics(&null_output)
    );

    let output_path = unique_output_path()?;
    let mut file_arguments = common_arguments();
    file_arguments.extend([
        OsString::from("--output"),
        output_path.clone().into_os_string(),
    ]);
    let file_output = run_gpssim(&file_arguments)?;
    assert_bounded(&file_output);
    assert!(
        file_output.status.success(),
        "file run failed: {}",
        diagnostics(&file_output)
    );
    let metadata = std::fs::metadata(&output_path)
        .map_err(|error| format!("missing generated output: {error}"))?;
    assert!(metadata.len() > 0, "generated output was empty");
    std::fs::remove_file(&output_path).map_err(|error| {
        format!("failed to remove generated output: {error}")
    })?;
    Ok(())
}

#[test]
fn cli_generation_failure_is_nonzero() -> Result<(), String> {
    let mut arguments = common_arguments();
    let missing_navigation = unique_output_path()?.into_os_string();
    let ephemerides = arguments
        .get_mut(1)
        .ok_or_else(|| "common arguments omitted ephemerides".to_string())?;
    *ephemerides = missing_navigation;
    arguments.extend([OsString::from("--tx"), OsString::from("null")]);

    let output = run_gpssim(&arguments)?;
    assert_bounded(&output);
    assert!(!output.status.success());
    let diagnostics = diagnostics(&output);
    assert!(
        diagnostics.contains("NotFound")
            || diagnostics.contains("No such file")
            || diagnostics.contains("not found"),
        "missing navigation root cause was absent: {diagnostics}"
    );
    Ok(())
}

#[test]
fn non_terminal_tui_is_rejected_without_hanging() -> Result<(), String> {
    let output = run_gpssim(&[OsString::from("--tui")])?;
    assert_bounded(&output);
    assert!(!output.status.success());
    assert!(diagnostics(&output).contains("requires an interactive terminal"));
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
fn cli_dev_full_failure_is_nonzero() -> Result<(), String> {
    let mut arguments = common_arguments();
    arguments.extend([
        OsString::from("--output"),
        OsString::from("/dev/full"),
        OsString::from("--tx"),
        OsString::from("null"),
    ]);

    let output = run_gpssim(&arguments)?;
    assert_bounded(&output);
    assert!(!output.status.success());
    let diagnostics = diagnostics(&output);
    assert!(diagnostics.contains("/dev/full"));
    assert!(diagnostics.contains("No space left on device"));
    Ok(())
}

#[cfg(target_os = "linux")]
const LINUX_PTY_DRIVER: &str = r#"
import errno
import fcntl
import os
import re
import select
import struct
import subprocess
import sys
import termios
import time

binary, navigation = sys.argv[1:3]
master, slave = os.openpty()
fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 120, 0, 0))
environment = os.environ.copy()
environment.setdefault("TERM", "xterm-256color")
process = subprocess.Popen(
    [
        binary,
        "--tui",
        "--ephemerides", navigation,
        "--location", "35.681298,139.766247,10.0",
        "--duration", "0.000001",
        "--frequency", "1000000",
        "--bits", "8",
        "--output", "/dev/full",
        "--tx", "null",
    ],
    stdin=slave,
    stdout=slave,
    stderr=slave,
    close_fds=True,
    start_new_session=True,
    env=environment,
)
os.close(slave)
output = bytearray()

def read_once(timeout):
    readable, _, _ = select.select([master], [], [], timeout)
    if not readable:
        return False
    try:
        chunk = os.read(master, 65536)
    except OSError as error:
        if error.errno == errno.EIO:
            return False
        raise
    if not chunk:
        return False
    output.extend(chunk)
    return True

ansi_escape = re.compile(rb"\x1b\[[0-?]*[ -/]*[@-~]")

def compact(data):
    without_ansi = ansi_escape.sub(b"", bytes(data))
    return b"".join(without_ansi.split()).lower()

def fail(message):
    if process.poll() is None:
        process.kill()
        process.wait(timeout=5)
    os.close(master)
    sys.stderr.write(message + "\n")
    sys.stderr.write(output.decode("utf-8", "replace"))
    sys.exit(1)

ready_marker = b"Enter=start"
deadline = time.monotonic() + 10.0
while ready_marker not in output:
    remaining = deadline - time.monotonic()
    if remaining <= 0:
        fail("TUI was not ready for input before deadline")
    read_once(min(0.2, remaining))
    if process.poll() is not None:
        fail("TUI exited before becoming ready for input")

os.write(master, b"\r")
failure_marker = b"lasterror"
deadline = time.monotonic() + 20.0
while failure_marker not in compact(output):
    remaining = deadline - time.monotonic()
    if remaining <= 0:
        fail("TUI failure marker was not displayed before deadline")
    read_once(min(0.2, remaining))
    if process.poll() is not None:
        fail("TUI exited before displaying the worker failure")

os.write(master, b"q")
deadline = time.monotonic() + 10.0
while process.poll() is None:
    remaining = deadline - time.monotonic()
    if remaining <= 0:
        fail("TUI did not exit after q before deadline")
    read_once(min(0.2, remaining))

while read_once(0.05):
    pass
os.close(master)
text = output.decode("utf-8", "replace")
compact_text = compact(output).decode("utf-8", "replace")
if process.returncode == 0:
    sys.stderr.write(text)
    sys.exit("failed TUI run exited with status 0")
if "/dev/full" not in text or "nospaceleftondevice" not in compact_text:
    sys.stderr.write(text)
    sys.exit("final process diagnostics lost the /dev/full root cause")
print(f"CHILD_STATUS={process.returncode}")
print("ROOT_CAUSE=No space left on device")
print(text)
"#;

#[cfg(target_os = "linux")]
#[test]
fn tui_dev_full_failure_followed_by_quit_is_nonzero() -> Result<(), String> {
    let output = run_with_deadline(
        Path::new("python3"),
        &[
            OsString::from("-c"),
            OsString::from(LINUX_PTY_DRIVER),
            OsString::from(gpssim_binary()),
            navigation_path().into_os_string(),
        ],
        Duration::from_secs(40),
    )?;
    assert_bounded(&output);
    assert!(
        output.status.success(),
        "PTY driver failed: {}",
        diagnostics(&output)
    );
    let diagnostics = diagnostics(&output);
    assert!(diagnostics.contains("CHILD_STATUS="));
    assert!(diagnostics.contains("ROOT_CAUSE=No space left on device"));
    assert!(diagnostics.contains("/dev/full"));
    Ok(())
}

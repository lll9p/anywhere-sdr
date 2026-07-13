use std::{
    ffi::{OsStr, OsString},
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

use gps::Error;

pub static WORKSPACE_DIR: &str = env!("CARGO_WORKSPACE_DIR");
pub static OUTPUT_DIR: &str = concat!(env!("CARGO_WORKSPACE_DIR"), "/output");
pub static RESOURCES_DIR: &str =
    concat!(env!("CARGO_WORKSPACE_DIR"), "/resources");

static GPSSIM_MUTEX: Mutex<()> = Mutex::new(());
static TEMPORARY_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);
const COMPILER_FLAGS: &[&str] = &["-lm", "-O3"];
const IDENTITY_VERSION: &[u8] = b"anywhere-sdr-c-compatibility-v2";
const MANIFEST_VERSION: &[u8] = b"artifact-manifest-v1";
const MAX_DIAGNOSTIC_BYTES: usize = 16 * 1024;

struct CompiledGpsSim {
    path: PathBuf,
    identity: Vec<u8>,
}

#[derive(Clone, Copy)]
struct ArtifactFingerprint {
    length: u64,
    hash_a: u64,
    hash_b: u64,
}

fn append_identity_field(identity: &mut Vec<u8>, value: &[u8]) {
    identity.extend_from_slice(&(value.len() as u64).to_le_bytes());
    identity.extend_from_slice(value);
}

#[cfg(unix)]
fn append_os_str(identity: &mut Vec<u8>, value: &OsStr) {
    use std::os::unix::ffi::OsStrExt;

    append_identity_field(identity, value.as_bytes());
}

#[cfg(windows)]
fn append_os_str(identity: &mut Vec<u8>, value: &OsStr) {
    use std::os::windows::ffi::OsStrExt;

    let mut encoded = Vec::new();
    for unit in value.encode_wide() {
        encoded.extend_from_slice(&unit.to_le_bytes());
    }
    append_identity_field(identity, &encoded);
}

fn compilation_identity(
    compiler: &Path, source_path: &Path, source: &[u8],
    compiler_flags: &[OsString], working_directory: &Path,
) -> Vec<u8> {
    let mut identity = Vec::new();
    append_identity_field(&mut identity, IDENTITY_VERSION);
    append_os_str(&mut identity, compiler.as_os_str());
    append_os_str(&mut identity, source_path.as_os_str());
    append_identity_field(&mut identity, source);
    append_os_str(&mut identity, working_directory.as_os_str());
    identity.extend_from_slice(&(compiler_flags.len() as u64).to_le_bytes());
    for flag in compiler_flags {
        append_os_str(&mut identity, flag);
    }
    identity
}

fn executable_identity(
    compilation_identity: &[u8], fingerprint: ArtifactFingerprint,
) -> Vec<u8> {
    let mut identity = Vec::new();
    append_identity_field(&mut identity, compilation_identity);
    append_fingerprint(&mut identity, fingerprint);
    identity
}

fn output_identity(
    executable_identity: &[u8], arguments: &[OsString], output_path: &Path,
    working_directory: &Path,
) -> Vec<u8> {
    let mut identity = Vec::new();
    append_identity_field(&mut identity, IDENTITY_VERSION);
    append_identity_field(&mut identity, executable_identity);
    append_os_str(&mut identity, working_directory.as_os_str());
    identity.extend_from_slice(&(arguments.len() as u64).to_le_bytes());
    for argument in arguments {
        append_os_str(&mut identity, argument);
    }
    append_os_str(&mut identity, output_path.as_os_str());
    identity
}

fn identity_key(identity: &[u8]) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in identity {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

fn sibling_path(path: &Path, suffix: &str) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(suffix);
    PathBuf::from(value)
}

fn manifest_path(path: &Path) -> PathBuf {
    sibling_path(path, ".manifest")
}

fn temporary_path(path: &Path) -> PathBuf {
    let sequence = TEMPORARY_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    sibling_path(path, &format!(".tmp-{}-{sequence}", std::process::id()))
}

fn artifact_fingerprint(path: &Path) -> Result<ArtifactFingerprint, Error> {
    let mut file = File::open(path)?;
    let mut buffer = vec![0_u8; 64 * 1024];
    let mut length = 0_u64;
    let mut hash_a = 0xcbf2_9ce4_8422_2325_u64;
    let mut hash_b = 0x8422_2325_cbf2_9ce4_u64;
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        length = length.checked_add(count as u64).ok_or_else(|| {
            Error::msg(format!("Artifact {} is too large", path.display()))
        })?;
        for byte in &buffer[..count] {
            hash_a ^= u64::from(*byte);
            hash_a = hash_a.wrapping_mul(0x0000_0100_0000_01b3);
            hash_b ^= u64::from(*byte);
            hash_b = hash_b.rotate_left(5).wrapping_mul(0x517c_c1b7_2722_0a95);
        }
    }
    Ok(ArtifactFingerprint {
        length,
        hash_a,
        hash_b,
    })
}

fn append_fingerprint(
    manifest: &mut Vec<u8>, fingerprint: ArtifactFingerprint,
) {
    manifest.extend_from_slice(&fingerprint.length.to_le_bytes());
    manifest.extend_from_slice(&fingerprint.hash_a.to_le_bytes());
    manifest.extend_from_slice(&fingerprint.hash_b.to_le_bytes());
}

fn manifest_contents(
    identity: &[u8], fingerprint: ArtifactFingerprint,
) -> Vec<u8> {
    let mut manifest = Vec::new();
    append_identity_field(&mut manifest, MANIFEST_VERSION);
    append_identity_field(&mut manifest, identity);
    append_fingerprint(&mut manifest, fingerprint);
    manifest
}

fn cached_artifact_fingerprint(
    path: &Path, identity: &[u8],
) -> Result<Option<ArtifactFingerprint>, Error> {
    let manifest = match fs::read(manifest_path(path)) {
        Ok(manifest) => manifest,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(None);
        }
        Err(error) => return Err(error.into()),
    };
    if !path.is_file() {
        return Ok(None);
    }
    let fingerprint = artifact_fingerprint(path)?;
    if manifest == manifest_contents(identity, fingerprint) {
        Ok(Some(fingerprint))
    } else {
        Ok(None)
    }
}

fn remove_file_if_present(path: &Path) -> Result<(), Error> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn cleanup_paths(paths: &[&Path], message: &mut String) {
    for path in paths {
        if let Err(error) = remove_file_if_present(path) {
            message.push_str("; failed to remove ");
            message.push_str(&path.display().to_string());
            message.push_str(": ");
            message.push_str(&error.to_string());
        }
    }
}

fn install_artifact(
    temporary_artifact: &Path, artifact: &Path, identity: &[u8],
) -> Result<ArtifactFingerprint, Error> {
    let fingerprint =
        artifact_fingerprint(temporary_artifact).map_err(|error| {
            let mut message = format!(
                "Failed to fingerprint temporary artifact {}: {error}",
                temporary_artifact.display()
            );
            cleanup_paths(&[temporary_artifact], &mut message);
            Error::msg(message)
        })?;
    let manifest = manifest_path(artifact);
    let temporary_manifest = temporary_path(&manifest);
    if let Err(error) = fs::write(
        &temporary_manifest,
        manifest_contents(identity, fingerprint),
    ) {
        let mut message = format!(
            "Failed to write temporary cache manifest {}: {error}",
            temporary_manifest.display()
        );
        cleanup_paths(&[temporary_artifact, &temporary_manifest], &mut message);
        return Err(Error::msg(message));
    }

    let install_result = remove_file_if_present(&manifest)
        .and_then(|()| remove_file_if_present(artifact))
        .and_then(|()| {
            fs::rename(temporary_artifact, artifact).map_err(Into::into)
        })
        .and_then(|()| {
            fs::rename(&temporary_manifest, &manifest).map_err(Into::into)
        });
    if let Err(error) = install_result {
        let mut message = format!(
            "Failed to install cache artifact {}: {error}",
            artifact.display()
        );
        cleanup_paths(
            &[temporary_artifact, &temporary_manifest, artifact, &manifest],
            &mut message,
        );
        return Err(Error::msg(message));
    }
    Ok(fingerprint)
}

fn bounded_diagnostic(bytes: &[u8]) -> String {
    if bytes.len() <= MAX_DIAGNOSTIC_BYTES {
        return String::from_utf8_lossy(bytes).into_owned();
    }
    let half = MAX_DIAGNOSTIC_BYTES / 2;
    let omitted = bytes.len() - MAX_DIAGNOSTIC_BYTES;
    format!(
        "{}\n... {omitted} bytes omitted ...\n{}",
        String::from_utf8_lossy(&bytes[..half]),
        String::from_utf8_lossy(&bytes[bytes.len() - half..])
    )
}

fn process_failure(
    description: &str, program: &Path, output: &Output,
) -> Error {
    let stdout = bounded_diagnostic(&output.stdout);
    let stderr = bounded_diagnostic(&output.stderr);
    Error::msg(format!(
        "{description} `{}` failed with status {}\nstdout:\n{}\nstderr:\n{}",
        program.display(),
        output.status,
        stdout.trim_end(),
        stderr.trim_end()
    ))
}

fn process_start_failure(
    description: &str, program: &Path, error: std::io::Error,
    temporary_artifact: &Path,
) -> Error {
    let mut message = format!(
        "Failed to start {description} `{}`: {error}",
        program.display()
    );
    cleanup_paths(&[temporary_artifact], &mut message);
    Error::msg(message)
}

fn compile_gpssim(
    compiler: &Path, source_path: &Path, output_directory: &Path,
    compiler_flags: &[OsString], working_directory: &Path,
) -> Result<CompiledGpsSim, Error> {
    let source = fs::read(source_path).map_err(|error| {
        Error::msg(format!(
            "Failed to read C compatibility source {}: {error}",
            source_path.display()
        ))
    })?;
    let compilation_identity = compilation_identity(
        compiler,
        source_path,
        &source,
        compiler_flags,
        working_directory,
    );
    fs::create_dir_all(output_directory)?;

    let executable_name = format!(
        "gpssim-{}{}",
        identity_key(&compilation_identity),
        if cfg!(windows) { ".exe" } else { "" }
    );
    let executable = output_directory.join(executable_name);
    if let Some(fingerprint) =
        cached_artifact_fingerprint(&executable, &compilation_identity)?
    {
        return Ok(CompiledGpsSim {
            path: executable,
            identity: executable_identity(&compilation_identity, fingerprint),
        });
    }

    let temporary_executable = temporary_path(&executable);
    remove_file_if_present(&temporary_executable)?;
    let output = Command::new(compiler)
        .current_dir(working_directory)
        .arg(source_path)
        .args(compiler_flags)
        .arg("-o")
        .arg(&temporary_executable)
        .output()
        .map_err(|error| {
            process_start_failure(
                "C compiler",
                compiler,
                error,
                &temporary_executable,
            )
        })?;

    if !output.status.success() {
        let mut message =
            process_failure("C compatibility compiler", compiler, &output)
                .to_string();
        cleanup_paths(&[&temporary_executable], &mut message);
        return Err(Error::msg(message));
    }
    if !temporary_executable.is_file() {
        return Err(Error::msg(format!(
            "C compiler `{}` succeeded without creating {}",
            compiler.display(),
            temporary_executable.display()
        )));
    }

    let fingerprint = install_artifact(
        &temporary_executable,
        &executable,
        &compilation_identity,
    )?;
    Ok(CompiledGpsSim {
        path: executable,
        identity: executable_identity(&compilation_identity, fingerprint),
    })
}

fn runtime_arguments(
    params: &[Vec<String>], output_path: &Path,
) -> Result<Vec<OsString>, Error> {
    let mut arguments = Vec::new();
    for param in params {
        match param.as_slice() {
            [flag, _] if flag == "-o" => {}
            [flag, _] if flag == "-i" || flag == "-v" || flag == "-T" => {
                arguments.push(flag.into());
            }
            [flag, value] => {
                arguments.push(flag.into());
                arguments.push(value.into());
            }
            _ => {
                return Err(Error::msg(format!(
                    "Invalid C compatibility argument group: {param:?}"
                )));
            }
        }
    }
    arguments.push("-o".into());
    arguments.push(output_path.as_os_str().to_owned());
    Ok(arguments)
}

fn prepare_c_bin_with(
    params: &[Vec<String>], c_bin_file_path: &Path,
    executable: &CompiledGpsSim, working_directory: &Path,
) -> Result<(), Error> {
    if let Some(parent) = c_bin_file_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let identity_arguments = runtime_arguments(params, c_bin_file_path)?;
    let identity = output_identity(
        &executable.identity,
        &identity_arguments,
        c_bin_file_path,
        working_directory,
    );
    if cached_artifact_fingerprint(c_bin_file_path, &identity)?.is_some() {
        return Ok(());
    }

    let temporary_output = temporary_path(c_bin_file_path);
    remove_file_if_present(&temporary_output)?;
    let arguments = runtime_arguments(params, &temporary_output)?;
    let output = Command::new(&executable.path)
        .current_dir(working_directory)
        .args(&arguments)
        .output()
        .map_err(|error| {
            process_start_failure(
                "C compatibility executable",
                &executable.path,
                error,
                &temporary_output,
            )
        })?;

    if !output.status.success() {
        let mut message = process_failure(
            "C compatibility executable",
            &executable.path,
            &output,
        )
        .to_string();
        cleanup_paths(&[&temporary_output], &mut message);
        return Err(Error::msg(message));
    }
    if !temporary_output.is_file() {
        return Err(Error::msg(format!(
            "C compatibility executable succeeded without creating {}",
            temporary_output.display()
        )));
    }

    install_artifact(&temporary_output, c_bin_file_path, &identity)?;
    Ok(())
}

fn check_gpssim() -> Result<CompiledGpsSim, Error> {
    let source_path = PathBuf::from(RESOURCES_DIR).join("gpssim.c");
    if !source_path.is_file() {
        return Err(Error::msg(format!(
            "C compatibility source gpssim.c was not found under \
             {RESOURCES_DIR}"
        )));
    }
    let compiler_flags = COMPILER_FLAGS
        .iter()
        .map(OsString::from)
        .collect::<Vec<_>>();
    compile_gpssim(
        Path::new("gcc"),
        &source_path,
        Path::new(OUTPUT_DIR),
        &compiler_flags,
        Path::new(WORKSPACE_DIR),
    )
}

pub fn prepare_c_bin(
    params: &[Vec<String>], c_bin_file: &str,
) -> Result<(), Error> {
    let _lock = GPSSIM_MUTEX.lock().map_err(|error| {
        Error::msg(format!("Failed to acquire gpssim fixture lock: {error}"))
    })?;
    let executable = check_gpssim()?;
    prepare_c_bin_with(
        params,
        Path::new(c_bin_file),
        &executable,
        Path::new(WORKSPACE_DIR),
    )
}

#[cfg(all(test, debug_assertions))]
#[path = "support/prepare_tests.rs"]
mod tests;

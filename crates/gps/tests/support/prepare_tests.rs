use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use gps::Error;

use super::{
    MAX_DIAGNOSTIC_BYTES, bounded_diagnostic, cached_artifact_fingerprint,
    compilation_identity, compile_gpssim, install_artifact, manifest_path,
    output_identity, prepare_c_bin_with, runtime_arguments,
};

static TEST_DIRECTORY_SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn create() -> Result<Self, Error> {
        let sequence = TEST_DIRECTORY_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "anywhere-sdr-c-compatibility-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&path)?;
        Ok(Self(path))
    }

    fn path(&self) -> &Path {
        &self.0
    }

    fn temporary_files(&self) -> Result<Vec<PathBuf>, Error> {
        let mut paths = Vec::new();
        for entry in fs::read_dir(&self.0)? {
            let path = entry?.path();
            if path
                .file_name()
                .is_some_and(|name| name.to_string_lossy().contains(".tmp-"))
            {
                paths.push(path);
            }
        }
        Ok(paths)
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.0) {
            eprintln!(
                "Failed to remove test directory {}: {error}",
                self.0.display()
            );
        }
    }
}

fn flags(values: &[&str]) -> Vec<OsString> {
    values.iter().map(OsString::from).collect()
}

#[test]
fn missing_compiler_is_reported() -> Result<(), Error> {
    let directory = TestDirectory::create()?;
    let source = directory.path().join("valid.c");
    fs::write(&source, "int main(void) { return 0; }")?;
    let missing_compiler = directory.path().join("missing-compiler");

    let error = compile_gpssim(
        &missing_compiler,
        &source,
        directory.path(),
        &flags(&["-O0"]),
        directory.path(),
    )
    .err()
    .ok_or_else(|| Error::msg("Missing compiler unexpectedly succeeded"))?;
    let message = error.to_string();
    assert!(message.contains("Failed to start C compiler"));
    assert!(message.contains("missing-compiler"));

    let temporary_artifact = directory.path().join("install-source");
    let blocked_artifact = directory.path().join("blocked-artifact");
    fs::write(&temporary_artifact, b"complete")?;
    fs::create_dir(manifest_path(&blocked_artifact))?;
    let install_error = install_artifact(
        &temporary_artifact,
        &blocked_artifact,
        b"install identity",
    )
    .err()
    .ok_or_else(|| {
        Error::msg("Blocked artifact install unexpectedly succeeded")
    })?;
    assert!(install_error.to_string().contains("Failed to install"));
    assert!(!temporary_artifact.exists());
    assert!(!blocked_artifact.exists());
    assert!(directory.temporary_files()?.is_empty());
    Ok(())
}

#[test]
fn compiler_failure_includes_bounded_diagnostics() -> Result<(), Error> {
    let directory = TestDirectory::create()?;
    let source = directory.path().join("invalid.c");
    fs::write(&source, "int main(void) { this is not valid C; }")?;

    let error = compile_gpssim(
        Path::new("gcc"),
        &source,
        directory.path(),
        &flags(&["-O0"]),
        directory.path(),
    )
    .err()
    .ok_or_else(|| Error::msg("Invalid C source unexpectedly compiled"))?;
    let message = error.to_string();
    assert!(message.contains("C compatibility compiler"));
    assert!(message.contains("stderr:"));
    assert!(message.to_ascii_lowercase().contains("error"));

    let diagnostic = vec![b'x'; MAX_DIAGNOSTIC_BYTES * 3];
    let bounded = bounded_diagnostic(&diagnostic);
    assert!(bounded.contains("bytes omitted"));
    assert!(bounded.len() < MAX_DIAGNOSTIC_BYTES + 100);
    assert!(directory.temporary_files()?.is_empty());
    Ok(())
}

#[test]
#[allow(clippy::too_many_lines)]
fn compilation_cache_identity_and_manifest_are_complete() -> Result<(), Error> {
    let directory = TestDirectory::create()?;
    let source = directory.path().join("fixture.c");
    let alternate_source = directory.path().join("alternate.c");
    let working_directory = directory.path();
    let alternate_working_directory = directory.path().join("working");
    fs::create_dir(&alternate_working_directory)?;
    fs::write(&source, "int main(void) { return 0; }")?;

    let source_bytes = fs::read(&source)?;
    let base_identity = compilation_identity(
        Path::new("gcc"),
        &source,
        &source_bytes,
        &flags(&["-O0", "-Wall"]),
        working_directory,
    );
    assert_ne!(
        base_identity,
        compilation_identity(
            Path::new("cc"),
            &source,
            &source_bytes,
            &flags(&["-O0", "-Wall"]),
            working_directory,
        )
    );
    assert_ne!(
        base_identity,
        compilation_identity(
            Path::new("gcc"),
            &alternate_source,
            &source_bytes,
            &flags(&["-O0", "-Wall"]),
            working_directory,
        )
    );
    assert_ne!(
        base_identity,
        compilation_identity(
            Path::new("gcc"),
            &source,
            b"int main(void) { return 1; }",
            &flags(&["-O0", "-Wall"]),
            working_directory,
        )
    );
    assert_ne!(
        base_identity,
        compilation_identity(
            Path::new("gcc"),
            &source,
            &source_bytes,
            &flags(&["-Wall", "-O0"]),
            working_directory,
        )
    );
    assert_ne!(
        base_identity,
        compilation_identity(
            Path::new("gcc"),
            &source,
            &source_bytes,
            &flags(&["-O0", "-Wall"]),
            &alternate_working_directory,
        )
    );
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;

        let non_utf8_a =
            PathBuf::from(OsString::from_vec(b"compiler-\x80".to_vec()));
        let non_utf8_b =
            PathBuf::from(OsString::from_vec(b"compiler-\x81".to_vec()));
        assert_ne!(
            compilation_identity(
                &non_utf8_a,
                &source,
                &source_bytes,
                &flags(&["-O0", "-Wall"]),
                working_directory,
            ),
            compilation_identity(
                &non_utf8_b,
                &source,
                &source_bytes,
                &flags(&["-O0", "-Wall"]),
                working_directory,
            )
        );
    }

    let first = compile_gpssim(
        Path::new("gcc"),
        &source,
        directory.path(),
        &flags(&["-O0"]),
        working_directory,
    )?;
    let cached = compile_gpssim(
        Path::new("gcc"),
        &source,
        directory.path(),
        &flags(&["-O0"]),
        working_directory,
    )?;
    assert_eq!(first.path, cached.path);
    assert_eq!(first.identity, cached.identity);
    assert!(
        cached_artifact_fingerprint(&first.path, b"different exact identity")?
            .is_none()
    );

    fs::write(&first.path, b"corrupt executable")?;
    let repaired = compile_gpssim(
        Path::new("gcc"),
        &source,
        directory.path(),
        &flags(&["-O0"]),
        working_directory,
    )?;
    assert_eq!(first.path, repaired.path);
    assert_ne!(fs::read(&repaired.path)?, b"corrupt executable");

    fs::write(&source, "int main(void) { return 1; }")?;
    let source_changed = compile_gpssim(
        Path::new("gcc"),
        &source,
        directory.path(),
        &flags(&["-O0"]),
        working_directory,
    )?;
    assert_ne!(first.path, source_changed.path);

    let flags_changed = compile_gpssim(
        Path::new("gcc"),
        &source,
        directory.path(),
        &flags(&["-O3"]),
        working_directory,
    )?;
    assert_ne!(source_changed.path, flags_changed.path);
    assert!(directory.temporary_files()?.is_empty());
    Ok(())
}

#[test]
#[allow(clippy::too_many_lines)]
fn runtime_cache_identity_manifest_and_cleanup_are_complete()
-> Result<(), Error> {
    let directory = TestDirectory::create()?;
    let source = directory.path().join("argument-writer.c");
    fs::write(
        &source,
        r#"
#include <stdio.h>
#include <string.h>

int main(int argc, char **argv) {
    const char *output = NULL;
    const char *value = "";
    int fail = 0;
    for (int index = 1; index < argc; ++index) {
        if (strcmp(argv[index], "-o") == 0 && index + 1 < argc) {
            output = argv[++index];
        } else if (strcmp(argv[index], "--value") == 0 && index + 1 < argc) {
            value = argv[++index];
        } else if (strcmp(argv[index], "--fail") == 0 && index + 1 < argc) {
            fail = 1;
            ++index;
        }
    }
    if (output == NULL) {
        return 2;
    }
    FILE *file = fopen(output, "wb");
    if (file == NULL) {
        return 3;
    }
    if (fputs(value, file) == EOF || fclose(file) != 0) {
        return 4;
    }
    if (fail) {
        fputs("deliberate runtime failure\n", stderr);
        return 5;
    }
    return 0;
}
"#,
    )?;
    let executable = compile_gpssim(
        Path::new("gcc"),
        &source,
        directory.path(),
        &flags(&["-O0"]),
        directory.path(),
    )?;
    let output = directory.path().join("fixture.bin");
    let first_params = vec![
        vec!["--value".to_string(), "first".to_string()],
        vec!["-o".to_string(), "ignored.bin".to_string()],
    ];
    prepare_c_bin_with(&first_params, &output, &executable, directory.path())?;
    assert_eq!(fs::read_to_string(&output)?, "first");

    let first_arguments = runtime_arguments(&first_params, &output)?;
    let first_identity = output_identity(
        &executable.identity,
        &first_arguments,
        &output,
        directory.path(),
    );
    let alternate_output = directory.path().join("alternate.bin");
    let alternate_arguments =
        runtime_arguments(&first_params, &alternate_output)?;
    assert_ne!(
        first_identity,
        output_identity(
            &executable.identity,
            &alternate_arguments,
            &alternate_output,
            directory.path(),
        )
    );
    assert_ne!(
        first_identity,
        output_identity(
            b"different executable",
            &first_arguments,
            &output,
            directory.path(),
        )
    );
    assert_ne!(
        first_identity,
        output_identity(
            &executable.identity,
            &first_arguments,
            &output,
            &directory.path().join("other-working-directory"),
        )
    );

    fs::write(&output, b"stale")?;
    prepare_c_bin_with(&first_params, &output, &executable, directory.path())?;
    assert_eq!(fs::read_to_string(&output)?, "first");
    fs::write(manifest_path(&output), b"wrong exact identity")?;
    prepare_c_bin_with(&first_params, &output, &executable, directory.path())?;
    assert_eq!(fs::read_to_string(&output)?, "first");

    let second_params = vec![
        vec!["--value".to_string(), "second".to_string()],
        vec!["-o".to_string(), "ignored.bin".to_string()],
    ];
    prepare_c_bin_with(&second_params, &output, &executable, directory.path())?;
    assert_eq!(fs::read_to_string(&output)?, "second");

    let failing_params = vec![
        vec!["--value".to_string(), "partial".to_string()],
        vec!["--fail".to_string(), "yes".to_string()],
        vec!["-o".to_string(), "ignored.bin".to_string()],
    ];
    let error = prepare_c_bin_with(
        &failing_params,
        &output,
        &executable,
        directory.path(),
    )
    .err()
    .ok_or_else(|| Error::msg("Failing C fixture unexpectedly succeeded"))?;
    let message = error.to_string();
    assert!(message.contains("failed with status"));
    assert!(message.contains("deliberate runtime failure"));
    assert_eq!(fs::read_to_string(&output)?, "second");
    assert!(directory.temporary_files()?.is_empty());
    Ok(())
}

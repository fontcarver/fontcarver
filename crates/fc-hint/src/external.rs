//! Optional out-of-process ttfautohint integration; no bundled or linked library.

use std::{
    fs::{self, File},
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use read_fonts::{FontRef, TableProvider};

use crate::Error;

const MAX_FONT_BYTES: u64 = 256 * 1024 * 1024;
const MAX_LOG_BYTES: u64 = 4 * 1024 * 1024;

/// Discovers the optional user-installed hinter without executing a shell.
pub fn executable() -> Option<PathBuf> {
    let name = if cfg!(windows) {
        "ttfautohint.exe"
    } else {
        "ttfautohint"
    };
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|directory| directory.join(name))
        .find(|path| is_executable(path))
}

fn is_executable(path: &Path) -> bool {
    let Ok(metadata) = path.metadata() else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

/// Hints a static TrueType sfnt using the installed executable and private files.
///
/// Font permission restrictions are left to ttfautohint; no override flag is used.
/// The input is never modified. Timeout, size limits and nonzero exits are errors.
pub fn autohint(bytes: &[u8]) -> Result<Vec<u8>, Error> {
    let executable = executable().ok_or(Error::Unavailable)?;
    run(&executable, bytes, Duration::from_secs(120))
}

fn run(executable: &Path, bytes: &[u8], timeout: Duration) -> Result<Vec<u8>, Error> {
    if bytes.len() as u64 > MAX_FONT_BYTES {
        return Err(Error::Invalid(
            "TrueType autohint input exceeds 256 MiB".into(),
        ));
    }
    let face = FontRef::new(bytes).map_err(|e| Error::Invalid(e.to_string()))?;
    face.glyf()
        .map_err(|_| Error::Invalid("ttfautohint requires TrueType outlines".into()))?;
    if face.fvar().is_ok() {
        return Err(Error::Invalid(
            "ttfautohint requires a static master or instance".into(),
        ));
    }
    let glyph_count = face
        .maxp()
        .map_err(|e| Error::Invalid(e.to_string()))?
        .num_glyphs();
    let directory = tempfile::tempdir()?;
    let input = directory.path().join("input.ttf");
    let output = directory.path().join("output.ttf");
    let log = directory.path().join("stderr.log");
    fs::write(&input, bytes)?;
    let stderr = File::create(&log)?;
    let child = Command::new(executable)
        .arg("--no-info")
        .arg(&input)
        .arg(&output)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(stderr)
        .spawn()?;
    let mut child = RunningChild(child);
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.0.try_wait()? {
            break status;
        }
        if start.elapsed() >= timeout {
            return Err(Error::External(format!(
                "timed out after {} milliseconds",
                timeout.as_millis()
            )));
        }
        if fs::metadata(&output).is_ok_and(|m| m.len() > MAX_FONT_BYTES)
            || fs::metadata(&log).is_ok_and(|m| m.len() > MAX_LOG_BYTES)
        {
            return Err(Error::External("output resource limit exceeded".into()));
        }
        thread::sleep(Duration::from_millis(20));
    };
    if fs::metadata(&log)?.len() > MAX_LOG_BYTES {
        return Err(Error::External("output resource limit exceeded".into()));
    }
    if !status.success() {
        let mut file = File::open(log)?;
        let len = file.metadata()?.len();
        file.seek(SeekFrom::Start(len.saturating_sub(65_536)))?;
        let mut detail = Vec::new();
        file.take(65_536).read_to_end(&mut detail)?;
        return Err(Error::External(format!(
            "{status}: {}",
            String::from_utf8_lossy(&detail).trim()
        )));
    }
    if fs::metadata(&output)?.len() > MAX_FONT_BYTES {
        return Err(Error::External("output font exceeds 256 MiB".into()));
    }
    let output = fs::read(output)?;
    let result = FontRef::new(&output).map_err(|e| Error::External(e.to_string()))?;
    result.glyf().map_err(|e| Error::External(e.to_string()))?;
    result
        .loca(None)
        .map_err(|e| Error::External(e.to_string()))?;
    if result
        .maxp()
        .map_err(|e| Error::External(e.to_string()))?
        .num_glyphs()
        != glyph_count
    {
        return Err(Error::External("output glyph count changed".into()));
    }
    Ok(output)
}

struct RunningChild(Child);
impl Drop for RunningChild {
    fn drop(&mut self) {
        // Reap on every error path, including output validation or temporary I/O.
        if !matches!(self.0.try_wait(), Ok(Some(_))) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn invoke(script: &str, timeout: Duration) -> Result<Vec<u8>, Error> {
        let directory = tempfile::tempdir().unwrap();
        let executable = directory.path().join("test hinter");
        fs::write(&executable, format!("#!/bin/sh\n{script}\n")).unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
        run(
            &executable,
            include_bytes!("../../../fixtures/fonts/DejaVuSans.ttf"),
            timeout,
        )
    }

    #[test]
    fn external_process_handles_quoted_paths_and_rejects_invalid_results() {
        let bytes = invoke("cp \"$2\" \"$3\"", Duration::from_secs(5)).unwrap();
        assert_eq!(
            bytes,
            include_bytes!("../../../fixtures/fonts/DejaVuSans.ttf")
        );
        let error = invoke(
            "printf 'hinter refused input' >&2; exit 3",
            Duration::from_secs(5),
        )
        .unwrap_err();
        assert!(error.to_string().contains("hinter refused input"));
        assert!(invoke("printf 'invalid font' > \"$3\"", Duration::from_secs(5)).is_err());
    }

    #[test]
    fn external_process_timeout_reaps_the_child_and_returns_promptly() {
        let start = Instant::now();
        let result = invoke("exec sleep 60", Duration::from_millis(30));
        assert!(result.unwrap_err().to_string().contains("timed out"));
        assert!(start.elapsed() < Duration::from_secs(5));
    }
}

//! Optional out-of-process ttfautohint integration; no bundled or linked library.
//!
//! FontCarver never links, bundles or vendors `ttfautohint`. It discovers a
//! user-installed executable on `PATH`, reports honestly when none is present,
//! passes a validated subset of the tool's documented options, and keeps every
//! message the tool writes so a successful run can still disclose warnings.

use std::{
    ffi::OsStr,
    fs::{self, File},
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use read_fonts::{FontRef, TableProvider};

use crate::Error;

mod composite;

#[cfg(all(test, unix))]
mod real_tests;

const MAX_FONT_BYTES: u64 = 256 * 1024 * 1024;
const MAX_LOG_BYTES: u64 = 4 * 1024 * 1024;
const MAX_MESSAGES: usize = 64;
const MAX_MESSAGE_CHARS: usize = 400;

/// How much provenance `ttfautohint` may record in the output `name` table.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AutohintInfo {
    /// Pass `--no-info`: the version string is not added to the `name` table.
    #[default]
    None,
    /// Pass neither flag: the tool adds its short info string.
    Short,
    /// Pass `--detailed-info`: the tool records the full option list.
    Detailed,
}

/// A validated subset of the documented `ttfautohint` command-line options.
///
/// Every field is validated by [`AutohintOptions::validate`] before any process
/// starts, so no unchecked user text can become a command-line switch. Fields
/// left at their default are not passed at all, which keeps the tool's own
/// defaults authoritative.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AutohintOptions {
    /// `--hinting-range-min`: smallest ppem the generated programs hint.
    pub hinting_range_min: Option<u16>,
    /// `--hinting-range-max`: largest ppem the generated programs hint.
    pub hinting_range_max: Option<u16>,
    /// `--hinting-limit`: largest ppem that is hinted at all; `0` removes the limit.
    pub hinting_limit: Option<u16>,
    /// `--fallback-stem-width`: stem width for scripts without blue zones.
    pub fallback_stem_width: Option<u16>,
    /// `--increase-x-height`: ppem limit for x-height rounding up; `0` switches it off.
    pub increase_x_height: Option<u16>,
    /// `--x-height-snapping-exceptions`: ppem values and ranges, or `-` for all.
    pub x_height_snapping_exceptions: String,
    /// `--default-script`: four-letter script tag for glyphs without a script.
    pub default_script: String,
    /// `--fallback-script`: four-letter script tag for unassigned glyphs.
    pub fallback_script: String,
    /// `--strong-stem-width`: any of `g` (grayscale), `G` (GDI) and `D` (DirectWrite).
    pub strong_stem_width: String,
    /// `--adjust-subglyphs`: run the subglyph adjustment pass.
    pub adjust_subglyphs: bool,
    /// `--composites`: hint composite glyphs as a whole.
    pub composites: bool,
    /// `--dehint`: remove all hints instead of generating new ones.
    pub dehint: bool,
    /// `--symbol`: accept a font without any standard characters.
    pub symbol: bool,
    /// `--ttfa-table`: add a `TTFA` table with the tool's parameters.
    pub ttfa_table: bool,
    /// `--windows-compatibility`: add blue zones for Windows vertical clipping.
    pub windows_compatibility: bool,
    /// How much provenance the tool records in the `name` table.
    pub info: AutohintInfo,
}

impl AutohintOptions {
    /// Rejects out-of-range numbers and anything that is not a plain value.
    ///
    /// Script tags must be exactly four lowercase ASCII letters, the stem-width
    /// mode may only contain `g`, `G` and `D`, and the snapping exceptions must
    /// be `-` or a comma-separated list of ppem values and `min-max` ranges.
    pub fn validate(&self) -> Result<(), Error> {
        let ppem = |value: Option<u16>, zero_allowed: bool| {
            value.is_none_or(|v| (2..=10_000).contains(&v) || (zero_allowed && v == 0))
        };
        if !ppem(self.hinting_range_min, false)
            || !ppem(self.hinting_range_max, false)
            || !ppem(self.hinting_limit, true)
            || !ppem(self.increase_x_height, true)
            || !self
                .fallback_stem_width
                .is_none_or(|v| (1..=10_000).contains(&v))
        {
            return Err(Error::Invalid(
                "ttfautohint ppem and stem-width options must be within 2..=10000".into(),
            ));
        }
        if let (Some(min), Some(max)) = (self.hinting_range_min, self.hinting_range_max) {
            if min > max {
                return Err(Error::Invalid(
                    "ttfautohint hinting range minimum exceeds its maximum".into(),
                ));
            }
        }
        if let (Some(limit), Some(max)) = (self.hinting_limit, self.hinting_range_max) {
            if limit != 0 && limit < max {
                return Err(Error::Invalid(
                    "ttfautohint hinting limit must be zero or at least the range maximum".into(),
                ));
            }
        }
        for script in [&self.default_script, &self.fallback_script] {
            if !script.is_empty()
                && (script.len() != 4 || !script.bytes().all(|b| b.is_ascii_lowercase()))
            {
                return Err(Error::Invalid(
                    "a ttfautohint script must be four lowercase ASCII letters".into(),
                ));
            }
        }
        if self.strong_stem_width.len() > 3
            || !self
                .strong_stem_width
                .chars()
                .all(|c| matches!(c, 'g' | 'G' | 'D'))
        {
            return Err(Error::Invalid(
                "ttfautohint strong stem widths may only contain g, G and D".into(),
            ));
        }
        validate_exceptions(&self.x_height_snapping_exceptions)
    }

    fn arguments(&self) -> Vec<String> {
        let mut args = Vec::new();
        let mut number = |flag: &str, value: Option<u16>| {
            if let Some(value) = value {
                args.push(format!("{flag}={value}"));
            }
        };
        number("--hinting-range-min", self.hinting_range_min);
        number("--hinting-range-max", self.hinting_range_max);
        number("--hinting-limit", self.hinting_limit);
        number("--fallback-stem-width", self.fallback_stem_width);
        number("--increase-x-height", self.increase_x_height);
        for (flag, value) in [
            (
                "--x-height-snapping-exceptions",
                &self.x_height_snapping_exceptions,
            ),
            ("--default-script", &self.default_script),
            ("--fallback-script", &self.fallback_script),
            ("--strong-stem-width", &self.strong_stem_width),
        ] {
            if !value.is_empty() {
                args.push(format!("{flag}={value}"));
            }
        }
        for (flag, enabled) in [
            ("--adjust-subglyphs", self.adjust_subglyphs),
            ("--composites", self.composites),
            ("--dehint", self.dehint),
            ("--symbol", self.symbol),
            ("--ttfa-table", self.ttfa_table),
            ("--windows-compatibility", self.windows_compatibility),
        ] {
            if enabled {
                args.push(flag.to_owned());
            }
        }
        match self.info {
            AutohintInfo::None => args.push("--no-info".to_owned()),
            AutohintInfo::Short => {}
            AutohintInfo::Detailed => args.push("--detailed-info".to_owned()),
        }
        args
    }
}

fn validate_exceptions(value: &str) -> Result<(), Error> {
    if value.is_empty() || value == "-" {
        return Ok(());
    }
    let invalid = || Error::Invalid("invalid ttfautohint x-height snapping exceptions".into());
    for item in value.split(',') {
        let item = item.trim();
        let (low, high) = match item.split_once('-') {
            Some((low, high)) => (low.trim(), high.trim()),
            None => (item, item),
        };
        let low: u16 = low.parse().map_err(|_| invalid())?;
        let high: u16 = high.parse().map_err(|_| invalid())?;
        if !(2..=10_000).contains(&low) || !(2..=10_000).contains(&high) || low > high {
            return Err(invalid());
        }
    }
    Ok(())
}

/// An installed external hinter and the version string it reported.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Availability {
    /// Absolute path of the executable that was found on `PATH`.
    pub path: PathBuf,
    /// First line of the tool's `--version` output, empty when it reported none.
    pub version: String,
}

/// The result of a successful external hinting run.
#[derive(Clone, Debug)]
pub struct AutohintOutcome {
    /// Validated hinted font bytes.
    pub bytes: Vec<u8>,
    /// The hinter's reported version, empty when it could not be read.
    pub version: String,
    /// Bounded messages the tool wrote while succeeding.
    pub messages: Vec<String>,
}

/// Discovers the optional user-installed hinter without executing a shell.
pub fn executable() -> Option<PathBuf> {
    find(std::env::split_paths(&std::env::var_os("PATH")?))
}

/// Reports the installed hinter and its version, or `None` when it is absent.
///
/// A tool that cannot be executed or does not answer `--version` promptly is
/// still reported, with an empty version, because the export path would use it.
pub fn availability() -> Option<Availability> {
    let path = executable()?;
    let version = version_of(&path);
    Some(Availability { path, version })
}

fn find(directories: impl Iterator<Item = PathBuf>) -> Option<PathBuf> {
    let name = if cfg!(windows) {
        "ttfautohint.exe"
    } else {
        "ttfautohint"
    };
    directories
        .map(|directory| directory.join(name))
        .find(|path| is_executable(path))
}

fn version_of(executable: &Path) -> String {
    capture(
        executable,
        [OsStr::new("--version")],
        Duration::from_secs(10),
    )
    .ok()
    .and_then(|output| {
        output
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty())
            .map(|line| line.chars().take(MAX_MESSAGE_CHARS).collect())
    })
    .unwrap_or_default()
}

fn capture<const N: usize>(
    executable: &Path,
    args: [&OsStr; N],
    timeout: Duration,
) -> Result<String, Error> {
    let directory = tempfile::tempdir()?;
    let log = directory.path().join("stdout.log");
    let child = Command::new(executable)
        .args(args)
        .stdin(Stdio::null())
        .stdout(File::create(&log)?)
        .stderr(Stdio::null())
        .spawn()?;
    let mut child = RunningChild(child);
    let start = Instant::now();
    loop {
        if child.0.try_wait()?.is_some() {
            break;
        }
        if start.elapsed() >= timeout || fs::metadata(&log).is_ok_and(|m| m.len() > MAX_LOG_BYTES) {
            return Err(Error::External("version query did not complete".into()));
        }
        thread::sleep(Duration::from_millis(10));
    }
    Ok(String::from_utf8_lossy(&read_tail(&log, 65_536)?).into_owned())
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
/// An absent executable is [`Error::Unavailable`], never a silent success.
pub fn autohint(bytes: &[u8], options: &AutohintOptions) -> Result<AutohintOutcome, Error> {
    options.validate()?;
    let executable = executable().ok_or(Error::Unavailable)?;
    let version = version_of(&executable);
    let (bytes, messages) = run(&executable, bytes, options, Duration::from_secs(120))?;
    Ok(AutohintOutcome {
        bytes,
        version,
        messages,
    })
}

fn run(
    executable: &Path,
    bytes: &[u8],
    options: &AutohintOptions,
    timeout: Duration,
) -> Result<(Vec<u8>, Vec<String>), Error> {
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
        .args(options.arguments())
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
    let detail = read_tail(&log, 65_536)?;
    if !status.success() {
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
    let output_glyph_count = result
        .maxp()
        .map_err(|e| Error::External(e.to_string()))?
        .num_glyphs();
    // Composite hinting may append one internal .ttfautohint glyph. It keeps
    // existing glyph IDs intact and is never mapped by cmap; see the upstream
    // manual's "The .ttfautohint Glyph" section. Subglyph adjustment and dehinting
    // do not use this helper. Reject every other glyph-count change.
    let composite_helper = options.composites
        && !options.adjust_subglyphs
        && !options.dehint
        && glyph_count.checked_add(1) == Some(output_glyph_count);
    if output_glyph_count != glyph_count
        && !(composite_helper && composite::valid_helper(&result, glyph_count)?)
    {
        return Err(Error::External("output glyph count changed".into()));
    }
    Ok((output, messages(&detail)))
}

/// Keeps a bounded number of bounded lines so a chatty tool cannot flood a report.
fn messages(bytes: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(bytes)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .take(MAX_MESSAGES)
        .map(|line| line.chars().take(MAX_MESSAGE_CHARS).collect())
        .collect()
}

fn read_tail(path: &Path, limit: u64) -> Result<Vec<u8>, Error> {
    let mut file = File::open(path)?;
    let len = file.metadata()?.len();
    file.seek(SeekFrom::Start(len.saturating_sub(limit)))?;
    let mut detail = Vec::new();
    file.take(limit).read_to_end(&mut detail)?;
    Ok(detail)
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

    fn hinter(directory: &Path, script: &str) -> PathBuf {
        let executable = directory.join("ttfautohint");
        fs::write(&executable, format!("#!/bin/sh\n{script}\n")).unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
        executable
    }

    fn invoke(script: &str, timeout: Duration) -> Result<(Vec<u8>, Vec<String>), Error> {
        let directory = tempfile::tempdir().unwrap();
        let executable = directory.path().join("test hinter");
        fs::write(&executable, format!("#!/bin/sh\n{script}\n")).unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
        run(
            &executable,
            include_bytes!("../../../fixtures/fonts/DejaVuSans.ttf"),
            &AutohintOptions::default(),
            timeout,
        )
    }

    #[test]
    fn external_process_handles_quoted_paths_and_rejects_invalid_results() {
        // Default options pass exactly one flag, so the paths are "$2" and "$3".
        let (bytes, messages) = invoke("cp \"$2\" \"$3\"", Duration::from_secs(5)).unwrap();
        assert_eq!(
            bytes,
            include_bytes!("../../../fixtures/fonts/DejaVuSans.ttf")
        );
        assert!(messages.is_empty());
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

    #[test]
    fn composite_mode_rejects_changed_counts_without_a_valid_helper() {
        let directory = tempfile::tempdir().unwrap();
        let tool = hinter(
            directory.path(),
            "for arg do output=\"$arg\"; done; cp \"$(dirname \"$0\")/forged.ttf\" \"$output\"",
        );
        let source = include_bytes!("../../../fixtures/fonts/DejaVuSans.ttf");
        let face = FontRef::new(source).unwrap();
        let count = face.maxp().unwrap().num_glyphs();
        let offset = face
            .table_directory()
            .table_records()
            .iter()
            .find(|table| table.tag() == read_fonts::types::Tag::new(b"maxp"))
            .unwrap()
            .offset() as usize
            + 4;
        for forged_count in [count - 1, count + 1, count + 2] {
            let mut forged = source.to_vec();
            forged[offset..offset + 2].copy_from_slice(&forged_count.to_be_bytes());
            fs::write(directory.path().join("forged.ttf"), forged).unwrap();
            let result = run(
                &tool,
                source,
                &AutohintOptions {
                    composites: true,
                    ..AutohintOptions::default()
                },
                Duration::from_secs(5),
            );
            assert!(matches!(result, Err(Error::External(_))), "{forged_count}");
        }
    }

    #[test]
    fn a_successful_run_keeps_the_tools_own_warnings() {
        let (_, messages) = invoke(
            "printf 'warning: glyph 7 is empty\\n\\nwarning: second\\n' >&2; cp \"$2\" \"$3\"",
            Duration::from_secs(5),
        )
        .unwrap();
        assert_eq!(
            messages,
            ["warning: glyph 7 is empty", "warning: second"]
                .map(str::to_owned)
                .to_vec()
        );
    }

    #[test]
    fn absence_is_reported_and_never_claimed_as_success() {
        let empty = tempfile::tempdir().unwrap();
        assert_eq!(find(std::iter::once(empty.path().to_path_buf())), None);
        let directory = tempfile::tempdir().unwrap();
        let path = hinter(directory.path(), "exit 0");
        assert_eq!(
            find(
                [empty.path().to_path_buf(), directory.path().to_path_buf()]
                    .into_iter()
                    .chain(std::iter::once(empty.path().to_path_buf()))
            ),
            Some(path)
        );
    }

    #[test]
    fn version_is_read_from_the_tools_first_nonempty_line() {
        let directory = tempfile::tempdir().unwrap();
        let path = hinter(directory.path(), "printf '\\nttfautohint 1.8.4\\nmore\\n'");
        assert_eq!(version_of(&path), "ttfautohint 1.8.4");
        let silent = hinter(directory.path(), "exit 0");
        assert_eq!(version_of(&silent), "");
    }

    #[test]
    fn validated_options_become_exactly_the_documented_flags() {
        let options = AutohintOptions {
            hinting_range_min: Some(8),
            hinting_range_max: Some(50),
            hinting_limit: Some(0),
            fallback_stem_width: Some(70),
            increase_x_height: Some(0),
            x_height_snapping_exceptions: "10,14-16".into(),
            default_script: "latn".into(),
            fallback_script: "none".into(),
            strong_stem_width: "gGD".into(),
            adjust_subglyphs: true,
            composites: true,
            dehint: false,
            symbol: true,
            ttfa_table: true,
            windows_compatibility: true,
            info: AutohintInfo::Detailed,
        };
        options.validate().unwrap();
        assert_eq!(
            options.arguments(),
            [
                "--hinting-range-min=8",
                "--hinting-range-max=50",
                "--hinting-limit=0",
                "--fallback-stem-width=70",
                "--increase-x-height=0",
                "--x-height-snapping-exceptions=10,14-16",
                "--default-script=latn",
                "--fallback-script=none",
                "--strong-stem-width=gGD",
                "--adjust-subglyphs",
                "--composites",
                "--symbol",
                "--ttfa-table",
                "--windows-compatibility",
                "--detailed-info",
            ]
        );
        assert_eq!(
            AutohintOptions::default().arguments(),
            ["--no-info".to_owned()]
        );
        assert_eq!(
            AutohintOptions {
                info: AutohintInfo::Short,
                ..AutohintOptions::default()
            }
            .arguments(),
            Vec::<String>::new()
        );
    }

    #[test]
    fn invalid_options_are_refused_before_any_process_starts() {
        for options in [
            AutohintOptions {
                hinting_range_min: Some(20),
                hinting_range_max: Some(10),
                ..AutohintOptions::default()
            },
            AutohintOptions {
                hinting_range_max: Some(50),
                hinting_limit: Some(20),
                ..AutohintOptions::default()
            },
            AutohintOptions {
                default_script: "Latin".into(),
                ..AutohintOptions::default()
            },
            AutohintOptions {
                strong_stem_width: "gx".into(),
                ..AutohintOptions::default()
            },
            AutohintOptions {
                x_height_snapping_exceptions: "12;14".into(),
                ..AutohintOptions::default()
            },
            AutohintOptions {
                x_height_snapping_exceptions: "16-14".into(),
                ..AutohintOptions::default()
            },
            AutohintOptions {
                hinting_range_min: Some(1),
                ..AutohintOptions::default()
            },
            AutohintOptions {
                fallback_stem_width: Some(0),
                ..AutohintOptions::default()
            },
        ] {
            let error = options.validate().unwrap_err();
            assert!(matches!(error, Error::Invalid(_)), "{options:?}");
            // Validation runs before the executable is even looked up, so the
            // option error, not `Unavailable`, is what the caller sees.
            assert_eq!(
                autohint(b"", &options).unwrap_err().to_string(),
                error.to_string(),
                "invalid options must not reach the process: {options:?}"
            );
        }
        for options in [
            AutohintOptions {
                x_height_snapping_exceptions: "-".into(),
                ..AutohintOptions::default()
            },
            AutohintOptions {
                hinting_limit: Some(0),
                increase_x_height: Some(0),
                ..AutohintOptions::default()
            },
        ] {
            options.validate().unwrap();
        }
    }
}

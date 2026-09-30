//! Bounded JSON-line bridge to an explicitly invoked installed Python process.

use std::{
    io::{BufRead, BufReader, Read, Write},
    process::{Child, Command, Stdio},
    sync::{atomic::Ordering, mpsc},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use serde_json::{json, Value};

use crate::{Api, ApiError, AutomationPythonParams, AutomationPythonResult, METHODS};

use super::SOURCE_LIMIT;

const BRIDGE: &str = include_str!("python_bridge.py");
const MESSAGE_LIMIT: usize = 256 * 1024;
const OUTPUT_LIMIT: usize = 64 * 1024;
const RPC_LIMIT: u32 = 1000;

struct Running<'a>(&'a std::sync::atomic::AtomicBool);

impl Drop for Running<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

struct OwnedChild(Child);

impl Drop for OwnedChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

struct Supervisor {
    stop: mpsc::Sender<()>,
    thread: Option<JoinHandle<()>>,
}

impl Drop for Supervisor {
    fn drop(&mut self) {
        let _ = self.stop.send(());
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn supervise(mut child: OwnedChild, deadline: Instant) -> Result<Supervisor, ApiError> {
    let (stop, receiver) = mpsc::channel();
    let thread = thread::Builder::new()
        .name("fontcarver-python-watchdog".into())
        .spawn(move || loop {
            if Instant::now() >= deadline {
                break;
            }
            match child.0.try_wait() {
                Ok(Some(_)) | Err(_) => break,
                Ok(None) => {}
            }
            let delay = deadline
                .saturating_duration_since(Instant::now())
                .min(Duration::from_millis(20));
            match receiver.recv_timeout(delay) {
                Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
        })
        .map_err(|error| ApiError::internal(error.to_string()))?;
    Ok(Supervisor {
        stop,
        thread: Some(thread),
    })
}

fn spawn_python(candidates: &[&str]) -> Result<OwnedChild, ApiError> {
    for executable in candidates {
        match Command::new(executable)
            .args(["-I", "-X", "utf8", "-u", "-c", BRIDGE])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(child) => return Ok(OwnedChild(child)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(ApiError::io(error.to_string())),
        }
    }
    Err(ApiError {
        code: "python_unavailable".into(),
        message: "Install Python 3 and make python3 or python available on PATH".into(),
    })
}

fn read_messages(reader: impl Read, sink: mpsc::SyncSender<Result<Value, String>>) {
    let mut reader = BufReader::new(reader);
    loop {
        let mut line = Vec::new();
        let result = (&mut reader)
            .take(MESSAGE_LIMIT as u64 + 2)
            .read_until(b'\n', &mut line);
        let message = match result {
            Ok(0) => Err("Python exited without a completion message".into()),
            Ok(_) if line.len() > MESSAGE_LIMIT + 1 => {
                Err("Python RPC message exceeds 262144 bytes".into())
            }
            Ok(_) if line.last() != Some(&b'\n') => Err("Python RPC message is incomplete".into()),
            Ok(_) => serde_json::from_slice(&line)
                .map_err(|error| format!("Invalid Python RPC message: {error}")),
            Err(error) => Err(format!("Python output read failed: {error}")),
        };
        let failed = message.is_err();
        if sink.send(message).is_err() || failed {
            break;
        }
    }
}

fn write_messages(mut writer: impl Write, source: mpsc::Receiver<Vec<u8>>) {
    while let Ok(message) = source.recv() {
        if writer
            .write_all(&message)
            .and_then(|()| writer.write_all(b"\n"))
            .and_then(|()| writer.flush())
            .is_err()
        {
            break;
        }
    }
}

pub(super) fn run(
    api: &Api,
    params: AutomationPythonParams,
) -> Result<AutomationPythonResult, ApiError> {
    if params.source.len() > SOURCE_LIMIT {
        return Err(ApiError::invalid_params(
            "Python source exceeds 65536 bytes",
        ));
    }
    let timeout = params.timeout_ms.unwrap_or(10_000);
    if !(100..=30_000).contains(&timeout) {
        return Err(ApiError::invalid_params(
            "Python timeout_ms must be between 100 and 30000",
        ));
    }
    if api
        .automation
        .running
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Err(ApiError {
            code: "automation_busy".into(),
            message: "A Python script is already running".into(),
        });
    }
    let _running = Running(&api.automation.running);
    let started = Instant::now();
    let deadline = started + Duration::from_millis(timeout.into());
    let mut child = spawn_python(&["python3", "python"])?;
    let stdout = child
        .0
        .stdout
        .take()
        .ok_or_else(|| ApiError::internal("Python stdout pipe unavailable"))?;
    let stdin = child
        .0
        .stdin
        .take()
        .ok_or_else(|| ApiError::internal("Python stdin pipe unavailable"))?;
    let _supervisor = supervise(child, deadline)?;
    let (messages, received) = mpsc::sync_channel(16);
    thread::Builder::new()
        .name("fontcarver-python-output".into())
        .spawn(move || read_messages(stdout, messages))
        .map_err(|error| ApiError::internal(error.to_string()))?;
    let (responses, writes) = mpsc::sync_channel(1);
    thread::Builder::new()
        .name("fontcarver-python-input".into())
        .spawn(move || write_messages(stdin, writes))
        .map_err(|error| ApiError::internal(error.to_string()))?;
    let config = json!({"source": params.source, "context": {"doc": params.doc, "master": params.master, "glyph": params.glyph}});
    responses
        .try_send(
            serde_json::to_vec(&config).map_err(|error| ApiError::internal(error.to_string()))?,
        )
        .map_err(|error| ApiError::internal(error.to_string()))?;
    let mut result = AutomationPythonResult {
        output: Vec::new(),
        error: None,
        elapsed_ms: 0,
        rpc_calls: 0,
    };
    let mut output_bytes = 0;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            result.error = Some("Python script exceeded its deadline".into());
            break;
        }
        let message = match received.recv_timeout(remaining) {
            Ok(Ok(message)) => message,
            Ok(Err(error)) => {
                result.error = Some(if Instant::now() >= deadline {
                    "Python script exceeded its deadline".into()
                } else {
                    error
                });
                break;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                result.error = Some("Python script exceeded its deadline".into());
                break;
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                result.error = Some("Python output connection closed".into());
                break;
            }
        };
        match message.get("kind").and_then(Value::as_str) {
            Some("output") => {
                let Some(text) = message.get("text").and_then(Value::as_str) else {
                    result.error = Some("Invalid Python output message".into());
                    break;
                };
                output_bytes += text.len() + 1;
                if output_bytes > OUTPUT_LIMIT {
                    result.error = Some("Python output exceeds 65536 bytes".into());
                    break;
                }
                result.output.push(text.into());
            }
            Some("call") => {
                result.rpc_calls += 1;
                if result.rpc_calls > RPC_LIMIT {
                    result.error = Some("Python RPC count exceeds 1000 calls".into());
                    break;
                }
                let response = dispatch_rpc(api, &params, &message);
                let response = match response {
                    Ok(value) => json!({"result": value}),
                    Err(error) => json!({"error": error}),
                };
                let bytes = serde_json::to_vec(&response)
                    .map_err(|error| ApiError::internal(error.to_string()))?;
                if bytes.len() > MESSAGE_LIMIT {
                    result.error = Some("Python RPC response exceeds 262144 bytes".into());
                    break;
                }
                if responses.try_send(bytes).is_err() {
                    result.error = Some("Python input connection closed".into());
                    break;
                }
            }
            Some("done") => {
                result.error = match message.get("error") {
                    Some(Value::Null) => None,
                    Some(Value::String(error)) => Some(error.clone()),
                    _ => Some("Invalid Python completion message".into()),
                };
                break;
            }
            _ => {
                result.error = Some("Unknown Python RPC message".into());
                break;
            }
        }
    }
    if Instant::now() >= deadline {
        result.error = Some("Python script exceeded its deadline".into());
    }
    result.elapsed_ms = started.elapsed().as_millis().min(u128::from(u32::MAX)) as u32;
    Ok(result)
}

fn dispatch_rpc(
    api: &Api,
    context: &AutomationPythonParams,
    message: &Value,
) -> Result<Value, ApiError> {
    let method = message
        .get("method")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::invalid_params("Python RPC method must be a string"))?;
    let params = message.get("params").cloned().unwrap_or_else(|| json!({}));
    if method.starts_with("automation.")
        || method.starts_with("process.")
        || matches!(
            method,
            "prefs.set" | "doc.edit_begin" | "doc.edit_apply" | "doc.edit_end"
        )
        || (method.starts_with("session.")
            && !matches!(
                method,
                "session.list" | "session.faces" | "session.recent" | "session.recover_list"
            ))
    {
        return Err(ApiError::invalid_params(format!("Python RPC does not permit {method}; recursion, process and session lifecycle calls are disabled")));
    }
    if !METHODS.iter().any(|entry| entry.0 == method) {
        return Err(ApiError::not_found(format!(
            "unknown API method `{method}`"
        )));
    }
    if is_mutation(method) {
        let doc = context.doc.ok_or_else(|| {
            ApiError::invalid_params("Python mutations require a captured document")
        })?;
        if params.get("doc") != Some(&json!(doc)) {
            return Err(ApiError::invalid_params(
                "Python mutations must target the captured document",
            ));
        }
    }
    api.call(method, params)
}

fn is_mutation(method: &str) -> bool {
    // Explicitly enumerated read methods; newly added methods require a captured
    // document until they have been reviewed for scripting access.
    !matches!(
        method,
        "session.list"
            | "session.faces"
            | "session.recent"
            | "session.recover_list"
            | "prefs.get"
            | "doc.summary"
            | "doc.font_info"
            | "doc.sidebar"
            | "doc.find_problems"
            | "doc.metrics_resolve"
            | "glyphs.list"
            | "glyphs.index_of"
            | "glyphs.cell"
            | "glyphs.detail"
            | "glyphs.unihan"
            | "glyphs.search"
            | "glyphs.outline"
            | "glyphs.flattened_components"
            | "glyphs.ids"
            | "hints.tt_state"
            | "hints.tt_preview"
            | "hints.state"
            | "hints.analyze"
            | "effects.mask"
            | "cid.state"
            | "svg.state"
            | "color.state"
            | "color.graph_preview"
            | "color.preview"
            | "math.state"
            | "math.preview"
            | "variation.state"
            | "variation.preview"
            | "bitmap.preview"
            | "bitmap.state"
            | "bitmap.glyph"
            | "features.get"
            | "features.validate"
            | "layout.inspect"
            | "layout.rules"
            | "layout.rule_glyphs"
            | "layout.prepare_conversion"
            | "kerning.status"
            | "kerning.glyph"
            | "kerning.resolve"
            | "kerning.pairs"
            | "kerning.status_vertical"
            | "kerning.resolve_vertical"
            | "kerning.pairs_vertical"
            | "kerning.groups"
            | "kerning.members"
            | "text.shape"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn missing_interpreter_has_an_actionable_stable_error() {
        let error = match super::spawn_python(&["fontcarver-no-such-python-executable"]) {
            Ok(_) => panic!("unexpected executable found"),
            Err(error) => error,
        };
        assert_eq!(error.code, "python_unavailable");
        assert!(error.message.contains("Python 3"));
    }
}

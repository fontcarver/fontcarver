use std::{fs, time::Instant};

use fc_api::Api;
use serde_json::{json, Value};
use tempfile::TempDir;

const SOURCE_LIMIT: usize = 64 * 1024;

fn configured_api() -> (TempDir, Api) {
    let config = tempfile::tempdir().expect("temporary configuration");
    let api = Api::with_config_dir(config.path());
    (config, api)
}

fn run_python(api: &Api, source: &str) -> Value {
    api.call("automation.python_run", json!({ "source": source }))
        .expect("run installed Python interpreter")
}

fn output(result: &Value) -> String {
    result["output"]
        .as_array()
        .expect("output array")
        .iter()
        .map(|line| line.as_str().expect("output string"))
        .collect::<Vec<_>>()
        .join("")
}

fn script_error(result: &Value) -> &str {
    result["error"].as_str().expect("script failure in result")
}

#[test]
fn script_discovery_reads_direct_sources_without_executing_them() {
    let (config, api) = configured_api();
    let scripts = config.path().join("scripts");
    fs::create_dir_all(scripts.join("nested")).expect("create scripts directory");
    let marker = config.path().join("unexpected-execution");
    let python = format!(
        "from pathlib import Path\nPath({}).write_text('executed')\n",
        serde_json::to_string(&marker.to_string_lossy()).expect("marker literal")
    );
    let javascript = "throw new Error('reload must not run this');\n";
    fs::write(scripts.join("b.py"), &python).expect("write Python script");
    fs::write(scripts.join("a.js"), javascript).expect("write JavaScript script");
    fs::write(scripts.join("README.txt"), "not a script").expect("write non-script");
    fs::write(scripts.join("nested/hidden.py"), "print('hidden')").expect("write nested script");
    fs::write(config.path().join("outside.py"), "print('outside')")
        .expect("write source outside scripts directory");

    for method in ["automation.scripts_list", "automation.scripts_reload"] {
        let result = api.call(method, json!({})).expect("discover scripts");
        assert_eq!(
            result["folders"],
            json!([{
                "kind": "preferences",
                "path": scripts.to_string_lossy().as_ref(),
                "exists": true,
            }])
        );
        assert_eq!(
            result["scripts"],
            json!([
                { "name": "a.js", "folder": "preferences", "language": "javascript", "bytes": javascript.len() },
                { "name": "b.py", "folder": "preferences", "language": "python", "bytes": python.len() },
            ])
        );
        assert!(result["warnings"].is_array());
        assert_eq!(result["fingerprint"].as_str().map(str::len), Some(64));
    }
    assert!(!marker.exists(), "discovery executed a local script");

    assert_eq!(
        api.call("automation.script_read", json!({ "name": "b.py" }))
            .expect("load Python source"),
        json!({ "name": "b.py", "folder": "preferences", "language": "python", "source": python })
    );
    assert!(!marker.exists(), "reading executed a local script");

    fs::write(scripts.join("a.js"), "print('updated');").expect("update source");
    fs::remove_file(scripts.join("b.py")).expect("remove source");
    let refreshed = api
        .call("automation.scripts_reload", json!({}))
        .expect("reload changed directory");
    assert_eq!(refreshed["scripts"].as_array().unwrap().len(), 1);
    assert_eq!(
        api.call("automation.script_read", json!({ "name": "a.js" }))
            .expect("read latest source")["source"],
        "print('updated');"
    );
}

#[test]
fn an_absent_scripts_directory_has_an_empty_listing() {
    let (_config, api) = configured_api();
    let result = api
        .call("automation.scripts_list", json!({}))
        .expect("list without a scripts directory");
    assert_eq!(result["scripts"], json!([]));
}

#[test]
fn script_read_rejects_traversal_absolute_paths_and_unsupported_names() {
    let (config, api) = configured_api();
    fs::create_dir(config.path().join("scripts")).expect("scripts directory");
    let outside = config.path().join("outside.py");
    fs::write(&outside, "print('private')").expect("outside source");
    for name in [
        "../outside.py".to_owned(),
        "nested/script.py".to_owned(),
        "nested\\script.py".to_owned(),
        outside.to_string_lossy().into_owned(),
        "".to_owned(),
        ".".to_owned(),
        "README.txt".to_owned(),
    ] {
        let error = api
            .call("automation.script_read", json!({ "name": name }))
            .expect_err("invalid name must not read a path");
        assert_eq!(error.code, "invalid_params", "name: {name}");
    }
}

#[test]
fn script_discovery_and_read_bound_source_bytes_and_reject_invalid_utf8() {
    let (config, api) = configured_api();
    let scripts = config.path().join("scripts");
    fs::create_dir(&scripts).expect("scripts directory");
    fs::write(scripts.join("exact.py"), "#".repeat(SOURCE_LIMIT)).expect("source at byte limit");
    fs::write(scripts.join("large.py"), "#".repeat(SOURCE_LIMIT + 1))
        .expect("source beyond byte limit");
    fs::write(scripts.join("invalid.py"), [0xff, 0xfe]).expect("invalid UTF-8 source");

    assert_eq!(
        api.call("automation.script_read", json!({ "name": "exact.py" }))
            .expect("source at limit is readable")["source"]
            .as_str()
            .unwrap()
            .len(),
        SOURCE_LIMIT
    );
    for name in ["large.py", "invalid.py"] {
        assert!(api
            .call("automation.script_read", json!({ "name": name }))
            .is_err());
    }
    let result = api
        .call("automation.scripts_list", json!({}))
        .expect("list bounded sources");
    assert!(!result["scripts"]
        .as_array()
        .unwrap()
        .iter()
        .any(|script| script["name"] == "large.py"));
    assert!(!result["warnings"].as_array().unwrap().is_empty());
}

#[test]
fn script_discovery_limits_the_number_of_listed_files() {
    let (config, api) = configured_api();
    let scripts = config.path().join("scripts");
    fs::create_dir(&scripts).expect("scripts directory");
    for index in 0..257 {
        fs::write(scripts.join(format!("script-{index:03}.py")), "pass\n")
            .expect("write discovery limit fixture");
    }
    let result = api
        .call("automation.scripts_reload", json!({}))
        .expect("bounded discovery");
    assert_eq!(result["scripts"].as_array().unwrap().len(), 256);
    assert!(!result["warnings"].as_array().unwrap().is_empty());
}

#[cfg(unix)]
#[test]
fn script_discovery_and_read_reject_symlinked_sources() {
    use std::os::unix::fs::symlink;

    let (config, api) = configured_api();
    let scripts = config.path().join("scripts");
    fs::create_dir(&scripts).expect("scripts directory");
    let outside = config.path().join("outside.py");
    fs::write(&outside, "print('private')").expect("outside source");
    symlink(&outside, scripts.join("linked.py")).expect("symlinked script");

    let result = api
        .call("automation.scripts_list", json!({}))
        .expect("list without following source symlink");
    assert_eq!(result["scripts"], json!([]));
    assert!(api
        .call("automation.script_read", json!({ "name": "linked.py" }))
        .is_err());
}

#[cfg(unix)]
#[test]
fn script_discovery_and_read_reject_a_symlinked_scripts_directory() {
    use std::os::unix::fs::symlink;

    let (config, api) = configured_api();
    let outside = config.path().join("outside");
    fs::create_dir(&outside).expect("outside directory");
    fs::write(outside.join("private.py"), "print('private')").expect("outside source");
    symlink(&outside, config.path().join("scripts")).expect("symlink scripts directory");

    match api.call("automation.scripts_list", json!({})) {
        Ok(result) => assert_eq!(result["scripts"], json!([])),
        Err(error) => assert_eq!(error.code, "invalid_params"),
    }
    assert!(api
        .call("automation.script_read", json!({ "name": "private.py" }))
        .is_err());
}

#[test]
fn python_returns_stdout_and_runtime_errors_without_poisoning_the_api() {
    let (_config, api) = configured_api();
    let success = run_python(&api, "print('hello from Python 字体 é')\nprint(2 + 3)\n");
    assert_eq!(success["error"], Value::Null);
    assert!(output(&success).contains("hello from Python 字体 é"));
    assert!(output(&success).contains('5'));
    assert_eq!(success["rpc_calls"], 0);
    assert!(success["elapsed_ms"].is_number());

    let failed = run_python(
        &api,
        "print('before failure')\nraise ValueError('macro failed')\n",
    );
    assert!(output(&failed).contains("before failure"));
    assert!(script_error(&failed).contains("macro failed"));
    assert_eq!(run_python(&api, "print('recovered')")["error"], Value::Null);
    assert_eq!(api.call("session.list", json!({})).unwrap(), json!([]));
}

#[test]
fn python_rpc_uses_the_same_session_and_records_one_undoable_run() {
    let (config, api) = configured_api();
    let saved = config.path().join("python-result.fcf");
    let created = api
        .call(
            "session.new",
            json!({ "family": "Python automation", "units_per_em": 1000 }),
        )
        .expect("create document before running a script");
    let source = r#"
import fontcarver
doc = fontcarver.context['doc']
assert any(item['doc'] == doc for item in fontcarver.call('session.list', {}))
added = fontcarver.call('glyphs.add', {'doc': doc, 'recipe': 'A'})
gid = added['added'][0]['gid']
listed = fontcarver.call('glyphs.list', {'doc': doc, 'offset': 0, 'limit': 10, 'include_paths': False})
assert listed['total'] == 1
outline = fontcarver.call('glyphs.outline', {'doc': doc, 'gid': gid})
fontcarver.call('doc.apply', {'doc': doc, 'label': 'Python width', 'command': {'SetWidth': {'glyph': 'A', 'layer': outline['layer'], 'width': 725}}})
try:
    fontcarver.call('doc.undo', {'doc': doc})
    raise AssertionError('history navigation inside a run must be rejected')
except RuntimeError as error:
    assert 'script transaction' in str(error), str(error)
assert fontcarver.call('glyphs.outline', {'doc': doc, 'gid': gid})['width'] == 725
print('edited')
"#;

    let result = api
        .call(
            "automation.python_run",
            json!({ "source": source, "doc": created["doc"], "label": "Python run" }),
        )
        .expect("run document script");
    assert_eq!(result["error"], Value::Null, "{result}");
    assert_eq!(result["rpc_calls"], 7);
    assert_eq!(result["undo_label"], "Python run");
    let docs = api.call("session.list", json!({})).expect("shared session");
    assert_eq!(docs.as_array().unwrap().len(), 1);
    assert_eq!(docs[0]["family_name"], "Python automation");
    let glyphs = api
        .call(
            "glyphs.search",
            json!({ "doc": docs[0]["doc"], "query": "A", "limit": 10 }),
        )
        .expect("read Python edit through API");
    assert_eq!(glyphs[0]["width"], 725.0);

    // One run is one history entry: a single undo removes the added glyph and
    // the width change together, and redo restores both.
    api.call("doc.undo", json!({ "doc": created["doc"] }))
        .expect("undo the whole run");
    assert_eq!(
        api.call(
            "glyphs.list",
            json!({ "doc": created["doc"], "offset": 0, "limit": 10, "include_paths": false }),
        )
        .expect("inventory after undo")["total"],
        0
    );
    api.call("doc.redo", json!({ "doc": created["doc"] }))
        .expect("redo the whole run");
    assert_eq!(
        api.call(
            "glyphs.search",
            json!({ "doc": created["doc"], "query": "A", "limit": 10 }),
        )
        .expect("state after redo")[0]["width"],
        725.0
    );

    api.call(
        "session.save",
        json!({ "doc": created["doc"], "path": saved }),
    )
    .expect("save script edits through the same API");
    assert!(saved.is_file());
    let reopened = api
        .call("session.open", json!({ "path": saved }))
        .expect("reopen saved Python edit");
    let persisted = api
        .call(
            "glyphs.search",
            json!({ "doc": reopened["doc"], "query": "A", "limit": 10 }),
        )
        .expect("persisted glyph state");
    assert_eq!(persisted[0]["width"], 725.0);
}

#[test]
fn python_receives_captured_context_and_can_edit_its_document() {
    let (_config, api) = configured_api();
    let doc = api
        .call(
            "session.new",
            json!({ "family": "Context", "units_per_em": 1000 }),
        )
        .expect("create captured document");
    let result = api
        .call(
            "automation.python_run",
            json!({
                "doc": doc["doc"],
                "master": doc["masters"][0]["id"],
                "glyph": 17,
                "source": "import fontcarver\nassert fontcarver.context['doc'] is not None\nassert fontcarver.context['master'] is not None\nassert fontcarver.context['glyph'] == 17\nfontcarver.call('glyphs.add', {'doc': fontcarver.context['doc'], 'recipe': 'A'})\n"
            }),
        )
        .expect("run with captured context");
    assert_eq!(result["error"], Value::Null, "{result}");
    assert_eq!(
        api.call("doc.summary", json!({ "doc": doc["doc"] }))
            .unwrap()["glyph_count"],
        1
    );
}

#[test]
fn python_with_a_captured_document_rejects_edits_to_another_document() {
    let (_config, api) = configured_api();
    let create = |family| {
        api.call(
            "session.new",
            json!({ "family": family, "units_per_em": 1000 }),
        )
        .expect("create document")
    };
    let captured = create("Captured");
    let other = create("Other");
    let source = format!(
        "import fontcarver\nfontcarver.call('glyphs.add', {{'doc': {}, 'recipe': 'A'}})\n",
        other["doc"]
    );
    let result = api
        .call(
            "automation.python_run",
            json!({ "source": source, "doc": captured["doc"] }),
        )
        .expect("run captured script");
    assert!(!script_error(&result).is_empty());
    assert_eq!(
        api.call("doc.summary", json!({ "doc": other["doc"] }))
            .unwrap()["glyph_count"],
        0
    );
}

#[test]
fn python_rpc_rejects_recursive_automation_calls() {
    let (_config, api) = configured_api();
    for method in ["automation.python_run", "automation.scripts_reload"] {
        let source =
            format!("import fontcarver\nfontcarver.call('{method}', {{'source': 'print(123)'}})\n");
        let result = run_python(&api, &source);
        assert!(!script_error(&result).is_empty(), "method: {method}");
    }
}

#[test]
fn python_rpc_rejects_session_file_and_process_operations() {
    let (config, api) = configured_api();
    let created = api
        .call(
            "session.new",
            json!({ "family": "Captured", "units_per_em": 1000 }),
        )
        .expect("create captured document");
    let path = config.path().join("unexpected-save.fcf");
    for (method, params) in [
        (
            "session.new",
            json!({ "family": "Unexpected", "units_per_em": 1000 }),
        ),
        ("session.open", json!({ "path": path })),
        ("session.close", json!({ "doc": created["doc"] })),
        (
            "session.save",
            json!({ "doc": created["doc"], "path": path }),
        ),
        ("session.recover", json!({ "path": path })),
        ("prefs.set", json!({ "prefs": {} })),
        ("session.export", json!({ "doc": created["doc"] })),
        ("process.run", json!({ "doc": created["doc"] })),
    ] {
        let source = format!(
            "import fontcarver, json\nfontcarver.call('{method}', json.loads({}))\n",
            serde_json::to_string(&params.to_string()).expect("Python JSON literal")
        );
        let result = api
            .call(
                "automation.python_run",
                json!({ "source": source, "doc": created["doc"] }),
            )
            .expect("run script with blocked method");
        assert!(
            script_error(&result).contains("does not permit"),
            "method: {method}, result: {result}"
        );
    }
    assert!(!path.exists(), "script wrote to a session file path");
    let docs = api
        .call("session.list", json!({}))
        .expect("session remains intact");
    assert_eq!(docs.as_array().unwrap().len(), 1);
    assert_eq!(docs[0]["doc"], created["doc"]);
}

#[test]
fn python_mutations_require_a_captured_document() {
    let (_config, api) = configured_api();
    let created = api
        .call(
            "session.new",
            json!({ "family": "Uncaptured", "units_per_em": 1000 }),
        )
        .expect("create document");
    let source = format!(
        "import fontcarver\nfontcarver.call('glyphs.add', {{'doc': {}, 'recipe': 'A'}})\n",
        created["doc"]
    );
    let result = run_python(&api, &source);
    assert!(!script_error(&result).is_empty());
    assert_eq!(
        api.call("doc.summary", json!({ "doc": created["doc"] }))
            .unwrap()["glyph_count"],
        0
    );
}

#[test]
fn python_deadline_stops_an_endless_loop_and_leaves_the_api_usable() {
    let (_config, api) = configured_api();
    let started = Instant::now();
    let result = api
        .call(
            "automation.python_run",
            json!({ "source": "while True:\n    pass\n", "timeout_ms": 100 }),
        )
        .expect("deadline is a script failure");
    assert!(
        started.elapsed().as_secs() < 5,
        "script exceeded deadline grace"
    );
    let error = script_error(&result).to_lowercase();
    assert!(
        error.contains("time") || error.contains("deadline"),
        "{error}"
    );
    assert_eq!(run_python(&api, "pass")["error"], Value::Null);
}

#[test]
fn python_rejects_oversized_source_before_execution() {
    let (_config, api) = configured_api();
    for source in [
        "#".repeat(SOURCE_LIMIT + 1),
        "é".repeat(SOURCE_LIMIT / 2 + 1),
    ] {
        let error = api
            .call("automation.python_run", json!({ "source": source }))
            .expect_err("source limit uses bytes");
        assert_eq!(error.code, "invalid_params");
    }
}

#[test]
fn python_rejects_timeouts_outside_the_supported_range() {
    let (_config, api) = configured_api();
    for timeout_ms in [0_u32, 99, 30_001, u32::MAX] {
        let error = api
            .call(
                "automation.python_run",
                json!({ "source": "pass", "timeout_ms": timeout_ms }),
            )
            .expect_err("out-of-range timeout");
        assert_eq!(error.code, "invalid_params", "timeout_ms: {timeout_ms}");
    }
    assert_eq!(
        api.call(
            "automation.python_run",
            json!({ "source": "pass", "timeout_ms": 30_000 }),
        )
        .expect("maximum timeout accepted")["error"],
        Value::Null
    );
}

#[test]
fn python_rpc_stops_at_the_call_limit() {
    let (_config, api) = configured_api();
    let result = run_python(
        &api,
        "import fontcarver\nfor index in range(1001):\n    fontcarver.call('session.list', {})\n",
    );
    assert!(!script_error(&result).is_empty());
    assert_eq!(result["rpc_calls"], 1001);
}

#[test]
fn python_bounds_output_and_individual_rpc_messages() {
    let (_config, api) = configured_api();
    let loud = run_python(&api, "print('x' * (64 * 1024 + 1))\n");
    assert!(!script_error(&loud).is_empty());
    assert!(output(&loud).len() <= SOURCE_LIMIT);

    let oversized_message = run_python(
        &api,
        "import fontcarver\nfontcarver.call('session.new', {'family': 'x' * (256 * 1024 + 1), 'units_per_em': 1000})\n",
    );
    assert!(!script_error(&oversized_message).is_empty());
    assert_eq!(api.call("session.list", json!({})).unwrap(), json!([]));
}

#[test]
fn python_rejects_raw_oversized_and_malformed_protocol_messages() {
    let (_config, api) = configured_api();
    for (source, expected) in [
        (
            "import os\nos.write(1, b'x' * (256 * 1024 + 1) + b'\\n')\n",
            "exceeds",
        ),
        ("import os\nos.write(1, b'not-json\\n')\n", "Invalid"),
        (
            "import os\nos.write(1, b'{\"kind\":\"unknown\"}\\n')\n",
            "Unknown",
        ),
        (
            "import os, json\nos.write(1, (json.dumps({'kind': 'output', 'text': 'x' * 65536}) + '\\n').encode())\n",
            "output exceeds",
        ),
    ] {
        let result = run_python(&api, source);
        assert!(script_error(&result).contains(expected), "{result}");
        assert!(output(&result).len() <= SOURCE_LIMIT);
    }
    assert_eq!(run_python(&api, "pass")["error"], Value::Null);
}

#[test]
fn a_script_transaction_merges_every_command_into_one_undo_entry() {
    let (_config, api) = configured_api();
    let created = api
        .call(
            "session.new",
            json!({ "family": "Transaction", "units_per_em": 1000 }),
        )
        .expect("create document");
    let doc = created["doc"].clone();

    let transaction = api
        .call(
            "automation.transaction_begin",
            json!({ "doc": doc, "label": "Script: batch.js" }),
        )
        .expect("open a script transaction");
    assert_eq!(transaction["doc"], doc);
    for recipe in ["A", "B", "C"] {
        api.call("glyphs.add", json!({ "doc": doc, "recipe": recipe }))
            .expect("add a glyph inside the transaction");
    }
    // History navigation inside a run is rejected rather than splitting history.
    let rejected = api
        .call("doc.undo", json!({ "doc": doc }))
        .expect_err("undo inside a transaction must be rejected");
    assert!(rejected.message.contains("script transaction"));
    let ended = api
        .call(
            "automation.transaction_end",
            json!({ "doc": doc, "token": transaction["token"] }),
        )
        .expect("close the transaction");
    assert_eq!(ended["ended"], true);
    assert_eq!(ended["undo_label"], "Script: batch.js");

    let summary = api.call("doc.summary", json!({ "doc": doc })).unwrap();
    assert_eq!(summary["undo_label"], "Script: batch.js");
    api.call("doc.undo", json!({ "doc": doc }))
        .expect("undo the whole run");
    assert_eq!(
        api.call(
            "glyphs.list",
            json!({ "doc": doc, "offset": 0, "limit": 10, "include_paths": false }),
        )
        .unwrap()["total"],
        0
    );
    api.call("doc.redo", json!({ "doc": doc })).unwrap();
    assert_eq!(
        api.call(
            "glyphs.list",
            json!({ "doc": doc, "offset": 0, "limit": 10, "include_paths": false }),
        )
        .unwrap()["total"],
        3
    );
    // A superseded token is reported, never an error: the console always closes
    // its transaction in a finally block.
    let again = api
        .call(
            "automation.transaction_end",
            json!({ "doc": doc, "token": transaction["token"] }),
        )
        .expect("closing twice is not an error");
    assert_eq!(again["ended"], false);
    assert!(again["undo_label"].is_null());
}

#[test]
fn a_script_transaction_label_is_validated_and_a_failed_run_stays_undoable() {
    let (_config, api) = configured_api();
    let created = api
        .call(
            "session.new",
            json!({ "family": "Labels", "units_per_em": 1000 }),
        )
        .expect("create document");
    let doc = created["doc"].clone();
    for label in [json!(""), json!("   "), json!("x".repeat(121))] {
        assert!(api
            .call(
                "automation.transaction_begin",
                json!({ "doc": doc, "label": label }),
            )
            .is_err());
    }
    let transaction = api
        .call(
            "automation.transaction_begin",
            json!({ "doc": doc, "label": "Script: partial.js" }),
        )
        .unwrap();
    api.call("glyphs.add", json!({ "doc": doc, "recipe": "A" }))
        .unwrap();
    // A rejected command inside the run leaves the earlier work in the entry.
    assert!(api
        .call(
            "doc.apply",
            json!({
                "doc": doc,
                "label": "Rejected",
                "command": {"SetWidth": {"glyph": "absent", "layer": "missing", "width": 10.0}},
            }),
        )
        .is_err());
    api.call(
        "automation.transaction_end",
        json!({ "doc": doc, "token": transaction["token"] }),
    )
    .unwrap();
    api.call("doc.undo", json!({ "doc": doc })).unwrap();
    assert_eq!(
        api.call(
            "glyphs.list",
            json!({ "doc": doc, "offset": 0, "limit": 10, "include_paths": false }),
        )
        .unwrap()["total"],
        0
    );
}

#[test]
fn configured_and_project_script_folders_are_searched_and_reads_are_root_scoped() {
    let (config, api) = configured_api();
    let extra = config.path().join("shared-scripts");
    fs::create_dir_all(&extra).expect("configured folder");
    fs::write(extra.join("shared.js"), "// shared\n").expect("configured script");
    let project = config.path().join("project");
    fs::create_dir_all(project.join("scripts")).expect("project folder");
    fs::write(project.join("scripts/local.py"), "# local\n").expect("project script");
    let document = project.join("Family.fcf");
    let created = api
        .call(
            "session.new",
            json!({ "family": "Folders", "units_per_em": 1000 }),
        )
        .expect("create document");
    api.call(
        "session.save",
        json!({ "doc": created["doc"], "path": document }),
    )
    .expect("save so the document has a project folder");

    let settings = api
        .call(
            "automation.settings_set",
            json!({ "scripts_directory": extra }),
        )
        .expect("store the extra folder");
    assert_eq!(
        settings["scripts_directory"],
        extra.to_string_lossy().as_ref()
    );
    assert_eq!(
        api.call("automation.settings_get", json!({})).unwrap(),
        settings
    );

    let listed = api
        .call("automation.scripts_list", json!({ "doc": created["doc"] }))
        .expect("list every searched folder");
    let kinds: Vec<&str> = listed["folders"]
        .as_array()
        .unwrap()
        .iter()
        .map(|folder| folder["kind"].as_str().unwrap())
        .collect();
    assert_eq!(kinds, vec!["configured", "preferences", "project"]);
    assert_eq!(
        listed["scripts"],
        json!([
            { "name": "shared.js", "folder": "configured", "language": "javascript", "bytes": 10 },
            { "name": "local.py", "folder": "project", "language": "python", "bytes": 8 },
        ])
    );
    assert_eq!(
        api.call(
            "automation.script_read",
            json!({ "name": "local.py", "folder": "project", "doc": created["doc"] }),
        )
        .unwrap(),
        json!({ "name": "local.py", "folder": "project", "language": "python", "source": "# local\n" })
    );
    // A name only present in another root is not readable from this one.
    assert!(api
        .call(
            "automation.script_read",
            json!({ "name": "local.py", "folder": "configured", "doc": created["doc"] }),
        )
        .is_err());
    // Without a document there is no project folder to resolve.
    assert!(api
        .call(
            "automation.script_read",
            json!({ "name": "local.py", "folder": "project" }),
        )
        .is_err());
}

#[test]
fn the_folder_fingerprint_changes_only_when_a_searched_folder_changes() {
    let (config, api) = configured_api();
    let scripts = config.path().join("scripts");
    fs::create_dir_all(&scripts).expect("scripts folder");
    fs::write(scripts.join("a.js"), "// one\n").expect("script");
    let first = api
        .call("automation.scripts_reload", json!({}))
        .expect("initial discovery");
    let again = api
        .call("automation.scripts_reload", json!({}))
        .expect("unchanged discovery");
    assert_eq!(first["fingerprint"], again["fingerprint"]);

    fs::write(scripts.join("b.py"), "# two\n").expect("new script");
    let changed = api
        .call("automation.scripts_reload", json!({}))
        .expect("changed discovery");
    assert_ne!(first["fingerprint"], changed["fingerprint"]);
    assert_eq!(changed["scripts"].as_array().unwrap().len(), 2);
}

#[test]
fn the_extra_scripts_folder_rejects_relative_and_missing_paths() {
    let (config, api) = configured_api();
    for value in [
        json!("relative/scripts"),
        json!(config.path().join("absent").to_string_lossy()),
    ] {
        assert!(api
            .call(
                "automation.settings_set",
                json!({ "scripts_directory": value })
            )
            .is_err());
    }
    // Clearing the preference is accepted and searched folders fall back.
    assert_eq!(
        api.call(
            "automation.settings_set",
            json!({ "scripts_directory": Value::Null }),
        )
        .unwrap()["scripts_directory"],
        Value::Null
    );
    let listed = api.call("automation.scripts_list", json!({})).unwrap();
    assert_eq!(listed["folders"].as_array().unwrap().len(), 1);
}

#[test]
fn an_abandoned_script_transaction_expires_and_a_new_run_supersedes_it() {
    let (_config, api) = configured_api();
    let created = api
        .call(
            "session.new",
            json!({ "family": "Abandoned", "units_per_em": 1000 }),
        )
        .expect("create document");
    let doc = created["doc"].clone();

    // A run that never closes its transaction, as a reloaded page leaves behind.
    let abandoned = api
        .call(
            "automation.transaction_begin",
            json!({ "doc": doc, "label": "Script: abandoned.js", "timeout_ms": 100 }),
        )
        .expect("open a short-lived transaction");
    api.call("glyphs.add", json!({ "doc": doc, "recipe": "A" }))
        .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(150));

    // The deadline has passed: later work is its own entry and history works.
    api.call("glyphs.add", json!({ "doc": doc, "recipe": "B" }))
        .unwrap();
    let summary = api.call("doc.summary", json!({ "doc": doc })).unwrap();
    assert_ne!(summary["undo_label"], "Script: abandoned.js");
    api.call("doc.undo", json!({ "doc": doc }))
        .expect("history is not blocked by an expired transaction");
    assert_eq!(
        api.call(
            "automation.transaction_end",
            json!({ "doc": doc, "token": abandoned["token"] }),
        )
        .unwrap()["ended"],
        false
    );
    api.call("doc.redo", json!({ "doc": doc })).unwrap();

    // A live transaction that the host loses is superseded by the next run.
    let lost = api
        .call(
            "automation.transaction_begin",
            json!({ "doc": doc, "label": "Script: lost.js" }),
        )
        .unwrap();
    api.call("glyphs.add", json!({ "doc": doc, "recipe": "C" }))
        .unwrap();
    let next = api
        .call(
            "automation.transaction_begin",
            json!({ "doc": doc, "label": "Script: next.js" }),
        )
        .expect("a new run supersedes the lost one");
    assert_ne!(next["token"], lost["token"]);
    assert_eq!(
        api.call(
            "automation.transaction_end",
            json!({ "doc": doc, "token": lost["token"] }),
        )
        .unwrap()["ended"],
        false
    );
    assert_eq!(
        api.call(
            "automation.transaction_end",
            json!({ "doc": doc, "token": next["token"] }),
        )
        .unwrap()["ended"],
        true
    );
    // The superseded run kept its own entry rather than corrupting history.
    api.call("doc.undo", json!({ "doc": doc })).unwrap();
    assert_eq!(
        api.call(
            "glyphs.list",
            json!({ "doc": doc, "offset": 0, "limit": 10, "include_paths": false }),
        )
        .unwrap()["total"],
        2
    );
}

#[test]
fn listing_scripts_reaps_only_expired_transactions() {
    let (_config, api) = configured_api();
    let created = api
        .call(
            "session.new",
            json!({ "family": "Listing", "units_per_em": 1000 }),
        )
        .expect("create document");
    let doc = created["doc"].clone();
    let live = api
        .call(
            "automation.transaction_begin",
            json!({ "doc": doc, "label": "Script: live.js" }),
        )
        .unwrap();

    // The console polls discovery every couple of seconds while a run is going.
    api.call("automation.scripts_reload", json!({})).unwrap();
    api.call("automation.scripts_list", json!({})).unwrap();

    assert_eq!(
        api.call(
            "automation.transaction_end",
            json!({ "doc": doc, "token": live["token"] }),
        )
        .unwrap()["ended"],
        true,
        "discovery must not close a live transaction"
    );
}

#[test]
fn a_script_transaction_lifetime_is_validated() {
    let (_config, api) = configured_api();
    let created = api
        .call(
            "session.new",
            json!({ "family": "Lifetime", "units_per_em": 1000 }),
        )
        .expect("create document");
    let doc = created["doc"].clone();
    for lifetime in [json!(0), json!(99), json!(120_001u32)] {
        assert!(api
            .call(
                "automation.transaction_begin",
                json!({ "doc": doc, "label": "Script: bounds.js", "timeout_ms": lifetime }),
            )
            .is_err());
    }
    assert!(api
        .call(
            "automation.transaction_begin",
            json!({ "doc": doc, "label": "Script: bounds.js", "timeout_ms": 120_000u32 }),
        )
        .is_ok());
}

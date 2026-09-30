//! Native workspace views share the application's existing API and event stream.

use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

struct Target {
    label: String,
    url: String,
    payload: Value,
    panel: bool,
}

impl Target {
    fn parse(value: Value) -> Result<Self, String> {
        let kind = value["kind"].as_str().ok_or("Missing window kind")?;
        let doc = value["doc"]
            .as_u64()
            .filter(|doc| *doc <= u64::from(u32::MAX))
            .ok_or("Invalid document identifier")?;
        let mut label = format!("fc-{kind}-{doc}");
        let mut url =
            tauri::Url::parse("http://localhost/index.html").map_err(|error| error.to_string())?;
        let mut payload = json!({ "kind": kind, "doc": doc });
        {
            let mut query = url.query_pairs_mut();
            query
                .append_pair("fcWindow", kind)
                .append_pair("doc", &doc.to_string());
            match kind {
                "document" => {}
                "tab" => {
                    let tab = value["tab"]
                        .as_str()
                        .filter(|tab| {
                            matches!(*tab, "font" | "glyph" | "fontinfo" | "features" | "bitmap")
                        })
                        .ok_or("Invalid tab kind")?;
                    label.push_str(&format!("-{tab}-"));
                    query.append_pair("tab", tab);
                    payload["tab"] = json!(tab);
                    if let Some(gid) = value.get("gid") {
                        let gid = gid
                            .as_u64()
                            .filter(|gid| *gid <= u64::from(u32::MAX))
                            .ok_or("Invalid glyph identifier")?;
                        if value.get("tabId").is_none() {
                            label.push_str(&gid.to_string());
                        }
                        query.append_pair("gid", &gid.to_string());
                        payload["gid"] = json!(gid);
                    } else if value.get("tabId").is_none() {
                        label.push_str("none");
                    }
                    if let Some(tab_id) = value.get("tabId") {
                        let tab_id = tab_id
                            .as_str()
                            .filter(|id| {
                                !id.is_empty()
                                    && id.len() <= 128
                                    && id.bytes().all(|byte| {
                                        byte.is_ascii_alphanumeric() || b"_:.-".contains(&byte)
                                    })
                            })
                            .ok_or("Invalid tab identifier")?;
                        label.push_str("id-");
                        for byte in tab_id.bytes() {
                            label.push_str(&format!("{byte:02x}"));
                        }
                        query.append_pair("tabId", tab_id);
                        payload["tabId"] = json!(tab_id);
                    }
                }
                "panel" => {
                    let panel = value["panel"]
                        .as_str()
                        .filter(|panel| matches!(*panel, "sidebar" | "inspector" | "preview"))
                        .ok_or("Invalid panel kind")?;
                    label.push_str(&format!("-{panel}"));
                    query.append_pair("panel", panel);
                    payload["panel"] = json!(panel);
                }
                _ => return Err("Invalid window kind".into()),
            }
        }
        Ok(Self {
            label,
            url: format!("index.html?{}", url.query().unwrap_or_default()),
            panel: kind == "panel",
            payload,
        })
    }
}

/// Opens or focuses a document view without creating another kernel session.
#[tauri::command]
pub(crate) async fn open_workspace_window(app: AppHandle, target: Value) -> Result<(), String> {
    let target = Target::parse(target)?;
    if let Some(window) = app.get_webview_window(&target.label) {
        window.unminimize().map_err(|error| error.to_string())?;
        window.show().map_err(|error| error.to_string())?;
        return window.set_focus().map_err(|error| error.to_string());
    }
    let (width, min_width) = if target.panel {
        (650.0, 420.0)
    } else {
        (1200.0, 900.0)
    };
    let window = WebviewWindowBuilder::new(&app, target.label, WebviewUrl::App(target.url.into()))
        .title("FontCarver")
        .inner_size(width, 850.0)
        .min_inner_size(min_width, 600.0)
        .build()
        .map_err(|error| error.to_string())?;
    window.on_window_event(move |event| {
        if matches!(event, tauri::WindowEvent::Destroyed) {
            if let Err(error) = app.emit("fc:window-closed", &target.payload) {
                tracing::warn!(%error, "failed to notify workspace about detached window close");
            }
        }
    });
    Ok(())
}

/// Closes an approved view, leaving kernel documents alive for the other views.
#[tauri::command]
pub(crate) async fn close_workspace_window(window: WebviewWindow) -> Result<(), String> {
    window.destroy().map_err(|error| error.to_string())
}

/// Restores a detached view in the main workspace before destroying its window.
#[tauri::command]
pub(crate) async fn rejoin_workspace_window(
    app: AppHandle,
    window: WebviewWindow,
    target: Value,
) -> Result<(), String> {
    let target = Target::parse(target)?;
    if window.label() != target.label {
        return Err("Workspace target does not match the requesting window".into());
    }
    let owner = app
        .get_webview_window(&format!("fc-document-{}", target.payload["doc"]))
        .filter(|owner| owner.label() != window.label())
        .or_else(|| app.get_webview_window("main"))
        .ok_or("The owning workspace is no longer open")?;
    owner.unminimize().map_err(|error| error.to_string())?;
    owner.show().map_err(|error| error.to_string())?;
    owner.set_focus().map_err(|error| error.to_string())?;
    app.emit("fc:window-rejoin", target.payload)
        .map_err(|error| error.to_string())?;
    window.destroy().map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_document_tab_and_preview_without_external_navigation() {
        let document = Target::parse(json!({ "kind": "document", "doc": 7 })).unwrap();
        assert_eq!(document.label, "fc-document-7");
        assert_eq!(document.url, "index.html?fcWindow=document&doc=7");
        let tab = Target::parse(json!({ "kind": "tab", "doc": 7, "tab": "glyph", "gid": 4,
            "tabId": "glyph:7:4:2" }))
        .unwrap();
        assert_eq!(tab.label, "fc-tab-7-glyph-id-676c7970683a373a343a32");
        assert_eq!(
            tab.url,
            "index.html?fcWindow=tab&doc=7&tab=glyph&gid=4&tabId=glyph%3A7%3A4%3A2"
        );
        let panel =
            Target::parse(json!({ "kind": "panel", "doc": 7, "panel": "preview" })).unwrap();
        assert_eq!(panel.label, "fc-panel-7-preview");
        assert!(panel.panel);
    }

    #[test]
    fn rejects_invalid_ids_kinds_and_navigation_injection() {
        for value in [
            json!({"kind": "document", "doc": -1}),
            json!({"kind": "document", "doc": 4_294_967_296_u64}),
            json!({"kind": "document", "doc": 1.5}),
            json!({"kind": "https://example.org", "doc": 1}),
            json!({"kind": "tab", "doc": 1, "tab": "font", "gid": -1}),
            json!({"kind": "tab", "doc": 1, "tab": "glyph", "tabId": "x&url=evil"}),
            json!({"kind": "tab", "doc": 1, "tab": "unknown"}),
            json!({"kind": "panel", "doc": 1, "panel": "unknown"}),
        ] {
            assert!(Target::parse(value.clone()).is_err(), "accepted {value}");
        }
    }

    #[test]
    fn distinct_text_tabs_get_distinct_window_names() {
        let first =
            Target::parse(json!({ "kind": "tab", "doc": 0, "tab": "glyph", "tabId": "text:1" }))
                .unwrap();
        let second =
            Target::parse(json!({ "kind": "tab", "doc": 0, "tab": "glyph", "tabId": "text:2" }))
                .unwrap();
        assert_ne!(first.label, second.label);
        assert!(first.label.starts_with("fc-tab-0-glyph-id-"));
    }
}

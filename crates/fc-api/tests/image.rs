use fc_api::Api;
use fc_doc::{DocId, Session};
use fc_model::{Font, Glyph, Layer};
use serde_json::{json, Value};
use std::sync::Arc;

fn fixture() -> (Api, Arc<Session>, DocId, Value) {
    let mut font = Font::new_empty("Image workflows", 1000);
    let mut glyph = Glyph::new("A");
    let layer = Layer::new_master(font.default_master().id.clone());
    let target = json!({"glyph":"A","layer":layer.id});
    glyph.layers.push(layer);
    font.insert_glyph(glyph).unwrap();
    let session = Arc::new(Session::new());
    let doc = session.open_font(font, None);
    let mut target = target;
    target["doc"] = json!(doc);
    (Api::new(session.clone()), session, doc, target)
}

// Authored 2x2 RGBA source: black top-left, opaque white elsewhere.
fn png() -> Vec<u8> {
    PNG.to_vec()
}

fn import(api: &Api, target: &Value) -> Value {
    let mut params = target.clone();
    params["expected_generation"] =
        api.call("image.state", target.clone()).unwrap()["generation"].clone();
    params["path"] = Value::Null;
    params["data"] = json!(png());
    params["file_name"] = json!("source.png");
    params["embed"] = json!(true);
    api.call("image.import", params).unwrap()
}

#[test]
fn embedded_image_transform_trace_preview_apply_undo_and_native_source() {
    let (api, session, doc, target) = fixture();
    assert_eq!(
        api.call("image.state", target.clone()).unwrap()["image"],
        Value::Null
    );
    import(&api, &target);
    let mut state = api.call("image.state", target.clone()).unwrap();
    assert_eq!(state["width"], 2);
    assert_eq!(state["height"], 2);
    assert_eq!(state["png"], json!(png()));
    state["image"]["transform"] = json!([2, 0, 0, 3, 10, 20]);
    let mut set = target.clone();
    set["expected_generation"] = state["generation"].clone();
    set["image"] = state["image"].clone();
    api.call("image.set", set).unwrap();
    let before = session.with_doc(doc, |d| d.font.clone()).unwrap();
    let mut params = target.clone();
    params["expected_generation"] = Value::Null;
    params["threshold"] = json!(128);
    params["tolerance"] = json!(0);
    params["dry_run"] = json!(true);
    params["append"] = json!(false);
    let preview = api.call("image.trace", params.clone()).unwrap();
    assert_eq!(preview["bounds"], json!([10.0, 23.0, 12.0, 26.0]));
    assert_eq!(session.with_doc(doc, |d| d.font.clone()).unwrap(), before);
    params["dry_run"] = json!(false);
    params["expected_generation"] = preview["generation"].clone();
    params["expected_source_hash"] = preview["source_hash"].clone();
    let applied = api.call("image.trace", params.clone()).unwrap();
    assert_eq!(preview["contours"], applied["contours"]);
    let after = session.with_doc(doc, |d| d.font.clone()).unwrap();
    assert_eq!(
        after.glyph("A").unwrap().layers[0].background_image,
        before.glyph("A").unwrap().layers[0].background_image
    );
    assert!(!after.glyph("A").unwrap().layers[0].contours.is_empty());
    assert!(api
        .call("image.trace", params)
        .unwrap_err()
        .message
        .contains("stale"));
    session.with_doc_mut(doc, |d| d.undo()).unwrap().unwrap();
    assert_eq!(session.with_doc(doc, |d| d.font.clone()).unwrap(), before);
    session.with_doc_mut(doc, |d| d.redo()).unwrap().unwrap();
    assert_eq!(session.with_doc(doc, |d| d.font.clone()).unwrap(), after);
    let encoded = serde_json::to_vec(&after).unwrap();
    let reopened: Font = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(reopened, after);
}

#[test]
fn lock_rejects_moving_replacing_and_removing_but_allows_explicit_unlock() {
    let (api, session, doc, target) = fixture();
    import(&api, &target);
    let state = api.call("image.state", target.clone()).unwrap();
    let mut set = target.clone();
    set["expected_generation"] = state["generation"].clone();
    set["image"] = state["image"].clone();
    set["image"]["locked"] = json!(true);
    api.call("image.set", set.clone()).unwrap();
    let locked = api.call("image.state", target.clone()).unwrap();
    set["expected_generation"] = locked["generation"].clone();
    set["image"]["transform"] = json!([1, 0, 0, 1, 100, 0]);
    assert!(api
        .call("image.set", set.clone())
        .unwrap_err()
        .message
        .contains("locked"));
    set["image"] = Value::Null;
    assert!(api.call("image.set", set.clone()).is_err());
    set["image"] = locked["image"].clone();
    set["image"]["locked"] = json!(false);
    api.call("image.set", set).unwrap();
    assert!(!session
        .with_doc(doc, |d| d.font.glyph("A").unwrap().layers[0]
            .background_image
            .as_ref()
            .unwrap()
            .locked)
        .unwrap());
}

#[test]
fn missing_links_invalid_png_and_stale_writes_are_reported_without_mutation() {
    let (api, session, doc, target) = fixture();
    let before = session.with_doc(doc, |d| d.font.clone()).unwrap();
    let mut params = target.clone();
    params["expected_generation"] = json!(0);
    params["path"] = Value::Null;
    params["data"] = json!([1, 2, 3]);
    params["file_name"] = json!("bad.png");
    params["embed"] = json!(true);
    assert!(api.call("image.import", params).is_err());
    assert_eq!(session.with_doc(doc, |d| d.font.clone()).unwrap(), before);
    session
        .with_doc_mut(doc, |d| {
            d.font.glyph_mut("A").unwrap().layers[0].background_image = Some(fc_model::Image {
                file_name: "missing.png".into(),
                ..Default::default()
            })
        })
        .unwrap();
    let state = api.call("image.state", target).unwrap();
    assert!(state["error"].as_str().is_some());
    assert_eq!(state["png"], Value::Null);
}

const PNG: &[u8] = &[
    137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 2, 0, 0, 0, 2, 8, 6, 0,
    0, 0, 114, 182, 13, 36, 0, 0, 0, 17, 73, 68, 65, 84, 120, 156, 99, 96, 96, 96, 248, 15, 2, 16,
    18, 8, 0, 95, 178, 12, 244, 216, 82, 206, 121, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
];

#[test]
fn relative_png_links_report_breakage_and_reject_changed_preview_sources() {
    let (api, session, doc, target) = fixture();
    let dir = tempfile::tempdir().unwrap();
    let document = dir.path().join("font.fcf");
    let source = dir.path().join("source.png");
    std::fs::write(&source, png()).unwrap();
    session
        .with_doc_mut(doc, |d| d.save(&document))
        .unwrap()
        .unwrap();
    let mut params = target.clone();
    params["expected_generation"] =
        api.call("image.state", target.clone()).unwrap()["generation"].clone();
    params["path"] = json!(source);
    params["data"] = Value::Null;
    params["file_name"] = json!("");
    params["embed"] = json!(false);
    api.call("image.import", params).unwrap();
    let state = api.call("image.state", target.clone()).unwrap();
    assert_eq!(state["image"]["file_name"], "source.png");
    assert_eq!(state["image"]["data"], Value::Null);
    let before = session.with_doc(doc, |d| d.font.clone()).unwrap();
    let mut trace = target.clone();
    trace["threshold"] = json!(128);
    trace["tolerance"] = json!(0);
    trace["append"] = json!(false);
    trace["dry_run"] = json!(true);
    trace["expected_generation"] = Value::Null;
    let preview = api.call("image.trace", trace.clone()).unwrap();
    std::fs::write(&source, WHITE_PNG).unwrap();
    trace["dry_run"] = json!(false);
    trace["expected_generation"] = preview["generation"].clone();
    trace["expected_source_hash"] = preview["source_hash"].clone();
    assert!(api
        .call("image.trace", trace)
        .unwrap_err()
        .message
        .contains("source changed"));
    assert_eq!(session.with_doc(doc, |d| d.font.clone()).unwrap(), before);
    std::fs::remove_file(&source).unwrap();
    let broken = api.call("image.state", target).unwrap();
    assert!(broken["error"].is_string());
    assert_eq!(broken["image"], state["image"]);
    assert_eq!(broken["png"], Value::Null);
}

#[test]
fn invalid_placement_and_glyph_locks_are_atomic() {
    let (api, session, doc, target) = fixture();
    import(&api, &target);
    let state = api.call("image.state", target.clone()).unwrap();
    let original = session.with_doc(doc, |d| d.font.clone()).unwrap();
    let mut set = target;
    set["expected_generation"] = state["generation"].clone();
    set["image"] = state["image"].clone();
    set["image"]["transform"] = json!([0, 0, 0, 0, 0, 0]);
    assert!(api.call("image.set", set.clone()).is_err());
    assert_eq!(session.with_doc(doc, |d| d.font.clone()).unwrap(), original);
    session
        .with_doc_mut(doc, |d| d.font.glyph_mut("A").unwrap().locked = true)
        .unwrap();
    set["image"] = Value::Null;
    assert!(api
        .call("image.set", set)
        .unwrap_err()
        .message
        .contains("locked"));
}

const WHITE_PNG: &[u8] = &[
    137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 2, 0, 0, 0, 2, 8, 6, 0,
    0, 0, 114, 182, 13, 36, 0, 0, 0, 14, 73, 68, 65, 84, 120, 156, 99, 248, 15, 5, 12, 48, 6, 0,
    143, 130, 15, 241, 60, 165, 86, 81, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
];

#[test]
fn a_corrupt_embedded_image_can_be_unlocked_and_removed_for_repair() {
    let (api, session, doc, target) = fixture();
    session
        .with_doc_mut(doc, |d| {
            d.font.glyph_mut("A").unwrap().layers[0].background_image = Some(fc_model::Image {
                file_name: "broken.png".into(),
                locked: true,
                data: Some(vec![1, 2, 3]),
                ..Default::default()
            });
        })
        .unwrap();
    let state = api.call("image.state", target.clone()).unwrap();
    assert!(state["error"].is_string());
    let mut params = target.clone();
    params["expected_generation"] = state["generation"].clone();
    params["image"] = state["image"].clone();
    params["image"]["locked"] = json!(false);
    api.call("image.set", params.clone()).unwrap();
    params["expected_generation"] = api.call("image.state", target).unwrap()["generation"].clone();
    params["image"] = Value::Null;
    api.call("image.set", params).unwrap();
    assert!(session
        .with_doc(doc, |d| d.font.glyph("A").unwrap().layers[0]
            .background_image
            .is_none())
        .unwrap());
}

/// Minimal 8-bit grayscale PNG writer, so these tests need no image encoder.
///
/// The zlib stream uses stored deflate blocks, which keeps the whole helper to
/// CRC-32 and Adler-32 and makes the bytes independent of any compressor.
fn grayscale_png(width: u32, height: u32, pixels: &[u8]) -> Vec<u8> {
    fn crc32(bytes: &[u8]) -> u32 {
        let mut crc = 0xffff_ffffu32;
        for &byte in bytes {
            crc ^= u32::from(byte);
            for _ in 0..8 {
                crc = if crc & 1 == 1 {
                    (crc >> 1) ^ 0xedb8_8320
                } else {
                    crc >> 1
                };
            }
        }
        !crc
    }
    fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
        out.extend((data.len() as u32).to_be_bytes());
        let mut body = kind.to_vec();
        body.extend_from_slice(data);
        out.extend(&body);
        out.extend(crc32(&body).to_be_bytes());
    }
    assert_eq!(pixels.len(), (width * height) as usize);
    let mut raw = Vec::with_capacity(((width + 1) * height) as usize);
    for row in 0..height {
        raw.push(0);
        let start = (row * width) as usize;
        raw.extend_from_slice(&pixels[start..start + width as usize]);
    }
    let mut zlib = vec![0x78, 0x01];
    for (index, block) in raw.chunks(0xffff).enumerate() {
        let last = (index + 1) * 0xffff >= raw.len();
        zlib.push(u8::from(last));
        zlib.extend((block.len() as u16).to_le_bytes());
        zlib.extend((!(block.len() as u16)).to_le_bytes());
        zlib.extend_from_slice(block);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for &byte in &raw {
        a = (a + u32::from(byte)) % 65521;
        b = (b + a) % 65521;
    }
    zlib.extend(((b << 16) | a).to_be_bytes());
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut header = Vec::new();
    header.extend(width.to_be_bytes());
    header.extend(height.to_be_bytes());
    header.extend([8, 0, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &header);
    chunk(&mut out, b"IDAT", &zlib);
    chunk(&mut out, b"IEND", &[]);
    out
}

/// A filled disc, plus an optional two-pixel speck in the corner.
fn disc_png(size: u32, radius: f64, speck: bool) -> Vec<u8> {
    let centre = f64::from(size) / 2.;
    let pixels: Vec<u8> = (0..size)
        .flat_map(|y| (0..size).map(move |x| (x, y)))
        .map(|(x, y)| {
            let inside = (f64::from(x) + 0.5 - centre).hypot(f64::from(y) + 0.5 - centre) <= radius;
            let dust = speck && x < 2 && y < 2;
            if inside || dust {
                0
            } else {
                255
            }
        })
        .collect();
    grayscale_png(size, size, &pixels)
}

fn import_bytes(api: &Api, target: &Value, bytes: Vec<u8>) {
    let mut params = target.clone();
    params["expected_generation"] =
        api.call("image.state", target.clone()).unwrap()["generation"].clone();
    params["path"] = Value::Null;
    params["data"] = json!(bytes);
    params["file_name"] = json!("disc.png");
    params["embed"] = json!(true);
    api.call("image.import", params).unwrap();
}

fn trace_params(target: &Value) -> Value {
    let mut params = target.clone();
    params["expected_generation"] = Value::Null;
    params["expected_source_hash"] = Value::Null;
    params["threshold"] = json!(128);
    params["tolerance"] = json!(1.0);
    params["curves"] = json!(true);
    params["corner_angle"] = json!(35.0);
    params["smoothing"] = json!(0.0);
    params["min_area"] = json!(0.0);
    params["dry_run"] = json!(true);
    params["append"] = json!(false);
    params
}

#[test]
fn curve_autotrace_reports_its_quality_and_writes_curve_nodes() {
    let (api, session, doc, target) = fixture();
    import_bytes(&api, &target, disc_png(200, 80., false));
    let mut params = trace_params(&target);
    let curved = api.call("image.trace", params.clone()).unwrap();
    params["curves"] = json!(false);
    let polygon = api.call("image.trace", params.clone()).unwrap();
    assert!(
        curved["nodes"].as_u64().unwrap() < polygon["nodes"].as_u64().unwrap(),
        "curves {} against lines {}",
        curved["nodes"],
        polygon["nodes"]
    );
    assert!(curved["curve_segments"].as_u64().unwrap() > 0);
    assert_eq!(polygon["curve_segments"], json!(0));
    assert_eq!(curved["dropped"], json!(0));
    assert_eq!(curved["fallback"], json!(false));
    assert!(
        curved["max_error"].as_f64().unwrap() > 0. && curved["max_error"].as_f64().unwrap() < 3.,
        "deviation {}",
        curved["max_error"]
    );
    let before = session.with_doc(doc, |d| d.font.clone()).unwrap();
    params["curves"] = json!(true);
    params["dry_run"] = json!(false);
    params["expected_generation"] = curved["generation"].clone();
    params["expected_source_hash"] = curved["source_hash"].clone();
    let applied = api.call("image.trace", params).unwrap();
    assert_eq!(applied["contours"], curved["contours"]);
    let after = session.with_doc(doc, |d| d.font.clone()).unwrap();
    let layer = &after.glyph("A").unwrap().layers[0];
    assert_eq!(layer.contours.len(), 1);
    assert!(layer.contours[0]
        .points
        .iter()
        .any(|point| point.kind == fc_model::PointKind::OffCurve));
    assert!(layer.contours[0].closed);
    assert_eq!(
        layer.background_image,
        before.glyph("A").unwrap().layers[0].background_image,
        "the source image survives the trace"
    );
    session.with_doc_mut(doc, |d| d.undo()).unwrap().unwrap();
    assert_eq!(session.with_doc(doc, |d| d.font.clone()).unwrap(), before);
}

#[test]
fn the_minimum_area_filter_and_invalid_options_are_enforced_by_the_kernel() {
    let (api, session, doc, target) = fixture();
    import_bytes(&api, &target, disc_png(120, 40., true));
    let mut params = trace_params(&target);
    let all = api.call("image.trace", params.clone()).unwrap();
    assert_eq!(all["contours"].as_array().unwrap().len(), 2);
    assert_eq!(all["dropped"], json!(0));
    params["min_area"] = json!(16.0);
    let filtered = api.call("image.trace", params.clone()).unwrap();
    assert_eq!(filtered["contours"].as_array().unwrap().len(), 1);
    assert_eq!(filtered["dropped"], json!(1));
    let before = session.with_doc(doc, |d| d.font.clone()).unwrap();
    for (field, value) in [
        ("tolerance", json!(17.0)),
        ("corner_angle", json!(-1.0)),
        ("corner_angle", json!(181.0)),
        ("smoothing", json!(9.0)),
        ("min_area", json!(-1.0)),
        ("threshold", json!(0)),
    ] {
        let mut invalid = trace_params(&target);
        invalid[field] = value.clone();
        assert!(
            api.call("image.trace", invalid.clone()).is_err(),
            "{field} {value} was accepted"
        );
        invalid["dry_run"] = json!(false);
        invalid["expected_generation"] = all["generation"].clone();
        invalid["expected_source_hash"] = all["source_hash"].clone();
        assert!(api.call("image.trace", invalid).is_err());
        assert_eq!(session.with_doc(doc, |d| d.font.clone()).unwrap(), before);
    }
}

#[test]
fn a_linked_source_reports_the_path_it_resolved_for_a_relink_prompt() {
    let (api, session, doc, target) = fixture();
    let dir = tempfile::tempdir().unwrap();
    let document = dir.path().join("font.fcf");
    let source = dir.path().join("linked.png");
    std::fs::write(&source, png()).unwrap();
    session
        .with_doc_mut(doc, |d| d.save(&document))
        .unwrap()
        .unwrap();
    import_bytes(&api, &target, png());
    assert_eq!(
        api.call("image.state", target.clone()).unwrap()["source_path"],
        Value::Null,
        "an embedded source has no path to relink"
    );
    let mut params = target.clone();
    params["expected_generation"] =
        api.call("image.state", target.clone()).unwrap()["generation"].clone();
    params["path"] = json!(source);
    params["data"] = Value::Null;
    params["file_name"] = json!("");
    params["embed"] = json!(false);
    api.call("image.import", params).unwrap();
    let linked = api.call("image.state", target.clone()).unwrap();
    assert_eq!(
        linked["source_path"].as_str().unwrap(),
        source.canonicalize().unwrap().to_str().unwrap()
    );
    assert!(linked["error"].is_null());
    std::fs::remove_file(&source).unwrap();
    let broken = api.call("image.state", target.clone()).unwrap();
    assert!(broken["error"].is_string(), "a broken link is reported");
    assert_eq!(broken["source_path"], linked["source_path"]);
    assert_eq!(broken["image"], linked["image"]);
    // Relinking is the ordinary import, which repairs the layer in one command.
    std::fs::write(&source, png()).unwrap();
    let mut relink = target.clone();
    relink["expected_generation"] = broken["generation"].clone();
    relink["path"] = json!(source);
    relink["data"] = Value::Null;
    relink["file_name"] = json!("");
    relink["embed"] = json!(false);
    api.call("image.import", relink).unwrap();
    let repaired = api.call("image.state", target).unwrap();
    assert!(repaired["error"].is_null());
    assert_eq!(repaired["width"], json!(2));
}

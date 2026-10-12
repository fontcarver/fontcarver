//! Static OpenType SVG subset with bounded parsing and exact attribute rewrites.
use super::MAX_SVG_DOCUMENT_BYTES;
use roxmltree::{Document, ParsingOptions};
use std::collections::{BTreeMap, BTreeSet, HashMap};

const SVG_NS: &str = "http://www.w3.org/2000/svg";
const XLINK_NS: &str = "http://www.w3.org/1999/xlink";

fn parse(source: &str) -> Result<Document<'_>, String> {
    if source.len() > MAX_SVG_DOCUMENT_BYTES {
        return Err("SVG document exceeds 2 MiB".into());
    }
    Document::parse_with_options(
        source,
        ParsingOptions {
            allow_dtd: false,
            nodes_limit: 100_000,
            ..Default::default()
        },
    )
    .map_err(|error| format!("SVG XML: {error}"))
}

/// Validate the editable static SVG subset, returning its unique element IDs.
/// Stylesheets, scripts, foreign content and external resources remain opaque on import.
pub fn validate_xml(source: &str) -> Result<BTreeSet<String>, String> {
    let doc = parse(source)?;
    if !doc.root_element().has_tag_name((SVG_NS, "svg")) {
        return Err("SVG needs an svg root in the SVG namespace".into());
    }
    let mut ids = BTreeSet::new();
    let mut references = Vec::new();
    let mut id_nodes = HashMap::new();
    for node in doc.descendants() {
        if node.is_pi() {
            return Err("SVG processing instructions are not editable".into());
        }
        if !node.is_element() {
            continue;
        }
        if node.ancestors().take(130).count() > 128 {
            return Err("SVG nesting exceeds 128 elements".into());
        }
        let tag = node.tag_name();
        if tag.namespace() != Some(SVG_NS)
            || !matches!(
                tag.name(),
                "svg"
                    | "g"
                    | "defs"
                    | "path"
                    | "rect"
                    | "circle"
                    | "ellipse"
                    | "line"
                    | "polyline"
                    | "polygon"
                    | "use"
                    | "linearGradient"
                    | "radialGradient"
                    | "stop"
                    | "clipPath"
                    | "mask"
                    | "pattern"
                    | "title"
                    | "desc"
            )
        {
            return Err(format!(
                "SVG element {} is outside the editable static subset",
                tag.name()
            ));
        }
        for attr in node.attributes() {
            let name = attr.name();
            let value = attr.value();
            if attr
                .namespace()
                .is_some_and(|ns| ns != XLINK_NS || name != "href")
                || name.to_ascii_lowercase().starts_with("on")
            {
                return Err(format!("SVG attribute {name} is not editable"));
            }
            if name == "id" {
                if !valid_id(value) || !ids.insert(value.to_owned()) {
                    return Err("SVG element IDs must be unique simple identifiers".into());
                }
                id_nodes.insert(value, node.id());
            } else if name == "href" {
                let id = value
                    .strip_prefix('#')
                    .filter(|id| valid_id(id))
                    .ok_or("SVG references must be local element IDs")?;
                references.push((node.id(), id.to_owned()));
            } else {
                // This subset has no CSS escapes, comments, declarations or animation.
                // That makes every URL reference explicit and safely rewritable.
                if value.contains(['\\', '@'])
                    || value.contains("/*")
                    || (name == "style" && value.to_ascii_lowercase().contains("animation"))
                {
                    return Err("SVG escaped CSS, imports and animations are not editable".into());
                }
                rewrite_urls(value, |id| {
                    references.push((node.id(), id.to_owned()));
                    Ok(id.to_owned())
                })?;
            }
        }
    }
    if let Some((_, id)) = references.iter().find(|(_, id)| !ids.contains(id)) {
        return Err(format!("SVG reference #{id} does not exist"));
    }
    check_expansion(&doc, &id_nodes, &references)?;
    Ok(ids)
}

fn check_expansion(
    doc: &Document<'_>,
    ids: &HashMap<&str, roxmltree::NodeId>,
    references: &[(roxmltree::NodeId, String)],
) -> Result<(), String> {
    let nodes: Vec<_> = doc.descendants().filter(|n| n.is_element()).collect();
    let indices: HashMap<_, _> = nodes.iter().enumerate().map(|(i, n)| (n.id(), i)).collect();
    let mut parents = vec![Vec::new(); nodes.len()];
    let mut remaining = vec![0usize; nodes.len()];
    for (i, node) in nodes.iter().enumerate() {
        for child in node.children().filter(|n| n.is_element()) {
            parents[indices[&child.id()]].push(i);
            remaining[i] += 1;
        }
    }
    for (node, id) in references {
        let target = ids.get(id.as_str()).ok_or("SVG missing reference")?;
        parents[indices[target]].push(indices[node]);
        remaining[indices[node]] += 1;
    }
    let mut ready: Vec<_> = (0..nodes.len()).filter(|i| remaining[*i] == 0).collect();
    let mut size = vec![1usize; nodes.len()];
    let mut visited = 0;
    while let Some(i) = ready.pop() {
        visited += 1;
        for parent in &parents[i] {
            size[*parent] = size[*parent].saturating_add(size[i]);
            if size[*parent] > 1_000_000 {
                return Err("SVG reference expansion exceeds 1000000 elements".into());
            }
            remaining[*parent] -= 1;
            if remaining[*parent] == 0 {
                ready.push(*parent);
            }
        }
    }
    if visited != nodes.len() {
        return Err("SVG cyclic element references are not editable".into());
    }
    Ok(())
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_.:-".contains(&b))
}

fn rewrite_urls(
    value: &str,
    mut map: impl FnMut(&str) -> Result<String, String>,
) -> Result<String, String> {
    let lower = value.to_ascii_lowercase();
    let mut result = String::new();
    let mut cursor = 0;
    while let Some(relative) = lower[cursor..].find("url(") {
        let start = cursor + relative;
        let end = value[start + 4..]
            .find(')')
            .map(|i| start + 4 + i)
            .ok_or("SVG URL is unterminated")?;
        let target = value[start + 4..end].trim();
        let target = if target.starts_with(['\'', '"']) {
            let quote = target.as_bytes()[0] as char;
            target
                .strip_prefix(quote)
                .and_then(|s| s.strip_suffix(quote))
                .ok_or("SVG URL quotes are unbalanced")?
        } else {
            target
        };
        let id = target
            .strip_prefix('#')
            .filter(|id| valid_id(id))
            .ok_or("SVG URLs must reference local element IDs")?;
        result.push_str(&value[cursor..start]);
        result.push_str(&format!("url(#{})", map(id)?));
        cursor = end + 1;
    }
    result.push_str(&value[cursor..]);
    Ok(result)
}

/// Rename all element IDs and local references, retaining source geometry and colors.
/// The map must cover every ID and contain unique valid replacements.
pub fn rewrite_ids(source: &str, mapping: &BTreeMap<String, String>) -> Result<String, String> {
    let ids = validate_xml(source)?;
    let values: BTreeSet<_> = mapping.values().collect();
    if mapping.len() != ids.len()
        || values.len() != ids.len()
        || !ids.iter().all(|id| mapping.contains_key(id))
        || !values.iter().all(|id| valid_id(id))
    {
        return Err("SVG identifier remapping is incomplete or ambiguous".into());
    }
    let doc = parse(source)?;
    let mapped = |id: &str| {
        mapping
            .get(id)
            .cloned()
            .ok_or_else(|| format!("SVG missing #{id}"))
    };
    let mut replacements = Vec::new();
    for node in doc.descendants().filter(|node| node.is_element()) {
        for attr in node.attributes() {
            let value = match attr.name() {
                "id" => mapped(attr.value())?,
                "href" => format!("#{}", mapped(&attr.value()[1..])?),
                _ => rewrite_urls(attr.value(), mapped)?,
            };
            if value == attr.value() {
                continue;
            }
            // Find the quoted span from the full range; range_value has parser
            // storage limits for unusually long whitespace around '='.
            let range = attr.range();
            let raw = &source[range.clone()];
            let start = raw.find(['\'', '"']).ok_or("SVG missing attribute quote")?;
            replacements.push((range.start + start + 1..range.end - 1, escape(&value)));
        }
    }
    let mut result = String::with_capacity(source.len());
    let mut cursor = 0;
    for (range, value) in replacements {
        result.push_str(&source[cursor..range.start]);
        result.push_str(&value);
        cursor = range.end;
    }
    result.push_str(&source[cursor..]);
    Ok(result)
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

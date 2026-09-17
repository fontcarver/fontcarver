//! Intern repeated glyph sets to keep complete imported source editable.
use std::collections::BTreeMap;

pub(super) fn compact_classes(source: &str) -> String {
    let mut classes = BTreeMap::<&str, usize>::new();
    let mut remaining = source;
    while let Some(start) = remaining.find('[') {
        remaining = &remaining[start..];
        let Some(end) = remaining.find(']') else {
            return source.into();
        };
        let class = &remaining[..=end];
        if class.len() > 32 {
            *classes.entry(class).or_default() += 1;
        }
        remaining = &remaining[end + 1..];
    }
    let classes: BTreeMap<_, _> = classes
        .into_iter()
        .filter(|(_, count)| *count > 1)
        .enumerate()
        .map(|(id, (class, _))| (class, format!("@FC_SET_{id:05}")))
        .collect();
    if classes.is_empty() {
        return source.into();
    }
    let mut result = String::new();
    for (class, name) in &classes {
        result.push_str(&format!("{name} = {class};\n"));
    }
    let mut remaining = source;
    while let Some(start) = remaining.find('[') {
        result.push_str(&remaining[..start]);
        remaining = &remaining[start..];
        let Some(end) = remaining.find(']') else {
            break;
        };
        let class = &remaining[..=end];
        result.push_str(classes.get(class).map_or(class, String::as_str));
        remaining = &remaining[end + 1..];
    }
    result.push_str(remaining);
    result
}

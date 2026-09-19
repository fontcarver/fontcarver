//! Source-shadow fields must never override modeled or behavioral native data.

use super::error;
use crate::Error;

pub(super) fn safe(key: &str) -> Result<(), Error> {
    if matches!(
        key,
        "HheadAscent"
            | "HheadDescent"
            | "HheadAOffset"
            | "HheadDOffset"
            | "OS2TypoAscent"
            | "OS2TypoDescent"
            | "OS2TypoAOffset"
            | "OS2TypoDOffset"
            | "OS2TypoLinegap"
            | "OS2TypoLineGap"
            | "OS2WinAscent"
            | "OS2WinDescent"
            | "OS2WinAOffset"
            | "OS2WinDOffset"
            | "OS2CapHeight"
            | "OS2XHeight"
            | "TTFWeight"
            | "TTFWidth"
            | "UnderlinePosition"
            | "UnderlineWidth"
            | "OS2Vendor"
            | "LineGap"
    ) {
        return Err(error(format!(
            "modeled SFD field {key} cannot be stored in a source shadow"
        )));
    }
    if matches!(
        key,
        "SplineFontDB"
            | "StartChar"
            | "EndChar"
            | "BeginChars"
            | "EndChars"
            | "EndSplineFont"
            | "FontName"
            | "FullName"
            | "FamilyName"
            | "Weight"
            | "Copyright"
            | "Version"
            | "UComments"
            | "Comments"
            | "Ascent"
            | "Descent"
            | "ItalicAngle"
            | "UFOAscent"
            | "UFODescent"
            | "Encoding"
            | "Order2"
            | "Layer"
            | "LayerCount"
            | "Width"
            | "VWidth"
            | "Refer"
            | "Ref"
            | "AltUni2"
            | "Comment"
            | "HStem"
            | "VStem"
            | "Flags"
            | "GlyphClass"
            | "Lookup"
            | "BeginPrivate"
            | "EndPrivate"
            | "LangName"
    ) {
        return Err(error(format!(
            "modeled SFD field {key} cannot be stored in a source shadow"
        )));
    }
    if [
        "MM",
        "CID",
        "BeginSub",
        "BeginMM",
        "Registry",
        "Ordering",
        "Supplement",
        "Bitmap",
        "BDF",
        "Context",
        "Chain",
        "Reverse",
        "MacContext",
        "MacInsert",
        "MacKern",
        "MarkAttach",
        "Tt",
        "ShortTable",
        "Image",
        "Fill",
        "Stroke",
        "PathFlags",
        "Spiro",
        "InSpiro",
        "Grid",
        "Kern",
        "VKern",
        "Ligature",
        "Substitution",
        "AlternateSubs",
        "MultipleSubs",
        "Position",
        "PairPos",
        "Anchor",
        "ScriptLang",
        "Justify",
        "Jstf",
        "Base",
        "MATH",
        "GlyphVariants",
        "GlyphConstruction",
        "TopRight",
        "TopLeft",
        "BottomRight",
        "BottomLeft",
        "DStem",
        "CounterMask",
        "Pickled",
        "UnlinkRm",
        "Multilayer",
        "OnlyBitmaps",
        "LCarets",
    ]
    .iter()
    .any(|prefix| key.starts_with(prefix))
    {
        return Err(error(format!(
            "unsupported output-affecting SFD field {key}"
        )));
    }
    Ok(())
}

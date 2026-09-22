use fc_glyphdata::{
    block_name, category, is_cjk_ideograph, name_for_unicode, script, sub_category,
    unicode_for_name, Category, CjkCharsets,
};

#[test]
fn resolves_aglfn_and_production_names() {
    assert_eq!(unicode_for_name("A"), Some(0x41));
    assert_eq!(unicode_for_name("uni4E00"), Some(0x4E00));
    assert_eq!(unicode_for_name("u1F600"), Some(0x1F600));
    assert_eq!(unicode_for_name("a.sc"), Some(0x61));
    assert_eq!(unicode_for_name("f_i"), None);

    assert_eq!(name_for_unicode(0x4E00), "uni4E00");
    assert_eq!(name_for_unicode(0x41), "A");
    assert_eq!(name_for_unicode(0x1F600), "u1F600");
}

#[test]
fn rejects_malformed_or_non_scalar_production_names() {
    assert_eq!(unicode_for_name("uni4e00"), Some(0x4E00));
    assert_eq!(unicode_for_name("u1f600.alt"), Some(0x1F600));
    assert_eq!(unicode_for_name("uni041"), None);
    assert_eq!(unicode_for_name("uni00410"), None);
    assert_eq!(unicode_for_name("uni110000"), None);
    assert_eq!(unicode_for_name("u123"), None);
    assert_eq!(unicode_for_name("u1234567"), None);
    assert_eq!(unicode_for_name("u110000"), None);
    assert_eq!(unicode_for_name("uniD800"), None);
    assert_eq!(unicode_for_name("f_i.alt"), None);
}

#[test]
fn reports_unicode_properties() {
    assert_eq!(category(0x41), Category::Letter);
    assert_eq!(sub_category(0x41), "Uppercase");
    assert_eq!(block_name(0x4E00), "CJK Unified Ideographs");
    assert_eq!(script(0x4E00), "Hani");
    assert!(is_cjk_ideograph(0x4E00));
}

#[test]
fn maps_every_documented_subcategory_bucket() {
    let cases = [
        (0x0041, "Uppercase"),
        (0x0061, "Lowercase"),
        (0x4E00, "Ideograph"),
        (0x3042, "Kana"),
        (0xAC00, "Hangul"),
        (0x02B0, "Modifier"),
        (0x05D0, "Other"),
        (0x0030, "Decimal"),
        (0x00BD, "Fraction"),
        (0x2160, "Other"),
        (0x2010, "Dash"),
        (0x0022, "Quote"),
        (0x0028, "Parenthesis"),
        (0x0021, "Other"),
        (0x0024, "Currency"),
        (0x002B, "Math"),
        (0x2192, "Arrow"),
        (0x00A9, "Other"),
        (0x0020, "Space"),
        (0x0301, "Nonspacing"),
        (0x0903, "Spacing"),
        (0x20DD, "Enclosing"),
    ];

    for (cp, expected) in cases {
        assert_eq!(sub_category(cp), expected, "U+{cp:04X}");
    }

    assert_eq!(category(0x200D), Category::Format);
    assert_eq!(category(0xE000), Category::Private);
    assert_eq!(category(0x0000), Category::Other);
}

#[test]
fn exposes_sidebar_property_values() {
    assert!(fc_glyphdata::all_block_names().contains(&"CJK Unified Ideographs"));
    assert!(fc_glyphdata::all_scripts().contains(&"Hani"));
    assert!(fc_glyphdata::all_scripts().contains(&"Zyyy"));
}

#[test]
fn reports_cjk_charset_membership() {
    let ideograph = fc_glyphdata::cjk_charset_membership(0x4E00);
    assert!(ideograph.contains(CjkCharsets::GB2312));
    assert!(ideograph.contains(CjkCharsets::BIG5));
    assert!(ideograph.contains(CjkCharsets::GB18030_LEVEL1));
    assert!(ideograph.contains(CjkCharsets::JIS_X_0208));
    assert!(ideograph.contains(CjkCharsets::KS_X_1001));

    let simplified = fc_glyphdata::cjk_charset_membership(0x56FD);
    assert!(simplified.contains(CjkCharsets::GB2312));
    assert!(!simplified.contains(CjkCharsets::BIG5));

    let traditional = fc_glyphdata::cjk_charset_membership(0x570B);
    assert!(traditional.contains(CjkCharsets::BIG5));
    assert!(!traditional.contains(CjkCharsets::GB2312));
}

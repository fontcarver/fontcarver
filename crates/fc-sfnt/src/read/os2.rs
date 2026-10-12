use fc_model::FontInfo;
use read_fonts::{tables::os2::Os2, FontRef, TableProvider};

use crate::Error;

pub(crate) fn apply(face: &FontRef<'_>, info: &mut FontInfo) -> Result<(), Error> {
    let Some(os2) = table(face)? else {
        return Ok(());
    };
    info.open_type_os2_vendor_id = Some(
        String::from_utf8_lossy(&os2.ach_vend_id().to_be_bytes())
            .trim_end_matches('\0')
            .to_owned(),
    );
    info.open_type_os2_weight_class = Some(os2.us_weight_class());
    info.open_type_os2_width_class = Some(os2.us_width_class());
    info.open_type_os2_selection = bits(u32::from(os2.fs_selection().bits()), 16);
    info.open_type_os2_type = bits(u32::from(os2.fs_type()), 16);
    info.open_type_os2_panose = os2.panose_10().try_into().ok();
    let family = os2.s_family_class().to_be_bytes();
    info.open_type_os2_family_class = Some((family[0], family[1]));
    let unicode_bits = u128::from(os2.ul_unicode_range_1())
        | (u128::from(os2.ul_unicode_range_2()) << 32)
        | (u128::from(os2.ul_unicode_range_3()) << 64)
        | (u128::from(os2.ul_unicode_range_4()) << 96);
    info.open_type_os2_unicode_ranges = bits128(unicode_bits);
    let code_page_bits = u64::from(os2.ul_code_page_range_1().unwrap_or(0))
        | (u64::from(os2.ul_code_page_range_2().unwrap_or(0)) << 32);
    info.open_type_os2_code_page_ranges = bits64(code_page_bits);
    info.open_type_os2_subscript_x_size = Some(f64::from(os2.y_subscript_x_size()));
    info.open_type_os2_subscript_y_size = Some(f64::from(os2.y_subscript_y_size()));
    info.open_type_os2_subscript_x_offset = Some(f64::from(os2.y_subscript_x_offset()));
    info.open_type_os2_subscript_y_offset = Some(f64::from(os2.y_subscript_y_offset()));
    info.open_type_os2_superscript_x_size = Some(f64::from(os2.y_superscript_x_size()));
    info.open_type_os2_superscript_y_size = Some(f64::from(os2.y_superscript_y_size()));
    info.open_type_os2_superscript_x_offset = Some(f64::from(os2.y_superscript_x_offset()));
    info.open_type_os2_superscript_y_offset = Some(f64::from(os2.y_superscript_y_offset()));
    info.open_type_os2_strikeout_size = Some(f64::from(os2.y_strikeout_size()));
    info.open_type_os2_strikeout_position = Some(f64::from(os2.y_strikeout_position()));
    Ok(())
}

pub(crate) fn table<'a>(face: &FontRef<'a>) -> Result<Option<Os2<'a>>, Error> {
    if face
        .table_data(read_fonts::types::Tag::new(b"OS/2"))
        .is_none()
    {
        return Ok(None);
    }
    face.os2()
        .map(Some)
        .map_err(|error| Error::parse("OS/2", error))
}

fn bits(value: u32, count: u8) -> Vec<u8> {
    (0..count).filter(|bit| value & (1 << bit) != 0).collect()
}

fn bits64(value: u64) -> Vec<u8> {
    (0..64)
        .filter(|bit| value & (1_u64 << bit) != 0)
        .map(|bit| bit as u8)
        .collect()
}

fn bits128(value: u128) -> Vec<u8> {
    (0..128)
        .filter(|bit| value & (1_u128 << bit) != 0)
        .map(|bit| bit as u8)
        .collect()
}

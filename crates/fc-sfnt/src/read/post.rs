use fc_model::FontInfo;
use read_fonts::{FontRef, TableProvider};

use crate::Error;

pub(crate) fn apply(face: &FontRef<'_>, info: &mut FontInfo) -> Result<(), Error> {
    if face
        .table_data(read_fonts::types::Tag::new(b"post"))
        .is_none()
    {
        return Ok(());
    }
    let post = face.post().map_err(|error| Error::parse("post", error))?;
    info.postscript_is_fixed_pitch = Some(post.is_fixed_pitch() != 0);
    info.postscript_underline_position = Some(f64::from(post.underline_position().to_i16()));
    info.postscript_underline_thickness = Some(f64::from(post.underline_thickness().to_i16()));
    Ok(())
}

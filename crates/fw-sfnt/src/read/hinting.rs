use fw_model::TtMasterData;
use read_fonts::{tables::maxp::Maxp, types::Tag, FontRef, TableProvider};

use crate::Error;

pub(crate) fn apply(
    face: &FontRef<'_>,
    maxp: &Maxp<'_>,
    target: &mut TtMasterData,
) -> Result<(), Error> {
    target.cvt = match face.cvt() {
        Ok(values) => values.iter().map(|value| value.get()).collect(),
        Err(read_fonts::ReadError::TableIsMissing(_)) => Vec::new(),
        Err(error) => return Err(Error::parse("cvt ", error)),
    };
    target.fpgm = table_bytes(face, Tag::new(b"fpgm"));
    target.prep = table_bytes(face, Tag::new(b"prep"));
    target.max_stack_elements = maxp.max_stack_elements();
    target.max_storage = maxp.max_storage();
    target.max_function_defs = maxp.max_function_defs();
    target.max_instruction_defs = maxp.max_instruction_defs();
    target.max_twilight_points = maxp.max_twilight_points();
    target.max_zones = maxp.max_zones();
    target.max_size_of_instructions = maxp.max_size_of_instructions();
    Ok(())
}

fn table_bytes(face: &FontRef<'_>, tag: Tag) -> Vec<u8> {
    face.table_data(tag)
        .map(|data| data.as_bytes().to_vec())
        .unwrap_or_default()
}

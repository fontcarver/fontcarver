use super::{checked, error};
use crate::Error;
use fc_model::Bitmap;
use std::io::Cursor;

pub(super) fn decode(data: &[u8]) -> Result<Bitmap, Error> {
    let mut decoder = png::Decoder::new(Cursor::new(data));
    decoder.set_limits(png::Limits {
        bytes: fc_model::MAX_BITMAP_BYTES,
    });
    decoder.set_transformations(png::Transformations::EXPAND);
    let mut reader = decoder.read_info().map_err(|e| error(e.to_string()))?;
    let info = reader.info();
    let width = u16::try_from(info.width).map_err(|_| error("PNG width overflow"))?;
    let height = u16::try_from(info.height).map_err(|_| error("PNG height overflow"))?;
    if info.bit_depth == png::BitDepth::Sixteen {
        return Err(error(
            "16-bit-channel PNG is not supported by RGBA8 strikes",
        ));
    }
    let mut bitmap = checked(Bitmap::new(width, height, 32))?;
    if reader.output_buffer_size() > fc_model::MAX_BITMAP_BYTES {
        return Err(error("PNG decode allocation limit"));
    }
    let mut buffer = vec![0; reader.output_buffer_size()];
    let frame = reader
        .next_frame(&mut buffer)
        .map_err(|e| error(e.to_string()))?;
    let channels = frame.color_type.samples();
    for (dst, src) in bitmap
        .rows
        .chunks_exact_mut(4)
        .zip(buffer[..frame.buffer_size()].chunks_exact(channels))
    {
        match frame.color_type {
            png::ColorType::Rgba => dst.copy_from_slice(src),
            png::ColorType::Rgb => dst.copy_from_slice(&[src[0], src[1], src[2], 255]),
            png::ColorType::Grayscale => dst.copy_from_slice(&[src[0], src[0], src[0], 255]),
            png::ColorType::GrayscaleAlpha => {
                dst.copy_from_slice(&[src[0], src[0], src[0], src[1]])
            }
            _ => return Err(error("unexpanded PNG palette")),
        }
    }
    Ok(bitmap)
}

pub(super) fn encode(bitmap: &Bitmap) -> Result<Vec<u8>, Error> {
    checked(bitmap.validate())?;
    if bitmap.width == 0 || bitmap.height == 0 {
        return Err(error("PNG cannot encode zero-sized images"));
    }
    let mut rgba = Vec::with_capacity(usize::from(bitmap.width) * usize::from(bitmap.height) * 4);
    if bitmap.depth == 32 {
        rgba.extend_from_slice(&bitmap.rows);
    } else {
        for row in 0..bitmap.height {
            for col in 0..bitmap.width {
                rgba.extend([0, 0, 0, bitmap.coverage(col, row)]);
            }
        }
    }
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(
            &mut bytes,
            u32::from(bitmap.width),
            u32::from(bitmap.height),
        );
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(|e| error(e.to_string()))?;
        writer
            .write_image_data(&rgba)
            .map_err(|e| error(e.to_string()))?;
    }
    Ok(bytes)
}

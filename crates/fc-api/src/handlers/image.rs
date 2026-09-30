//! Bounded source loading outside document locks and revision-checked commits.
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use fc_doc::Command;
use fc_model::{Image, Layer};
use kurbo::{BezPath, Shape};
use sha2::{Digest, Sha256};

use crate::{
    Api, ApiError, Change, ImageImportParams, ImageSetParams, ImageState, ImageTarget,
    ImageTraceParams, ImageTraceResult,
};

fn invalid(message: impl std::fmt::Display) -> ApiError {
    ApiError::invalid_params(message.to_string())
}

fn layer<'a>(
    font: &'a fc_model::Font,
    glyph: &str,
    id: &fc_model::LayerId,
) -> Result<&'a Layer, ApiError> {
    font.glyph(glyph)
        .and_then(|g| g.layer(id))
        .ok_or_else(|| ApiError::not_found("image layer does not exist"))
}

fn read_png(path: &Path) -> Result<Vec<u8>, ApiError> {
    let file = File::open(path).map_err(invalid)?;
    if !file.metadata().map_err(invalid)?.is_file() {
        return Err(invalid("image source must be a regular PNG file"));
    }
    let mut bytes = Vec::new();
    file.take((fc_raster::MAX_IMAGE_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(invalid)?;
    fc_raster::image_dimensions(&bytes).map_err(invalid)?;
    Ok(bytes)
}

fn resolve(image: &Image, document_path: Option<&Path>) -> Result<Vec<u8>, ApiError> {
    if let Some(data) = &image.data {
        fc_raster::image_dimensions(data).map_err(invalid)?;
        Ok(data.clone())
    } else {
        read_png(
            &fc_doc::image_paths::resolve_image_path(document_path, &image.file_name)
                .map_err(invalid)?,
        )
    }
}

pub(crate) fn state(api: &Api, target: ImageTarget) -> Result<ImageState, ApiError> {
    let (generation, image, path) = api
        .session
        .with_doc(target.doc, |doc| {
            Ok::<_, ApiError>((
                doc.generation,
                layer(&doc.font, &target.glyph, &target.layer)?
                    .background_image
                    .clone(),
                doc.path.clone(),
            ))
        })
        .map_err(ApiError::from)??;
    let mut state = ImageState {
        generation,
        image,
        png: None,
        width: None,
        height: None,
        error: None,
    };
    if let Some(image) = &state.image {
        match resolve(image, path.as_deref()).and_then(|data| {
            fc_raster::image_dimensions(&data)
                .map(|size| (data, size))
                .map_err(invalid)
        }) {
            Ok((data, (width, height))) => {
                state.png = Some(data);
                state.width = Some(width);
                state.height = Some(height);
            }
            Err(error) => state.error = Some(error.message),
        }
    }
    Ok(state)
}

pub(crate) fn set(api: &Api, params: ImageSetParams) -> Result<Change, ApiError> {
    let data_changed = api
        .session
        .with_doc(params.doc, |doc| {
            if doc.generation != params.expected_generation {
                return Err(invalid("image edit is stale; refresh the layer"));
            }
            let current = layer(&doc.font, &params.glyph, &params.layer)?;
            Ok(current
                .background_image
                .as_ref()
                .and_then(|i| i.data.as_ref())
                != params.image.as_ref().and_then(|i| i.data.as_ref()))
        })
        .map_err(ApiError::from)??;
    // A malformed pre-existing embedded image can still be unlocked or removed.
    if let Some(data) = params
        .image
        .as_ref()
        .and_then(|i| i.data.as_ref())
        .filter(|_| data_changed)
    {
        fc_raster::image_dimensions(data).map_err(invalid)?;
    }
    api.session
        .with_doc_mut(params.doc, |doc| {
            if doc.generation != params.expected_generation {
                return Err(invalid("image edit is stale; refresh the layer"));
            }
            layer(&doc.font, &params.glyph, &params.layer)?;
            doc.apply(
                Command::SetLayerImage {
                    glyph: params.glyph.into(),
                    layer: params.layer,
                    image: params.image,
                },
                "Edit source image",
            )
            .map_err(ApiError::from)
        })
        .map_err(ApiError::from)?
}

pub(crate) fn import(api: &Api, params: ImageImportParams) -> Result<Change, ApiError> {
    let document_path = api
        .session
        .with_doc(params.doc, |doc| {
            if doc.generation != params.expected_generation {
                return Err(invalid("image import is stale; refresh the layer"));
            }
            let target = layer(&doc.font, &params.glyph, &params.layer)?;
            if target.background_image.as_ref().is_some_and(|i| i.locked)
                || doc.font.glyph(&params.glyph).is_some_and(|g| g.locked)
            {
                return Err(invalid("the image or glyph is locked"));
            }
            Ok(doc.path.clone())
        })
        .map_err(ApiError::from)??;
    let (file_name, bytes) = match (params.path, params.data) {
        (Some(path), None) => {
            let input = PathBuf::from(&path);
            let resolved = if input.is_absolute() {
                input
            } else {
                fc_doc::image_paths::resolve_image_path(document_path.as_deref(), &path)
                    .map_err(invalid)?
            };
            let bytes = read_png(&resolved)?;
            let name = if params.embed {
                resolved
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("image.png")
                    .to_string()
            } else {
                let document = document_path
                    .as_deref()
                    .ok_or_else(|| invalid("save the document before linking an image"))?;
                fc_doc::image_paths::relative_image_path(document, &resolved).map_err(invalid)?
            };
            (name, bytes)
        }
        (None, Some(bytes)) if params.embed => {
            fc_raster::image_dimensions(&bytes).map_err(invalid)?;
            let name = Path::new(&params.file_name)
                .file_name()
                .and_then(|n| n.to_str())
                .filter(|n| !n.is_empty())
                .unwrap_or("image.png")
                .to_string();
            (name, bytes)
        }
        _ => return Err(invalid("provide one PNG path or embedded upload")),
    };
    set(
        api,
        ImageSetParams {
            doc: params.doc,
            glyph: params.glyph,
            layer: params.layer,
            expected_generation: params.expected_generation,
            image: Some(Image {
                file_name,
                data: params.embed.then_some(bytes),
                ..Default::default()
            }),
        },
    )
}

pub(crate) fn trace(api: &Api, params: ImageTraceParams) -> Result<ImageTraceResult, ApiError> {
    let (generation, target, document_path) = api
        .session
        .with_doc(params.doc, |doc| {
            if !params.dry_run && params.expected_generation != Some(doc.generation) {
                return Err(invalid("image trace is stale; preview again"));
            }
            if doc.font.glyph(&params.glyph).is_some_and(|g| g.locked) {
                return Err(invalid("the image glyph is locked"));
            }
            Ok((
                doc.generation,
                layer(&doc.font, &params.glyph, &params.layer)?.clone(),
                doc.path.clone(),
            ))
        })
        .map_err(ApiError::from)??;
    let image = target
        .background_image
        .as_ref()
        .ok_or_else(|| invalid("there is no source image"))?;
    let data = resolve(image, document_path.as_deref())?;
    let source_hash = format!("{:x}", Sha256::digest(&data));
    if !params.dry_run && params.expected_source_hash.as_ref() != Some(&source_hash) {
        return Err(invalid("image source changed; preview again"));
    }
    let contours =
        fc_raster::trace_image(&data, image.transform, params.threshold, params.tolerance)
            .map_err(invalid)?;
    let mut path = BezPath::new();
    for contour in &contours {
        path.extend(
            contour
                .try_to_bezpath()
                .map_err(invalid)?
                .elements()
                .iter()
                .copied(),
        );
    }
    let bounds = path.bounding_box();
    let mut result = ImageTraceResult {
        source_hash,
        generation,
        contours,
        path: path.to_svg(),
        bounds: [bounds.x0, bounds.y0, bounds.x1, bounds.y1],
        change: None,
    };
    if !params.dry_run {
        if result.contours.is_empty() {
            return Err(invalid(
                "the trace is empty; adjust the threshold before applying",
            ));
        }
        let mut contours = if params.append {
            target.contours
        } else {
            Vec::new()
        };
        contours.extend(result.contours.clone());
        let command = Command::ReplaceLayerOutline {
            glyph: params.glyph.into(),
            layer: params.layer,
            contours,
            components: if params.append {
                target.components
            } else {
                Vec::new()
            },
            anchors: target.anchors,
        };
        result.change = Some(
            api.session
                .with_doc_mut(params.doc, |doc| {
                    if doc.generation != generation {
                        return Err(invalid("image trace is stale; preview again"));
                    }
                    doc.apply(command, "Trace source image")
                        .map_err(ApiError::from)
                })
                .map_err(ApiError::from)??,
        );
    }
    Ok(result)
}

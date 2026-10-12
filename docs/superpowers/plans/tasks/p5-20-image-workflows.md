# P5-20 Image workflows

Scope: feature-audit rows 92 and 93. Add bounded PNG source images to glyph layers,
relative document links or embedded bytes, affine placement, explicit locking,
missing/invalid link feedback, threshold/simplification trace preview and atomic
apply/undo. Native source preserves the image; UFO writes embedded PNG data.
Raster support is explicitly PNG, with polygon outlines rather than invented
photographic Bezier fitting. No FontForge source is used.

Ownership: image_workflows owns model Image.locked, SetLayerImage command, image
API types/handlers, native schema, integration tests, and narrow registries.
Delegates own fc-raster/src/source_image.rs plus its tests, and isolated frontend
image panel/drawing modules with narrow inspector/canvas integration.

Interfaces: image.state(ImageTarget)->ImageState; image.import(ImageImportParams)
->Change; image.set(ImageSetParams)->Change; image.trace(ImageTraceParams)
->ImageTraceResult. Each mutation requires expected_generation. State includes
resolved PNG bytes, dimensions, and broken-link error without failing the whole
inspector. Trace apply recomputes from the current image and retains image source.

Acceptance: PNG dimensions/memory and malformed input bounds; alpha-aware dark
ink threshold; holes and affine transforms; simplification and preview/apply
identity; no mutation on stale/rejected input; lock/unlock enforcement; native
roundtrip and undo/redo; relative-link resolution/broken status; source-preserving
export; frontend controls and transform-correct background paint; isolated
headless agent-browser smoke. Parent runs final broad checks after integration.

# P5 bare CFF interchange

Owner: `remaining_formats/type42_cff`; parent owns module registration.

Implement single-face CFF1 import/export in `fc-sfnt/src/cff_interchange.rs`
without new dependencies or FontForge code. Reuse the checked CFF evaluator,
hint importer and writer. Import supplies temporary sfnt metrics internally,
then removes that synthetic source provenance. Support name-keyed and CID CFF,
fractional outlines, advances, private hints and font-matrix units per em.
Reject CFF2 and font sets with more than one face explicitly.

Public interfaces: `read_bytes(&[u8]) -> Result<Font, Error>`,
`read_outcome(&[u8]) -> Result<ReadOutcome, Error>` and
`write_bytes(&Font, &MasterId) -> Result<Vec<u8>, write::WriteError>`.
The integration helper `read_path(&Path) -> Result<ReadOutcome, Error>` reads
at most the 128 MiB input limit plus one sentinel byte.
The outcome warns that bare CFF lacks Unicode, layout and sfnt metadata.
Only directly interpretable glyph names infer Unicode; never invent CID maps.

Acceptance: semantic model round trip including non-1000 UPEM, external CFF
table fixture import, CID identity, malformed/truncated input and invalid model
rejection. Scoped rustfmt, focused Cargo tests serialized by the shared lock,
artifact checks before/after. Parent owns final workspace lint/regression.

Separate delegated Type42 work owns only its new `fc-formats` module/tests.

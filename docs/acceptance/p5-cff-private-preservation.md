# CFF1 CID private dictionary preservation acceptance

Implementation completed on 2026-09-30; runtime acceptance is pending the parent
integration run, per the final instruction to cancel delegated Cargo queues.

## Changes

Masters retain additional private dictionaries in `cff_private_dicts` (FD 1+);
layers retain `cff_private_dict_index`, with absent/zero selecting FD 0's existing
`private_dict`. Both serialize with backward-compatible defaults. Nonzero FD
selection counts as meaningful content even on an empty outline.

CFF1 reads every private dictionary and its glyph selection, normalizes supported
positive axis-aligned matrices, and warns when private parameters cannot survive
matrix normalization. Output writes FDArray/FDSelect in actual exported glyph
order, validates every dictionary, and rejects invalid selections, more than 256
dictionaries, or additional dictionaries without CID metadata. The format follows
[Adobe CFF specification sections 15, 18 and 19](https://adobe-type-tools.github.io/font-tech-notes/pdfs/5176.CFF.pdf).

Integrated the subroutinization owner's wrapper/hook in the shared CFF writer.
Authored flex and flex-marked component bases fail explicitly; details are in
[the flex guard report](p5-cff-flex-guard.md).

## Verification

Changed-file `rustfmt --check` and `git diff --check` pass. The first focused Cargo
attempt compiled `fc-model`, then failed on the unrelated concurrent TTX
`ParsingOptions.entity_resolver` initializer. Its owner subsequently fixed that
error. The combined rerun was cancelled while queued (exit 143), as requested;
no runtime pass or independent renderer acceptance is claimed.

Pending: four `fc-sfnt --test cff_private` tests, three `--test cff_flex` tests,
two private-normalization library tests, two `fc-model --test cff_private` tests,
and parent Clippy/regression checks. Setting `FC_CFF_PRIVATE_ACCEPTANCE` to a
task-owned output path during the CID round-trip test emits an OTF for independent
FontTools/FreeType checks; those checks have not run.

## Boundaries

CFF2 retains its explicit default-dictionary limitation. Imported flex binding and
unsupported private operators remain outside this slice. Other owners handle the
editor, transfers and export preprocessing. No FontForge code/executable or new
dependency was used. Owned queues stopped; owned scratch removed; disk check
result is recorded in the parent handoff.

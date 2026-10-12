# P5 remaining interchange formats

Scope: new bounded SVG-font and EPS outline codecs and AFM/PFM metric interchange in `fc-formats`; raw-table TTX import/export and TTC/OTC collection writing in `fc-sfnt`; API and CLI source opening. Export workflow owner integrates export routing. No FontForge implementation is read or run.

Interfaces: `vector::{read_svg,write_svg,read_eps,write_eps}` and `metrics` file/model adapters (delegated); `ttx::{encode,decode,read_path}`; `collection::{encode,extract_face}`.

Semantic boundaries: TTX raw=True hexdata preserves complete binary tables and rejects decompiled semantic XML, split external XML references and unsupported attributes; generated TTX is consumable by fontTools. Collection packing retains complete tables, uses TTC v1 and does not merge or renumber glyphs; signed DSIG inputs are rejected as invalidated by repacking. Inputs and aggregate outputs have explicit size, face/table-count and XML-node limits. SVG/EPS and metric codec plans define supported subsets explicitly. EPS export chooses an explicit glyph. AFM/PFM are metrics-only formats.

Acceptance: tests first for malformed/oversized data, independent read-fonts/fontTools decoding, semantic multi-face TTC round trip, raw TTX table preservation and rejection, API/CLI import/export integration and failed-input safety. Focused crate tests under shared Cargo lock; final workspace checks owned by root. Record exact commands and actual results in separate acceptance report. Resource checker runs at task boundaries; scratch removed on completion.

Primary references: Microsoft OpenType font-file specification (https://learn.microsoft.com/en-us/typography/opentype/spec/otff); fontTools TTX documentation and implementation (https://fonttools.readthedocs.io/en/latest/ttx.html, https://fonttools.readthedocs.io/en/latest/_modules/fontTools/ttLib/ttFont.html).

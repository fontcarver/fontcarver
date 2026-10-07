# Licensing policy

FontCarver is licensed **MIT OR Apache-2.0** — the Rust ecosystem's dual-license
convention. A licensee picks either license; contributions come in under both.
The copyright line is `Copyright (c) 2026 Tom Chen and the FontCarver
contributors`. The two texts are `LICENSE-MIT` and `LICENSE-APACHE`; the workspace
`Cargo.toml` declares `license = "MIT OR Apache-2.0"` and every crate inherits it
with `license.workspace = true`.

This document explains why that is possible for a project that openly takes
FontForge as its reference, and states the rules contributors must follow so that
it stays true.

## 1. Why a project "based on FontForge" is not GPL

Two different things are easy to confuse:

- **Learning from a program.** Reading source code, running a program, observing
  what it does with a given input, and writing down how a file format is laid out
  produces _facts and ideas_. Copyright does not cover facts, ideas, algorithms or
  file formats — only the particular expression of them. A new implementation
  written from that understanding is an original work, licensed however its
  authors choose.
- **Copying code from a program.** Taking a function, a table, or a distinctive
  block of expression and translating or transcribing it produces a _derivative
  work_, and it carries that code's license with it.

FontCarver does the first and, in the narrow case described in §3, a strictly
bounded amount of the second.

Concretely: FontCarver is written from scratch in Rust and TypeScript. Its font
logic is built on the permissively licensed Rust font ecosystem — `read-fonts`,
`write-fonts`, `skrifa`, `font-types`, `fontc`, `fea-rs`, `kurbo`, `flo_curves`,
`harfrust`, `norad`, `tiny-skia`, plus Tauri and React for the shell and UI. None
of it is a line-by-line translation of FontForge's C. FontCarver does not link
against FontForge, does not call it as a subprocess, does not embed any part of
it, and does not ship it.

## 2. What FontForge's license actually says

FontForge as a _whole distribution_ is offered under the GPLv3-or-later, because
the whole necessarily includes its most restrictive parts. But `fontforge/LICENSE`
is explicit that this is not the license of most of the code:

> FontForge is available as a whole under the terms of the GNU GPL, version 3 or
> any later version. However, almost all of its parts are available under the
> "revised BSD license" because FontForge was mostly written by George Williams,
> using that license.
>
> […] FontForge was known from 2000 to 2004 as PfaEdit, and was written and
> developed primarily by George Williams from 2000 until 2012. It is published as
> free/libre software and distributed under the 3-clause BSD license. […] Some
> features added since 2012 are licensed by their individual developers under the
> GPLv3.

That file goes on to enumerate, per directory, which sources are revised-BSD,
which are GPL, and which carry other licenses (Open Group, an Adobe Type 1
specification license, or nothing at all). The per-file header in the C source is
the authoritative statement for any given file.

The 3-clause BSD license is permissive: it allows use, modification and
redistribution — including inside a differently licensed work, and including
proprietary works — on the condition that the copyright notice, the list of
conditions and the disclaimer are retained, and that the author's name is not used
to endorse the derived product. Those conditions are satisfiable inside an
MIT/Apache-2.0 project. The GPLv3 is not: it would require the combined work to be
distributed under the GPLv3, which is incompatible with offering FontCarver
under MIT or Apache-2.0.

Hence the asymmetry that drives every rule below: **BSD-licensed FontForge files
may be adapted with attribution; GPL-only FontForge files may not be adapted at
all.**

## 3. The adaptation rule

1. **Read the header first.** Before adapting anything from the `fontforge/`
   checkout, open the source file and read its license header. Only a file whose
   own header states the revised (3-clause) BSD license may be adapted. If the
   header says GPL — or says nothing, or says something else (Open Group, Adobe) —
   the file is off limits for copying.
2. **Attribute every adapted block.** Each adapted function or module carries a
   comment at its top:

   ```rust
   // Adapted from FontForge <path> (3-clause BSD, Copyright (C) <years> George Williams / <author>)
   ```

   and a row in `THIRD_PARTY.md` naming our file, the FontForge source file, and
   the license found in that file's header. `THIRD_PARTY.md` also carries the
   verbatim BSD text and copyright notice, which is what discharges the license's
   retention conditions.

3. **Re-implement, don't copy, from GPL-only files.** Where a behaviour worth
   having lives in a GPL-only FontForge file, it is re-implemented from the format
   specification, from published documentation, or from observed input/output
   behaviour — never by reading that file and transcribing it. Naming a FontForge
   flag or policy in a doc comment so that readers know which behaviour is being
   matched (for example `ForceEndPointExtrema`) is description, not copying.
4. **Never link or embed FontForge.** No linking against libfontforge, no bundled
   binaries, no shelling out to `fontforge`, no vendored FontForge sources in the
   tree.
5. **The reference checkout is never shipped.** `fontforge/` is a local research
   checkout, listed in `.gitignore`, never committed, and not part of any build or
   release artifact. Tests that want its broader font corpus reach it through
   `fc_fixtures::external("fontforge-tests")`, which returns `None` when the
   checkout is absent so those tests skip cleanly.

"Reference, not translation" means exactly this: FontForge tells us _what_ a
mature font editor must handle — which formats, which edge cases, which
operations, which invariants — and `docs/research/*.md` records that knowledge in
our own words. It does not tell us _how_ to write our code.

## 4. Trademark and naming

FontCarver is not affiliated with, endorsed by, or sponsored by the FontForge
project or its contributors. "FontForge" appears in this repository only in its
descriptive sense — naming the program we studied, its file formats (SFD) and its
documented behaviours. It is not used as, or as part of, a FontCarver product
name, and FontCarver is not presented as a version, edition, fork or successor
of FontForge. The same restraint applies to Glyphs, FontLab and FontCreator, which
are named in the research and design documents purely for UX comparison.

## 5. Dependency policy

Every dependency FontCarver ships or builds against must be under a permissive
license:

**Allowed:** MIT, Apache-2.0 (including "WITH LLVM-exception"), BSD-2-Clause,
BSD-3-Clause, 0BSD, ISC, Zlib, BSL-1.0, Unicode-3.0 / Unicode-DFS-2016,
CC0-1.0 / MIT-0 / Unlicense (public-domain equivalents), and — for fonts and font
data only — SIL OFL 1.1 and the Bitstream Vera license.

**Forbidden:** GPL, LGPL, AGPL, SSPL, CDDL, EUPL, and any other copyleft or
source-available license, in any version, as a direct or transitive dependency of
a shipped artifact.

That rule governs **application dependencies** — the Cargo and npm graphs that are
linked into FontCarver itself. Libraries that a native bundler *copies* into an
artifact are a separate layer with a separate rule, because they are not linked
into FontCarver and do not change its license:

- **LGPL is permitted** in such a payload, provided the artifact carries the
  materials LGPL-2.1 §6 asks for: the LGPL license text, a notice naming each
  library and its version, and that library's source or a written offer for it.
  `scripts/appimage_inventory.py --notice-dir` generates them and
  `scripts/appimage_policy.py` refuses a release without them.
- **GPL, AGPL and SSPL are not permitted.** GPL has no linking exception, so a
  GPL shared library loaded into the application process is a real distribution
  problem rather than a notice obligation.
- **An unidentified license is not permitted** until it is identified. Missing
  license metadata is unknown, never evidence of permissiveness.
- A **GPL label requires a recorded ruling**, not an automatic refusal. Debian
  copyright files describe source packages, so a GPL label often covers utilities
  that were never shipped — libfuse is the standard example, with `Files: *` under
  GPL-2 while the shipped `lib/*` is LGPL-2.1. Such a package passes only via an
  entry in `appimage_licenses.RULINGS` naming the package and the reason.
- Never enable `bundleMediaFramework` in `tauri.conf.json` without re-running that
  scan. It bundles GStreamer, whose `ugly` plugin set is the one case Tauri's own
  documentation warns about, and it is the main route by which genuinely GPL code
  would enter a payload. Its absence from the config is what keeps it disabled.

**Case by case:** MPL-2.0 is file-level copyleft and does not reach across file
boundaries, so it can be carried inside an MIT/Apache-2.0 distribution — but only
with an explicit ruling recorded in `THIRD_PARTY.md` naming the crate, the reason
it is present, and the fact that it is unmodified. Do not add an MPL-2.0
dependency without that entry.

A dual- or multi-licensed package (`MIT OR Apache-2.0`, `Apache-2.0 OR MIT`,
`MIT OR Apache-2.0 OR LGPL-2.1-or-later`, …) is judged on the permissive branch we
take; it is allowed as long as at least one branch is on the allowed list.

Practical rules:

- Adding a dependency requires a one-line justification in the commit or PR
  message (see `CONVENTIONS.md`) _and_ a check of its license.
- Re-run the surveys in `THIRD_PARTY.md` when the dependency graph changes
  materially:
  `cargo metadata --format-version 1` for Rust, `pnpm --dir app licenses list` for
  npm.
- A package with no license field at all is treated as forbidden until its
  licensing is established.

### 5.1 Native Linux payloads and the AppImage

The source-package survey does not approve code copied by a native bundler.
Inspect the complete delivered payload, including launchers, plugins, data and
the executable runtime prepended to an AppImage's SquashFS filesystem. Missing
license data is **unknown**, never evidence of a permissive license. Debian
copyright files describe source packages; a copyleft label may apply to unshipped
tests or tools, so it is a review lead rather than a license ruling for a binary.

**Bundling these libraries does not change FontCarver's license.** GTK, GLib,
WebKitGTK and GStreamer core are LGPL-2.1, and the LGPL exists precisely to permit
linking from differently licensed programs. The application also sits inside the
payload while the runtime only mounts it and `exec`s the binary, which is
aggregation rather than linking. `fusermount3` is GPL-2 but is invoked as a
separate process, across a process boundary rather than a linking one. An earlier
revision of this section read as though copyleft in a payload were a legal bar on
shipping. It is not, and that was the wrong reason to state.

The real obligation is **notice plus source availability**: redistributing a copy
of an LGPL library triggers LGPL-2.1 §6, which asks for the license text, a notice
naming the library, the user's ability to replace it — satisfied mechanically by
dynamic linking — and the library's source or a written offer for it. A credit line
alone does not discharge the third item. §5 states the resulting rule; this section
states why it is shaped that way.

What the blanket ban actually bought was cheap auditing, and what it cost was a
measured compliance debt going unaddressed. `THIRD_PARTY.md` §3.4 records it: the
first real build carried 177 shared objects from 126 Debian packages with copyright
documents for ten of them. The gate in `scripts/appimage_policy.py` now closes that
gap per library instead of refusing the format wholesale, and an unidentifiable
license blocks the release rather than being recorded as unknown.

The Debian delivery keeps GTK/WebKit and their dependencies on the recipient's
system. **Do not copy those libraries into the deb** on the assumption that
dynamic linking makes redistribution obligations disappear; the deb's contract is
its `Depends` field. Release notice generation retains `native/HOST-LIBRARIES.txt`
and unmodified, hash-checked LGPL 2 and 2.1 texts from `scripts/release_licenses/`.
Those same texts are the AppImage's §6 documents, copied into the image by
`appimage_inventory.py --notice-dir`.

`scripts/appimage_inventory.py` records all extracted paths and hashes, the
separate runtime hash, and current-host Debian ownership candidates. It does not
execute the inspected image or convert those candidates into build provenance.
Its successful exit means the evidence was generated, not that the image is
approved; `scripts/appimage_policy.py` separately controls release eligibility.

## 6. Test fixtures

Font fixtures are _data_, not code, and live in `fixtures/`. They are aggregated
with FontCarver, not derived from it, so their licenses do not affect
FontCarver's own license — but they are still redistributed by this repository
and so must be licensed for redistribution, and their terms must be recorded.
Every committed fixture is listed in `fixtures/README.md` with its source and
license and in `THIRD_PARTY.md` with the license as verified from the font's own
`name` table. Prefer OFL or otherwise permissively licensed fixtures; a copyleft
fixture is a liability worth removing when an equivalent exists.

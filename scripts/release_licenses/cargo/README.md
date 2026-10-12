# Cargo notice sources

`../cargo.json` maps exact `name@version` identifiers to notice files relative to
`scripts/release_licenses`, their immutable upstream URLs, and SHA-256 digests of
the saved bytes. These files supplement notices present in downloaded crates;
they are not inferred copyright statements or dependency-policy exceptions.

Most source revisions come from each published crate's `.cargo_vcs_info.json`.
The older `cesu8`, `difflib`, and WinAPI target packages need release-history
verification because their archives omit that file. Source URLs in the catalog
remain pinned to full commit identifiers, including when several packages share
one upstream notice. A dependency version update requires a fresh review.

For `cesu8`, commit `f18962a9de5189f21600b7ff2898845157df4447` matches
the archive's manifest and `src/lib.rs` byte for byte. For both WinAPI target
crates, commit `9497609ef44cc9bcd16cd2411c0ee6ccaf5483aa` matches their
manifests, `build.rs`, and `src/lib.rs`; the original publication commit is not
known. The pinned `difflib` revision declares version 0.4.0 but contains source
changes compared with the archive. Its exact publication revision remains
unverified; the retained MIT license comes from that same-version upstream
snapshot, not a claimed exact reconstruction of the published source revision.

Special cases:

- The fontc workspace's pinned root `LICENSE` supplies Apache-2.0 even though
  package metadata also offers MIT. No absent MIT text was invented. Fontdrasil's
  Adobe glyph data files retain their own BSD notices in full.
- `r-efi` embeds its MIT permission text and contributor notices in `AUTHORS`.
  The original file also describes alternative licenses; retaining it does not
  select the LGPL alternative.
- UNIC's root copyright and author files accompany its original license texts.
- `cesu8` offers Apache-2.0/MIT but its pinned release has neither complete
  primary license text. Its original README and locally collected
  `COPYRIGHT-RUST.txt` accompany the standard Apache-2.0 text, selecting the
  expressly offered Apache branch. The copyright file is not duplicated here.
- The objc2 workspace supplies a license declaration rather than full license
  texts. Apache-2.0 standard text accompanies packages offering that branch;
  MIT-only packages receive the unmodified standard MIT text plus pinned crate
  and workspace manifests. Manifest authors remain authors, not newly asserted
  copyright holders. Upstream does not provide copyright lines in `LICENSE.md`;
  `objc2-foundation` also lacks package authors metadata.
- `selectors` retains its exact upstream source file containing the MPL notice,
  alongside the complete standard MPL-2.0 text. This collection does not grant an
  MPL policy exception.

Standard texts come from the SPDX license-list-data repository at commit
`31ba1a50e5397e00a304dbadc76531740e89ee48`. Templates are preserved unchanged;
placeholder copyright fields are not filled from guesses.

Upstream platform caveats remain unresolved by this collection. The objc2
`LICENSE.md` discusses Apple SDK derivation and uncertainty about redistribution.
The WebView2 project includes Microsoft SDK loader binaries while its pinned
repository supplies only the project's MIT license, without a separate
  Microsoft SDK redistribution notice. Its root README identifies SDK version
1.0.3650.58 and describes the static MSVC loader and GNU DLL loader. This catalog
preserves available notices; it does not establish missing SDK redistribution
terms or claim that either platform question has been legally resolved.

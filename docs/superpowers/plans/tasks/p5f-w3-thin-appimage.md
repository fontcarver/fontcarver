# Permissive thin AppImage candidate feasibility and implementation

Task: p5f-w3-thin-appimage, authorized by parent under current user request.

## Scope and ownership

Own new scripts/build_thin_appimage.py and tests if implemented; docs/p5f-w3-thin-appimage-acceptance.md; task-local runtime sources/binaries. Do not edit build_release.py/check_release_bundle.py/release_licenses or shared docs: coordinate /root/w3_packaging and /root/w3_appimage for interfaces.

## Interfaces and acceptance

User wants remaining AppImage tasks done, publication deferred. Existing Tauri image bundles forbidden LGPL/GPL runtime/payload. License agent found legacy AppImageKit13 runtime may be permissive with dynamic hostFUSE2: runtime/libappimageMIT, squashfuseBSD2, liblzmapublicdomain,zlib. Independently verify exact source/build closure and authentic pinned runtime; determine if a thin image containing only app/assets and permissible components can launch via hostGTK/WebKit with explicit distro prerequisites. Prefer trustworthy source build or independently verified exact upstream binary with reproducible inventory; do not blindly substitute oldruntime. If feasible, implement/test bounded custom packaging without changingpolicy or claiming universalportable image, wire with packaging owner. All heavybuilds/caches require parent slot; newdeps/source at scratch only; never commit largebinaries. If actual validated safe solution cannot be achieved, state specific evidence and keep release gate closed, not fake completion.

## Protocol

Read AGENTS.md, CONVENTIONS.md, docs/superpowers/plans/tasks/p5f-protocol.md and relevant audit/evidence first. Append gap analysis before implementation. Shared checkout: no commits/push/reset/clean or others' edits, no nested agents. Parent owns integration/status, coordinates builds. Scratch only .scratch/p5f-w3-thin-appimage, absolute task TMPDIR, artifact checker at start/batches/end, Cargo via flock .scratch/p5f/cargo.lock with shared target/default profiles. No native/global desktop input automation. Preserve exact evidence in own report, stop ownprocesses and delete owned scratch after acceptance. User defers publication and has no signingcredentials; no publishing or inventednative evidence. Report under300words.

## Gap analysis

Row 62: the standard AppImage runtime statically embeds LGPL FUSE and the
standard payload copies system libraries, violating the project dependency
policy. Tag 13 source could dynamically load host FUSE, but upstream marks its
release obsolete. The current type2-runtime commit
`8f39b89e2ac31e1640b3d3f7e9a5108e6ce805fa` has MIT/public-domain runtime source;
its static LGPL closure comes from its upstream build recipe. A feasibility
compile on this Ubuntu 24.04 host successfully instead links dynamically to
`libsquashfuse.so.0`, `libsquashfuse_ll.so.0`, `libfuse3.so.3`, and `libc.so.6`.
The candidate will use that pinned source, explicit host-library prerequisites,
a verified deb payload, source/recipe/build receipts and license notices. It
will not copy any system library or use an unverified prebuilt runtime. GCC's
three warnings in the upstream source require bounded read/buffer checks; the
upstream help and static-runtime marker must accurately reflect this variant.
Acceptance will check extraction, ordinary FUSE execution and no-display GTK
startup, with actual FUSE/environment failures reported. Universal portability,
publication and signing remain outside this candidate's claims.

## Later user scope change

The parent relayed the user's instruction to finish the current repair and stop.
Therefore this task ends after implementation, independent review, verified
minimal real-runtime FUSE/extraction probes and cleanup. The integrated
FontCarver AppImage build/install is not started and remains explicitly pending
in the acceptance report; row 62 must not be marked Complete from this tooling.

The user then revoked that stop and authorized all remaining local work excluding
publication and cross-platform verification. The original Linux candidate scope
is active again. Reacquire the recorded isolated prerequisites, keep reviewed
production code frozen unless a defect appears, and coordinate the actual image
build and final scratch cleanup with the parent and packaging agent.

## Actual local acceptance

The integrated candidate from application commit `6e35f70` was built and passed
the independent artifact gate, all nine input pins, normal FUSE and
extract-and-run version/no-display probes, and full extraction. A separate
private Xvfb display proved 15 seconds of native GUI-process survival without
input automation or connecting to the existing desktop; this does not prove
visible rendering or interactive native parity. Exact artifact/runtime hashes,
archived probe recipes, receipts and independent-review evidence are recorded in
`docs/p5f-w3-thin-appimage-acceptance.md` and its linked acceptance directory.
Local thin-AppImage work is complete; publication and cross-platform validation
remain outside the user's current scope.

Further actual-artifact review found a retained DEB bundle marker in the thin
image's application ELF. The parent authorized a bounded builder correction,
strict deb/AppImage marker gates and regression tests, followed by a fresh
coordinated rebuild. Startup evidence for the first candidate remains historical;
completion now requires the corrected APP marker and reversible three-byte
provenance to pass on the newly built candidate as well.

Final acceptance: the newly built `f6f6cf9` candidate passes those corrected
marker/receipt checks, actual FUSE/extract-and-run version and no-display probes,
full extraction, and another 15-second isolated Xvfb GUI-process survival check.
The acceptance report now points to that candidate; earlier `6e35f70` results
are explicitly superseded historical evidence.

Final independent release review passed on the actual rebuilt image and deb,
including all nine pins, the source fingerprint, receipts and the exact reversible
three-byte bundle-marker change. After all consuming agents confirmed they were
finished, the task's entire isolated sysroot/probe directory was removed
(78,625,253 bytes, 1,254 regular files); no owned processes or mounts remain.
Final resource check: build 5.33 GiB / 12,262 files, temporary 0.00 GiB / 1 file,
616.36 GiB free, PASS. Final candidates and compact acceptance evidence remain.
This local task is complete; native interactive GUI workflows, other platforms,
signing and publication have not been claimed as verified.

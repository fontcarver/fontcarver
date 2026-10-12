# npm notice sources

`../npm.json` maps exact `name@version` identities to notice files. Each entry
records its path relative to `scripts/release_licenses`, upstream source URL,
and SHA-256 digest. The notice artifacts retain their source bytes, including
complete READMEs when the license is embedded there. Do not replace a missing
upstream notice with a generic license template.

The catalog covers 68 locked package identities using 10 source artifacts.
For all 62 esbuild, Rollup, and Tauri CLI platform variants in `pnpm-lock.yaml`,
the exact npm version metadata was checked to have the same repository commit
as its family notice source. The catalog contains explicit identities, not
wildcards; a version change requires renewed verification.

Additional provenance:

- `imurmurhash@0.1.4`: the complete README comes from the exact npm tarball,
  whose SHA-1 matches registry metadata
  (`9218b9b2b928a238b13dc4fb6b6d576f231453ea`).
- `esrecurse@4.3.0` and `saxes@6.0.0`: sources use the exact npm registry
  `gitHead`; their notices include embedded or inherited attribution.
- `keyv@4.5.4`: all four published files match the pinned version-bump commit
  byte for byte. The license is at that repository commit's root.
- `@humanfs/types@0.15.0`: the exact registry `gitHead` identifies the types
  package and its Apache-2.0 declaration. The source is the sibling core
  package's Apache license in the same repository commit.
- `natural-compare@1.4.0`: the source is the immutable upstream commit that
  added the license. The package version remains 1.4.0 and `index.js` matches
  the published tarball and its registry `gitHead` byte for byte.

The full installed development graph still has two unavailable package
notices: `stackback@0.0.2` and `@napi-rs/lzma-linux-x64-gnu@1.5.1`. Both declare
MIT, but their published artifacts and inspected upstream source history do
not provide the full MIT text. Stackback contains a V8 BSD notice for one
source file; that does not fill the missing package MIT notice and is not
listed as a complete fallback.

These two packages are absent from the production npm graph. The release
audit checks license metadata for the full graph and requires complete notice
coverage for the npm production graph shipped with the application. No
catalog entry grants a policy exception or changes dependency licensing.

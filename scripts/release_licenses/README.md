# Release notice provenance

`check_release_licenses.py` checks dependency license metadata against the
existing policy and exact-version rulings. It retains published package legal
documents and adds the exact-version supplements in `cargo.json` and `npm.json`.
Each supplement records its upstream source URL and SHA-256. These documents
repair missing notice files in published dependency archives; they do not grant
new policy exceptions. The catalogs require review when dependency versions
change. Their adjacent READMEs explain package-specific provenance and caveats.

The distribution includes notices for the complete Cargo graph, conservatively
including build/test dependencies and other platforms, plus production npm
dependencies. Both full dependency graphs are still checked against license
policy. Some npm development tools omit complete license texts upstream; they
are not shipped, and their missing documents are not replaced with invented
copyright statements. The npm catalog preserves the complete documents recovered
during the audit, including exact platform variants; it is available if one of
those packages enters the distribution graph later.

Run `cargo fetch --locked` and `pnpm install --frozen-lockfile`, then:

```sh
python3 scripts/check_release_licenses.py
python3 scripts/check_release_licenses.py --output-dir target/release-notices
```

The output directory must be empty or absent. The audit itself is offline and
does not build code. The generated `DEPENDENCIES.json` records exact upstream
source archive URLs and hashes of the retained notice files. Package metadata
and notice presence checks are mechanical safeguards, not a new legal assessment
of upstream licensing claims. In particular, existing SDK-related upstream
caveats are preserved in their original documents.

This inventory does not inspect native libraries or bundler runtimes added to
actual installers or AppImages, such as GTK/WebKit libraries collected by
linuxdeploy. No such artifacts were built in this task. Complete distribution
license review requires inspecting those artifacts and their additional notices;
a passing Cargo/npm source-package audit does not establish that clearance.

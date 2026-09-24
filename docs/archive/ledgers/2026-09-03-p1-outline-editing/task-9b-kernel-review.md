# Compiled identity and portable fixture scoped review

Root/Astra reviewed `84596c0` and `cc17701`. Approved for P1 integration.

The compile wrapper remains compatible; compiled model identities are retained before post-format-3 removes names. The same order drives large-font cmap patching and shaping, avoiding document-handle guesses. Production names, synthesized .notdef, excluded glyphs and unencoded ligature identities have focused coverage. Root independently ran the post-format-3 shaping regression and seven fixture tests; all passed. Implementer compile/layout suites and clippy also passed.

Fixture lookup honors an explicit override and resolves the sibling checkout through the primary repository for linked worktrees. Relative gitdir paths are canonicalized. The synthetic default-path test is independent of the user's environment. The real Source Han debug open executed successfully; release export acceptance is still pending T12 and is not implied by its debug guard.

No new dependencies, external source adaptations, schema changes or remaining blocking findings.

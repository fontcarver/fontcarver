# T9b integration milestone review

Root inspected state milestone `cbb4679` while implementation continues.
Optional per-tab state and preview visibility introduce no new dependencies.
Two integration findings sent to implementer for the next slice:

- `replaceGlyphTab` cannot find a text tab by first matching doc/gid: distinct
  text tabs may currently select the same glyph. Update the caller's actual tab.
- Derived original-glyph tab ids may now be occupied by a text tab selecting a
  different glyph. Opening that original glyph from the grid must not silently
  focus the wrong active glyph or destroy typed text. Resolve active-target
  reuse and collision-free creation explicitly, with multi-tab regressions.

Sidebar stepping must reconcile text state, and tool transition flushing must
finish against the old target while avoiding recursive `settle`/`runCommand`.
These are in-progress integration findings, not a final T9 rejection/approval.

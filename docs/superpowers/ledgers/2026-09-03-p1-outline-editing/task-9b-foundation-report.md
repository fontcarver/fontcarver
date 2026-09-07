# T9b text foundation report

Status: complete on `p1-t9b` at `5c8bb04` (foundation `0db4f11`, review fixes `5c8bb04`).

## Contracts

- `app/src/views/text/textInput.ts`: `parseTextInput` preserves contiguous Unicode shaping spans, parses exact whitespace-delimited `/name` tokens, specifies `//` and bare-slash behavior, and maps shaped UTF-8 byte boundaries back to the original source.
- `app/src/views/text/textLayout.ts`: `positionShapedRun`, explicit sized-run → font-unit conversion, and `buildTextLayout`. Main shaping always sends `size: null`. Exact compiled names resolve through bounded `glyphs.search`; runs with 64+ distinct names use paged `glyphs.list`. Selected-master widths stay exact. Multi-token RTL reverses token visual order while retaining source bytes. Compiled GIDs remain metadata only; missing/fuzzy names become noneditable diagnostics. Request context supplies generation and reliable document UPEM for all-name text.
- `app/src/views/text/shapeCoordinator.ts`: controller-independent 150 ms debounce; stale polling with capped backoff; stale → fresh without document events; same-request last-good retention; generation-scoped compile-failure deduplication; cancellation and response epochs on update/unmount. Superseding a request aborts further shaping/resolver pool stages. Last-good storage is bounded to one layout and failure history to 64 entries.

## Verification

- Focused Vitest: 19 passed.
- `pnpm --dir app check`: passed.
- `pnpm --dir app lint`: passed.
- Full Vitest: 82 files, 710 tests passed.
- `pnpm --dir app exec prettier --check src/views/text`: passed.

No existing UI, shell, editor, i18n, generated binding, or Rust source was changed. UI integration remains for the reviewed integration phase. Large-font post-v3 runs safely remain noneditable when no exact compiled name is available; the separate kernel identity task can make those names resolvable without weakening this rule.

# Native Open Font supported format filter

Follow-up to the independent rows57–112 audit. Own only
app/src/shell/nativeDialogs.ts and its existing focused test, this task file and
docs/p5f-w3-open-filter-acceptance.md. Read CONVENTIONS.md and p5f-protocol.md.

Trace current session.open support and update OPEN_FONT_EXTENSIONS so already
implemented Type42/bare CFF/SVG/EPS/AFM/PFM/TTX imports can be selected in the
native Open dialog. Include actual accepted aliases only; do not advertise
unsupported formats. Verify existing native dialog tests and a meaningful
filter contract assertion. Do not change codecs, backend, locale or shared docs.
No native/global desktop input automation or claims of native platform execution.

Own task scratch with absolute TMPDIR, resource checks, preserve focused
acceptance evidence and clean scratch/processes. No commits/push/reset/clean.
Independent review remains with parent/release reviewer.

## Gap analysis

`session.open` accepts nine interchange extensions that the native picker filter
omits: cff, t42, p42, type42, svg, eps, afm, pfm and ttx. Export selector, API
and CLI routing already exist. Update the filter only and extend the existing
native-dialog boundary test, plus actual selected-path forwarding for CFF and
Type 42 aliases. Native WebView execution remains unverified; mocked Tauri
boundary tests demonstrate emitted filter and command routing only.

## Outcome

Filter updated after 5 failing assertions demonstrated the omission. Focused
suite now 13/13 passed; scoped ESLint/Prettier/diff checks clean. Independent
release reviewer reran 13/13 and accepted. TypeScript integration check found
the previously handed-off CJK regression fixture cast, reported to its owner;
no filter-specific finding. See docs/p5f-w3-open-filter-acceptance.md.

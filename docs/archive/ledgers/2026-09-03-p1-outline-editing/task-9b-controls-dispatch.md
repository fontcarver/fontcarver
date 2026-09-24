# T9b independent presentation components — Sol

Root permits this dependency-free slice before final T8 approval; the existing
rule that editor/controller integration waits for reviewed T8 remains in force.
Create `p1-text-controls` worktree from current P1. Read CONVENTIONS,
editor-ui.md section8, original Task9b dispatch (oldpaths/names obsolete), and
task-9b-resume-integration-rulings.md. Preserve existing design tokens/components.

Own NEW files under app/src/views/text only: TextBar, PreviewStrip, supporting
presentation types/helpers/CSS/tests; plus new namespaced en keys and matching
English placeholders in zh-CN. Do not wire GlyphView/Canvas/stores/shell yet,
modify existing parsing/coordinator code, or fetch via a second data pipeline.

- TextBar is controlled through props: text/onTextChange and input ref for
  focusing; size/onSizeChange; masters[{id,name}]/master/onMasterChange;
  features Array<[string,boolean]>/onFeaturesChange; direction ltr|rtl and
  onDirectionChange; onSample callback. Include Latin pangram, Chinese sample,
  all-encoded action, disabled P2 kerning sample and disabled P4 vertical option.
  Caller owns resolving all-encoded glyphs through the API. Export useful types.
  Accessible labels and buttons, number validation with safe bounds, 32px normal
  height with usable narrow layout. No new state store or transport calls.
- PreviewStrip accepts already-positioned paths in font units, UPEM, stale /
  compile-failed status and dismissal/toggle callbacks. Define an exported
  PreviewGlyph {path:string,x:number,y:number,xAdvance:number,yAdvance:number}
  list (repeated glyphs are separate positioned entries). Render12/24/48px using
  true per-size UPEM scales and y-up to canvas transform; no font fallback text
  pretending to be actual glyph paths. Clamp/scroll long runs instead of huge
  canvas allocations; 96px total strip perdesign (three adjacent size regions
  are acceptable). Independent dark/light styling, collapse action and status
  label with icon/dot. Optional compact master/feature controls may reuse a
  shared presentation component; caller owns their state. No new font shaping
  requests in the presentation layer, no controller.

Use React/CSS Modules/tokens, Lucide whereiconsneeded, strictTS,noany. Tests
cover actual callbacks, disabledfutureoptions, malformednumericinput, mode
independentstrip theme and rendering scales/offsets with mockcanvas as needed.
No redundant implementationmirror tests. Check/lint/prettier/fullVitest per
CONVENTIONS. Browser captures wait for integrated T9b (record this slicealone
is not final UI acceptance). Commitfocused, cleanworktree, no merge/push.
Report ROOT ignoredledger task-9b-controls-report.md under300words with APIs
and keys so final integrator can reuse directly. Translation is a separate
Sol agent after integration; do not translate your own feature strings.

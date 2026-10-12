# Welcome discovery: actual browser acceptance

2026-10-02. Parent reviewed the implementation and used the actual UI through a
private agent-browser session, `fcux-start-20261002`, isolated release kernel
8861, Vite 1471 and task configuration. No personal desktop or original font was
used. This is the one additional small task the user explicitly authorized.

The original welcome screen offered file actions and recents but no direct
reference/discovery entry: [before](acceptance/p5ux/start-discovery/welcome-before.png).
The three added rows are visible in [English/light](acceptance/p5ux/start-discovery/welcome-light-1366.png)
and [Chinese/dark](acceptance/p5ux/start-discovery/welcome-dark-zh-1366.png),
both 1366×768. Open and New retain primary placement. At 1024×360, the card
starts with its [top accessible](acceptance/p5ux/start-discovery/welcome-short-360-top.png)
and scrolls to the [last keyboard-focused action](acceptance/p5ux/start-discovery/welcome-short-360-bottom.png).
This is a welcome-card check, not a whole-editor high-DPI or hardware assessment.

Actual action checks:

- Getting started opens the existing welcome help article. Escape returns focus
  to the Getting started button.
- Command Palette opens search. The parent reproduced a development StrictMode
  focus replay defect: the search appeared but focus returned to the underlying
  welcome button. The narrow lifecycle repair now focuses the search field;
  DOM inspection confirms the active combobox immediately after opening.
- Typing the Chinese query “键盘” finds Keyboard Shortcuts; Enter opens its
  existing settings dialog. Escape dismisses settings and restores the welcome
  command-palette button, with zero remaining dialogs.
- The direct Keyboard Shortcuts row also opens the existing dialog. Keyboard
  Tab/Enter reaches the reference actions; the last row retains visible focus
  in the short window.

[Search screenshot](acceptance/p5ux/start-discovery/discovery-command-search.png)
and [interaction recording](acceptance/p5ux/start-discovery/discovery-command-search.webm)
retain final behavior. No browser JavaScript errors were reported. An incorrect
help-key namespace observed during implementation was corrected before these
final captures; no raw translation key remains.

Source review confirms current registry/keymap bindings, explicit unbinding,
alias deduplication and platform formatting instead of hard-coded key labels.
Those variants, late registration and file-action preservation have focused
behavior tests in [implementation acceptance](p5ux-start-discovery-acceptance.md).
[Independent GPT-6.1 Sol translation](p5ux-start-discovery-translation-acceptance.md)
adds one key and leaves 3,945 matching keys. No other product backlog item is
claimed complete. Final broad checks and source-matched packages are linked from
[integrated acceptance](p5f-w3-integration-acceptance.md).

The private browser and both task servers were stopped after verification.
Owned browser/configuration/intermediate files were removed; retained screenshots
and the final interaction recording are linked above. Resource check passed
at 5.33 GiB of build artifacts; broad integration continues in its own scratch.

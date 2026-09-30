# Context help

Press **F1**, choose **Help → Context Help**, or find **Context Help** in the
command palette. Help opens inside FontCarver and works without an internet
connection. The keyboard shortcut editor can change its default F1 binding.

Help starts with the topic for the current workspace tab. If a dialog is open,
it shows that dialog's name and selects its relevant topic when available.
Hover over a help-enabled control before pressing F1 to read that control's
topic and descriptive hint. A focused input keeps its own context; moving the
pointer away or changing focus discards the previous hover target.
Controls that provide a descriptive hint also show that hint in the help view.
Help reads labels and descriptions; it does not copy the contents of input
fields into the help article.

Use **Help topic** to read another article. Topics cover the font overview,
outline and text editing, bitmap editing, Font Info, OpenType features, export,
preferences, shortcuts and common dialogs. Selecting a topic leaves your
document and current settings intact.

Use Tab and Shift+Tab to move between the topic selector and **Close Help**.
Press Escape or choose **Close Help** to return to the control you were using.
When you open help from another dialog, that dialog stays open with its settings
intact. Hover hints are also available on controls annotated with help topics;
their descriptions are exposed to screen readers.

## Adding contextual help to a control

UI contributors can spread `helpHint('features')` (or another `HelpTopic`) onto a
control. It supplies the translated topic summary as both a `title` and an
`aria-description`, and marks the control with `data-help-topic` for F1.
Give the control its own accessible label as usual. An explicit topic takes
priority over the dialog or tab topic. Existing `aria-describedby`,
`aria-description` or `title` descriptions can be shown as focused-control help.

The shell mounts `ContextHelpDialog` once and calls `registerHelpCommands` with
the other built-in command registrations. The shortcut dispatcher uses the
command's `allowInModal` and `allowInInput` flags, so the active user binding also
works inside dialogs. Ordinary editing commands remain subject to modal guards.

# HTTP deadline review

Root review: approved `727708e` and merged into P1. Read the complete transport
and test diff; independently reran the 10 transport tests successfully.
`text.shape` and `session.export` alone default to 120 seconds; all ordinary
calls remain at 30 seconds and explicit caller deadlines override both classes.
No retry or new dependency was introduced. The observed 76.779-second initial
Source Han shaping call supports the longer deadline; cancellation no longer
falsely implies the server never received the request. Integrated browser
acceptance after T9 remains pending.

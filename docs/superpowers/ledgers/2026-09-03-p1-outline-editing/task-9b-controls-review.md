# T9b controls review

Root reviewed presentation sources and tests at `ae4e158`, then fix `3973ed4`.
Initial findings were numeric Escape falling through synchronous blur and
committing the cancelled draft; future-phase codes in product labels; and
preview status/focus colors inherited from the outer theme. The follow-up
suppresses cancellation blur, adds a real focused userEvent Escape regression,
uses understandable unavailable wording and derives status/focus from the
strip's own foreground. Independent rerun: two files / seven tests passed.

Approved and merged into P1. TextBar remains controlled; preview consumes model
outline paths and font-unit advances rather than pretending to rasterize a
hinted compiled font. The backing canvas width cap is not unlimited scrolling;
T9 integration must supply bounded visible outlines and account for the bounded
preview window. Whole text editing integration, browser screenshots and separate
translation/QA remain pending. No generated bindings or dependencies added.

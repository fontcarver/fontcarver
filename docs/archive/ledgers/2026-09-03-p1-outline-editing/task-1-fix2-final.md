Status: Done; report updated, working tree clean, not pushed.  
Commit: `10f1f1f` — `fix(fw-geom): curve-preserving stroke ring resolution; robust remove-overlap fast path and fallback`  
Tests: fmt, clippy, debug tests (50), and release tests (52) all pass.  
Corpus: strict overlap failures 347→0; fallback 3583/3583; stroke failures 409→333.  
Curved successful round strokes: 385/3174→3250/3250 (100%).CODEX_DONE exit=0

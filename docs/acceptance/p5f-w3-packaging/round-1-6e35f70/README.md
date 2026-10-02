# First candidate round, superseded

These records describe revision `6e35f70fb530e71365c93f1d67ac8ff57b73b745`.
The checks available at build time passed, including actual APT installation.
Final review then found that the thin AppImage main ELF retained Tauri’s `DEB`
bundle marker. This round is not the final accepted candidate; the next build
adds the required unique `APP` marker and stricter verification.

Previous binary copies were removed after the corrected final round passed
independent review. These historical records remain for comparison; the current
`target/release/bundle/` contains only the final candidates.

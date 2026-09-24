### Task 12: Integration, performance gates and P1 acceptance

`docs/p1-acceptance.md` with: both exit-criterion workflows executed end to end (scripted via e2e + CLI), preservation proof (byte-identical raw tables, instructions kept/dropped as specified, shaping fingerprint equal), export timings (DejaVu < 5 s, Source Han Sans < 60 s release), drag budget (pointer→repaint ≤ 8 ms on a 300-point glyph measured in dev; `edit_apply` ≤ 4 ms at 20k glyphs release), UFO round-trip corpus results, desktop app smoke (open, edit, export via native dialogs is manual — record what was automated).
- [ ] **Step 1–2:** run, fix small defects in their own commits, write the record; commit `test: P1 acceptance and performance gates`.

---


# Wave 27 stage follow-ups: what remains

See `docs/PORT_NOTES/WAVE27_STAGE_FOLLOWUPS.md` for what was ported.

- Mushroom Kingdom with items: an item bumping a brick from below during
  its own collision while that proc draws (bounce spark) fails closed
  (`scene_stage/inishie1.rs::apply_bumps`). It is common (172/600
  explorer cases), so the app starting Mushroom Kingdom with items will
  stop there often. Fix: run the brick callback (fn_801FBEB8) from inside
  the item's map pass (mpCollEnd), before it_80276FC4's bounce; recorded
  witnesses already exist: `w27k_mk_eb4935276_p1` (diverges at 591) and
  `w27k_mk_ee6ff560c_p2` (982), both full-length retail recordings.
  Then lift the all-`?` brick guard (hatena.rs `break_hatena`), which both
  of those recordings break right there.
- Kongo Jungle: the barrel catching a Klaptrap (grKongo_801D8078,
  it_802E20D8, it_802E2330 with it_80270CD8's knockback): about 1 in 100
  explorer cases.
- Octarock and Ottosea (it_8026F6BC) were not started.
- Scratch scenarios left in the main checkout with traces (pass):
  `w27_mklift_e19f8579b_p0` (bone dump), `w27k_kjdbg_ef89b3e70_p0`
  (particle dump to 1735).

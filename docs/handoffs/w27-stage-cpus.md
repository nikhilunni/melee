# Handoff: CPU players on wave 27's stages

Done (docs/PORT_NOTES/CPU_PLAYERS.md, "Wave 27's stages"): boundaries
`w27s_start_{mk2,gb,brin,im,venom,mc,pf}_fox_cpu9fox4` (warm and cold
exact, registered in `harness/boundaries.toml` and `CPU_WITNESSES`), the
stage arms of ftCo_800A229C / ftCo_800A8210 / ftCo_800A2718 /
ftCo_800A1B38 (Venom), Great Bay's route table, the saved island order on
import, and `CPU_SUPPORTED` for Mushroom Kingdom II, Great Bay, Brinstar,
Mute City and Poké Floats.

Open:

- Icicle Mountain: `corpus_v3_w27s_start_im_fox_cpu9fox4_ef89b3e70_p0`
  diverges at tick 1197 (p1.cur_pos.y 14.776 retail, 14.636 port; frames
  set to 3239). `..._ec13743d5_p1` gates keys exact over 6001 ticks but its
  particle dump failed (disk full): re-record it (`record_many.py --jobs 1`)
  and triage the first. Then add IcicleMountain to `CPU_SUPPORTED`.
- Venom: `corpus_v3_w27s_start_venom_fox_cpu9fox4_edb2b114a_p0` diverges at
  tick 4238 (frames 5745; run `melee-sim triage`). `..._e9943b4ab_p2`
  failed to record (disk). `..._e75fb4a9a_p1` is exact and registered.
  Then add Venom to `CPU_SUPPORTED`.
- `..._mc_..._e89a89d0e_p2` failed to record (disk); one Mute City match
  and three Poké Floats matches are registered.
- Not run: the full sweep against `e7e3a37d.tsv` (the island-order restore
  in `initial_state/islands.rs` runs for every savestate import, so the
  sweep is the check that matters), `m5_gate` in the main checkout, and
  the replay batch (no shared fighter code changed).
- Venom's reflected Arwing laser (itarwinglaser.c:600, :641) stops some
  explorer cases; it belongs to the Venom item owner.
- 3-4 player CPUs off FD/Battlefield: not started.

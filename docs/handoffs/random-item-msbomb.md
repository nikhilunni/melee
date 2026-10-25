> **Branch** `worktree-agent-a1bd109548cb8a119`, WIP commit `28fa09e4` (not merged): the Motion-Sensor Bomb port, diverging at tick 247 of `start_fd_items_msbomb`.

# Handoff: Motion-Sensor Bomb (random item), WIP

State: the WIP commit ports itmsbomb.c (crates/it-msbomb) and the engine
pieces it needs; it is not admitted. `start_fd_items_msbomb` (warm and cold,
recorded in the main checkout; boundary not registered) diverges at tick 247:
the falling bomb's landing height (items.0.pos.y 1.932 retail, 1.546 port).

What is in the WIP:

- it-msbomb: fall (it_8028FF8C with the x0 tilt), rest, hold, throw/drop
  (the thrown box, xDCE b7 off), arming on any contact (it_80290314,
  it_80290238 with its 802902A0 fmadds), the armed line-following
  (it_80276934 via `lean_to_normal`), the touch callback and the blast
  (it_80290614). Unwitnessed branches are guarded with their C lines.
- melee-it: `Toucher` / `touched_by` / `touched` (xDCE b6, toucher,
  Item.touched, run as Item_8026A294's last case and cleared in
  Item_80269CC4), `stuck_to_line` (xDCE b3 in the physics proc's line speed),
  `lean_with_contact` (it_802765BC), `lean_to_normal`, `use_collision_box`
  (it_80275D5C with a kind box), `ItemScratch::MotionSensor`.
- melee-ft: an inert item hitbox touching a fighter's hitbox (clank.rs) or
  hurt capsules (damage.rs, ftcoll.c:2236-2293) marks the item.
- melee-lib: item-vs-item inert touches (itcoll.c:600-645) replace the old
  guards.
- hsd-archive: ItemModelDesc xC bits 6-7 (`spin_joint_depth`), parsed but not
  yet used (it is 0 for the bomb).

The open divergence: the bomb spins about Z while falling (xD3C -15 degrees a
frame, same as retail per the raw xD3C) with its root tilted by x0 = pi/2
about X, and the fixed ECB follows the spin (xDCE b7). Retail's ECB (raw item
bytes at +0x41C, CollData ecb) and the port's differ in shape from tick 244 on
at the same spin angle: at -60 degrees retail has top 1.732, bottom -1.732,
right 1.559, left -1.732; the port 1.5, -1.5, 1.247, -1.940. Both are
+-1.5 at tick 245. Next step: dump the retail ECB source (ecb_source params
and desired_ecb in the raw CollData) per tick and compare with
mpColl_80043754's stepping and the port's fixed-box rotation; check whether
the x0 tilt reaches the ECB through HSD_JObjGetRotationZ on a joint whose
rotation was set with HSD_JObjSetRotationX.

Recording names already used: start_fd_items_msbomb (and _cold).

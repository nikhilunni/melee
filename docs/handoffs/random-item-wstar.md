# Handoff: Warp Star (random item), item side only

State: `it-wstar` ports the fall and the spinning rest; `start_fd_items_wstar`
(warm and cold) is exact and registered. The pickup fails closed
(it_3F14_Logic29_PickedUp, and ftpickupitem_8009447C's WStar takeover in
melee-ft item_pickup.rs). Super/Poison Mushrooms were not started.

## Recorded, not yet passing

Eight explorer bridges from `start_fd_items_wstar` are recorded in the main
checkout (`corpus_v3_fd_items_wstar_{ef89b3e70_p0, e89a89d0e_p0,
edb2b114a_p0, e1502cb40_p2, ec13743d5_p0, e726cfdde_p1, ec3145eb3_p2,
e3e74affa_p1}`). Six stop at the pickup; two diverge earlier:

- `edb2b114a_p0` tick 494 and `ec3145eb3_p2` tick 463: two resting stars
  push apart one frame early. Cause: the Warp Star's ItemModelDesc xC bits
  6-7 (`spin_joint_depth`) are 3, so it_80274A64 / it_80274740 /
  it_80274990 spin, reset and read the joint three first-children below
  the root (rest rotation 0), not the root. The port spins the root, whose
  rotation Y also takes HSD_JObjSetFacingDirItem's pi/2 (xDC8 x19), so the
  accumulated angle rounds differently and the push box (it_80271A58)
  drifts. Fix: keep a separate spin-joint rotation when the depth is
  nonzero (initialised from that joint's desc rotation, reset with
  lb_8000B804, used by `spin`, `reset_spin`, `ecb_angle`, `come_to_rest`'s
  reset, `leave_hand`'s it_80274990 read) and apply it to that joint in
  `article_pose`. Barrels (Taru, TaruCann) have depth 1 with zero rest
  rotations and x19 off; resweep their witnesses. Monsters in the public+8
  bank have depths 2-3 too.

## The ride (ftCo_WarpStar.c), not started

- ftCo_800C4724 at LightGet's end: it_80294430 (model scale x8 *
  x2C4.y / x4 * ItemAttr x60, where x2C4.y is the ftData x50 push shape's
  half width; hitbox scale = warp_star_hitbox_scale), facing 0, 120-frame
  timer, intangible (ftColl_8007B6A0), take_dmg/death2 callbacks drop the
  star (Item_8026ABD8), and it_80294364: HSD_Randi over the seven flight
  paths other than the last one used (the global it_804D6D00), a sound,
  then the path's AnimJoint is added to the fighter's root JObj and
  animated (ftAnim_80070734, HSD_JObjAnim). The seven AnimJoints (ItCo.usd
  0x72584, 0x729d0, 0x731ac, 0x73744, 0x73d08, 0x74290, 0x74a70) animate
  only the root: rotation X/Y/Z and translation X/Y/Z, 43-85 frames.
  Precompute each path's per-frame root SRT at load (no tick allocation).
- WarpStarJump_Phys: self_vel = root translation delta; Anim ends at the
  blast zone top or the timer (ftCo_800C4A38): WarpStarFall with the
  star's ECB box (it_802944AC), cur_pos = (saved x, blast top), root
  rotation quaternion, star state 3 (it_802946B0).
- WarpStarFall: Fall with PlCo x694/x698 and the drift; landing
  (ftCo_800C4C60): JumpB, star blast (it_802947CC), efAsync 1030 with
  atan2f of the floor normal, a large quake, rumble.
- The held star rides at FtPart_TopN (ftpickupitem_800948A8), not the
  hand part.

Recording names used: start_fd_items_wstar (and _cold), the eight above.

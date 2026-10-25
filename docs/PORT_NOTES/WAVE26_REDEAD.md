# Wave 26: the ReDead and CaptureLeadead

The ReDead (`itleadead.c`, `it-monsters/src/redead.rs`) out of a crate,
with the fighter's side of its hold (`ftCo_800C7434.c`, `ftCo_800C7590.c`,
`melee-ft/src/fighter/capture_redead.rs`).

## Witnesses (all exact with particles, particle sites and bones)

| Scenario | Ticks | What retail does |
|---|---:|---|
| `w25_redead_fall_700_v01` | 700 | Falls (10), reels from a barrel's hit (11), settles, walks (2), lunges (4), holds Fox (5; 333 at 517) until the timer lets go (6; CaptureCut 229 at 638). |
| `w26_redead_mash_1500_v01` | 1500 | Fox mashes out (CaptureCut 535, ReDead 6); Fox's dash attack knocks it down (7, 8), it rises (9), settles, walks and turns (3 at 1202). |
| `w26_redead_death_v01` | 826 | Marth's dash attack hits the ReDead holding Fox (release, knockdown 7 at 533); a down smash kills it lying down (tumble 15 at 626, gone by 684). |

## Shape

- Fighter-to-item calls (it_802EAAEC, it_802EAE80, it_802EADD8) are fighter
  item requests the scene runs at once with the fighter's own state
  (`scene_items::redead_request`), timed as other item requests (a hitbox
  made past link 11 is placed at once).
- The hold's constraint (lb_8000C1C0 on the root) is `ItemCore::constrained_root`,
  refreshed from the fighter's TopN before the item's links 11, 13 and 14.
  Joint 1's `HSD_JObjSetTranslateZ` is `joint_translation_z`; an ANIM_UPDATE
  state change clears it (Item_80268D34's rest pose).
- it_802E8CD8's first step (letting a held fighter go) runs just before the
  item's damage callback (`release_redead_on_hit`), since it_802759DC needs
  the mutable map; the fighter's ftCo_800C7800 follows through the owner
  request `ReDeadLetGo`.

## Still guarded

- A staggered or killed ReDead flying at the screen (it_802EA334, one in
  it_804D6D40 +8 = 0.16 of deaths) and a staggered ReDead's death.
- A fighter's grab catching a ReDead (ftColl_8007BC90; the held and thrown
  states 13 and 14), and a ReDead gone while holding a fighter.
- it_802759DC's floor probe that leaves `y_out` unset.
- The right-facing birth is now plain code (it_8027C56C with the queried
  facing); no witness yet.

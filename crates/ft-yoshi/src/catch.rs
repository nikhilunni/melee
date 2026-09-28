//! Yoshi's tongue grab: fn_800D9CE8's FTKIND_YOSHI arm (ftCo_CatchPull.c:28-37).
use crate::init::Yoshi;
use melee_ft::fighter::assets::FighterAssets;
use melee_ft::fighter::Fighter;

/// fn_800D9CE8 (800D9D3C..800D9D9C), no fused sites: a catch inside the
/// attribute window first advances Catch by the frames it has spent there
/// (retail sets that as the animation rate and runs ftAnim_8006EBA4), then
/// starts CatchPull at the table's frame for that whole-frame offset.
pub(crate) fn catch_pull_start(f: &mut Fighter, assets: &FighterAssets, frame: f32) -> f32 {
    let attributes = &f.character.get::<Yoshi>().attributes;
    let [start, end] = attributes.catch_pull_window;
    if !(frame >= start && frame < end) {
        return frame;
    }
    let table = attributes.catch_pull_start_frames;
    // 800D9D5C fsubs.
    let offset = frame - start;
    // ftAnim_SetAnimRate: Catch never freezes (x2223_b0) at this point.
    f.core
        .animation
        .set_rate(&mut f.core.skeleton, offset, false);
    f.step_animation(assets);
    // 800D9D74 fctiwz, 800D9D8C lbz: unsigned byte to float.
    f32::from(table[gekko_math::msl::fctiwz(offset) as usize])
}

//! The grabs' tether article (Link's and Young Link's hookshot): the
//! FTKIND_LINK / FTKIND_CLINK arms of ftCo_Catch.c, ftCo_CatchPull.c and
//! ftCo_CatchWait.c, and the accessory callbacks the article installs
//! (Fighter_CallAcessoryCallbacks_8006C624). Samus's grapple beam takes the
//! same arms through its own catch hooks.
use super::{assets, Fighter};
use melee_mp::CollMap;

/// A character's tether article hooks.
#[derive(Clone, Copy)]
pub struct Tether {
    /// fn_800D8EC8 / fn_800D9228: the grab's frame before the common end
    /// check; true when it changed the motion.
    pub animate: fn(&mut Fighter, &assets::FighterAssets, &mut CollMap) -> assets::Result<bool>,
    /// fn_800D949C: Catch or CatchDash left the floor; the article goes.
    pub departed: fn(&mut Fighter),
    /// fn_800D9CE8's arm: a grab caught its victim through the article,
    /// which reels it in; the pull follows the article's joint.
    pub caught: fn(&mut Fighter),
    /// ftCo_CatchPull_Anim's arm: the pull ends when this is true (instead
    /// of the animation's end or the release flag).
    pub pull_done: fn(&Fighter) -> bool,
    /// ftCo_CatchWait_IASA's arm (it_802A7AAC) once a pummel or throw began.
    pub released: fn(&mut Fighter),
    /// accessory2_cb outside hitlag, accessory3_cb in it; the article's
    /// step may change the fighter's motion (the aerial tether's wall).
    pub accessory:
        fn(&mut Fighter, &assets::FighterAssets, &mut CollMap, bool) -> assets::Result<()>,
}

impl Fighter {
    /// ftCo_CatchWait_IASA's kind arm (it_802A7AAC), once a pummel or
    /// throw began.
    pub fn release_tether(&mut self) {
        if let Some(tether) = self.character.table().tether {
            (tether.released)(self);
        }
    }
    /// Fighter_CallAcessoryCallbacks_8006C624's accessory2 (accessory3 in
    /// hitlag) while a tether article is out; accessory1 follows in the scene.
    pub fn tether_accessory(
        &mut self,
        assets: &assets::FighterAssets,
        map: &mut CollMap,
    ) -> assets::Result<()> {
        if self.core.status.disabled || !self.core.tether_article {
            return Ok(());
        }
        let Some(tether) = self.character.table().tether else {
            return Ok(());
        };
        let in_hitlag = self.core.in_hitlag();
        (tether.accessory)(self, assets, map, in_hitlag)
    }
}

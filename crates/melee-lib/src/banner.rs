//! The match-start HUD banners (ifStatus_803F9628, if_2F6E.c): the countdown
//! whose end releases input, then GO, whose end enables the HUD and so the
//! match clock. Only their animation completion is gameplay; nothing renders.
use anyhow::Result;
use hsd_anim::{
    jobj::{JObjId, JObjTree},
    load::{attach_anim_joint, load_joint_tree},
};
use melee_ft::fighter::RetailTrig;
#[cfg(test)]
use std::path::Path;

/// ifStatus_803F9628's element index, which is also the ScInfCnt model.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BannerKind {
    /// gm_Scene_SuddenDeath_OnEnter: ifStatus_802F6EA4(1).
    SuddenDeathCountdown = 1,
    /// gm_Scene_Vs_OnEnter: ifStatus_802F6EA4(3).
    Countdown = 3,
    /// fn_8016B7F8: ifStatus_802F6EA4(4), ended by fn_8016B784.
    Go = 4,
}

#[derive(Clone)]
pub(crate) struct Banner {
    pub(crate) kind: BannerKind,
    /// x12.x2: the start callback has run.
    announced: bool,
    tree: JObjTree,
    root: JObjId,
}

impl Banner {
    /// ifall.c loads IfAll; this port's NTSC English scenes use IfAll.usd.
    #[cfg(test)]
    pub(crate) fn load(files: &Path, kind: BannerKind) -> Result<Self> {
        let archive = hsd_archive::Archive::parse(&std::fs::read(files.join("IfAll.usd"))?)?;
        Self::from_archive(&archive, kind)
    }

    /// ifStatus_802F6EA4 (802F6EA4): load the element's model, request frame
    /// zero and evaluate it once.
    pub(crate) fn from_archive(archive: &hsd_archive::Archive, kind: BannerKind) -> Result<Self> {
        let mut banner = Self::preload(archive, kind)?;
        banner.start();
        Ok(banner)
    }

    /// Load a banner that a later event starts, so the tick never allocates.
    pub(crate) fn preload(archive: &hsd_archive::Archive, kind: BannerKind) -> Result<Self> {
        Self::load_model(archive, kind)
    }

    /// ifStatus_802F6EA4's HSD_JObjReqAnimAll(0) and HSD_JObjAnimAll.
    pub(crate) fn start(&mut self) {
        self.tree.req_anim_all(self.root, 0.0);
        self.tree.anim_all::<RetailTrig>(self.root);
    }

    /// A banner already running at a savestate boundary: its AObjs share the
    /// saved current frame and flags. The request seeks the curves; the saved
    /// flags then replace its AOBJ_FIRST_PLAY, since the banner has already
    /// played past its first frame.
    pub(crate) fn resume(
        archive: &hsd_archive::Archive,
        kind: BannerKind,
        frame: f32,
        flags: u32,
    ) -> Result<Self> {
        let mut banner = Self::load_model(archive, kind)?;
        banner.announced = true;
        banner.tree.req_anim_all(banner.root, frame);
        let ids: Vec<_> = banner.tree.ids().collect();
        for id in ids {
            if let Some(aobj) = &mut banner.tree.get_mut(id).aobj {
                aobj.flags = flags;
            }
        }
        Ok(banner)
    }

    fn load_model(archive: &hsd_archive::Archive, kind: BannerKind) -> Result<Self> {
        let (joint, animation) = hsd_archive::desc::model::read_dynamic_model_animation(
            archive,
            "ScInfCnt_scene_models",
            kind as u32,
            0,
        )?;
        let (mut tree, root) = load_joint_tree(archive, &joint)?;
        attach_anim_joint(&mut tree, root, &animation, archive)?;
        Ok(Self {
            kind,
            announced: false,
            tree,
            root,
        })
    }

    /// `if_802F73C4`'s first step: the kind whose start callback runs now.
    pub(crate) fn announce(&mut self) -> Option<BannerKind> {
        (!std::mem::replace(&mut self.announced, true)).then_some(self.kind)
    }

    /// if_802F73C4 (s_link 0, p_link 14): animate, then lb_8000B09C.
    /// On completion fn_8016B7F8 -> ftLib_800868A4 releases all input freezes.
    pub(crate) fn tick(&mut self) -> bool {
        self.tree.anim_all::<RetailTrig>(self.root);
        let mut active = false;
        self.tree.walk_tree(self.root, &mut |id, _| {
            if let Some(aobj) = &self.tree.get(id).aobj {
                active |= aobj.flags & hsd_anim::aobj::AOBJ_NO_ANIM == 0;
            }
        });
        !active
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::BufRead;

    #[test]
    fn countdown_completion_matches_retail_input_freeze_flags() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let ledger = root.join("harness/traces/start_fd_fox.ledger600.raw.jsonl");
        let files = root.join("harness/roms/files");
        if !melee_test_support::require_files([&ledger, &files.join("IfAll.usd")]) {
            return;
        }
        let mut countdown = Banner::load(&files, BannerKind::Countdown).unwrap();
        let mut frozen = true;
        let mut release_tick = None;
        for (tick, line) in melee_test_support::trace::open(&ledger)
            .unwrap()
            .lines()
            .enumerate()
        {
            if tick > 0 && frozen && countdown.tick() {
                frozen = false;
                release_tick = Some(tick);
            }
            let row: serde_json::Value = serde_json::from_str(&line.unwrap()).unwrap();
            for fighter in row["fighters"].as_array().unwrap() {
                let bytes = fighter["bytes"].as_str().unwrap();
                let flags = u8::from_str_radix(&bytes[0x221d * 2..0x221d * 2 + 2], 16).unwrap();
                assert_eq!(frozen, flags & 8 != 0, "tick {tick}");
            }
        }
        assert_eq!(release_tick, Some(85));
    }
}

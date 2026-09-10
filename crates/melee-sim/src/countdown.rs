//! The normal Versus countdown's input-release event (no HUD rendering).
use anyhow::Result;
use hsd_anim::{
    jobj::{JObjId, JObjTree},
    load::{attach_anim_joint, load_joint_tree},
};
use melee_ft::fighter::RetailTrig;
use std::path::Path;

pub(crate) struct Countdown {
    tree: JObjTree,
    root: JObjId,
}

impl Countdown {
    /// gm_Scene_Vs_OnEnter -> ifStatus_802F6EA4(3), joint animation 0.
    /// ifall.c loads IfAll; this port's NTSC English scenes use IfAll.usd.
    pub(crate) fn load(files: &Path) -> Result<Self> {
        let archive = hsd_archive::Archive::parse(&std::fs::read(files.join("IfAll.usd"))?)?;
        let (joint, animation) = hsd_archive::desc::model::read_dynamic_model_animation(
            &archive,
            "ScInfCnt_scene_models",
            3,
            0,
        )?;
        let (mut tree, root) = load_joint_tree(&archive, &joint)?;
        attach_anim_joint(&mut tree, root, &animation, &archive)?;
        tree.req_anim_all(root, 0.0);
        tree.anim_all::<RetailTrig>(root);
        Ok(Self { tree, root })
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
    use std::io::{BufRead, BufReader};

    #[test]
    fn countdown_completion_matches_retail_input_freeze_flags() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let ledger = root.join("harness/traces/start_fd_fox.ledger600.raw.jsonl");
        let files = root.join("harness/roms/files");
        if !melee_test_support::require_files([&ledger, &files.join("IfAll.usd")]) {
            return;
        }
        let mut countdown = Countdown::load(&files).unwrap();
        let mut frozen = true;
        let mut release_tick = None;
        for (tick, line) in BufReader::new(std::fs::File::open(ledger).unwrap())
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

//! Held articles seek their authored pose at the simulation's current frame.
use super::*;
use melee_it::{ItemCore, ItemDispatch, ItemScratch};
use melee_types::ItemKind;

pub(super) struct HeldModel {
    pub owner: usize,
    pub kind: ItemKind,
    pub visible: bool,
    selected: usize,
    attachment: JObjId,
    rest: JObjTree,
    states: Vec<JObjTree>,
}
impl HeldModel {
    pub fn new(
        archive: &Archive,
        visual: &hsd_archive::desc::item_visual::ItemVisual,
        owner: usize,
        kind: ItemKind,
    ) -> Result<Self, PresentationError> {
        let (rest, root) =
            hsd_anim::load::load_joint_tree(archive, &visual.model).map_err(error)?;
        let attachment = rest
            .bone(root, visual.attachment_bone)
            .ok_or_else(|| error("held article attachment bone missing"))?;
        let mut states = Vec::new();
        for state in &visual.states {
            if state.shape.is_some() {
                return Err(error("held article shape animation unsupported"));
            }
            let mut tree = rest.clone();
            if let Some(anim) = &state.joint {
                hsd_anim::load::attach_anim_joint(&mut tree, root, anim, archive).map_err(error)?;
            }
            if let Some(anim) = &state.material {
                let material = hsd_anim::load::material_animation(archive, anim).map_err(error)?;
                tree.add_anim_all(root, None, Some(&material));
            }
            states.push(tree);
        }
        Ok(Self {
            owner,
            kind,
            visible: false,
            selected: 0,
            attachment,
            rest,
            states,
        })
    }
    pub fn tree(&self) -> &JObjTree {
        &self.states[self.selected]
    }
    pub fn capture(
        &mut self,
        item: Option<&ItemCore>,
        hand: hsd_types::Mtx,
    ) -> Result<(), PresentationError> {
        self.visible = item
            .is_some_and(|item| matches!(&item.scratch, ItemScratch::Held(s) if s.visibility == 1));
        let logic = crate::scene_items::SceneItems::logic(self.kind);
        let animation = item.map_or(0, |item| {
            logic.states[usize::from(item.motion)].animation_id
        });
        if animation < 0 {
            self.visible = false;
            return Ok(());
        }
        self.selected = animation as usize;
        let tree = self
            .states
            .get_mut(self.selected)
            .ok_or_else(|| error("held article animation missing"))?;
        for id in self.rest.ids() {
            let rest = self.rest.get(id);
            let node = tree.get_mut(id);
            node.flags = rest.flags;
            node.translate = rest.translate;
            node.rotate = rest.rotate;
            node.scale = rest.scale;
            node.mtx = rest.mtx;
            node.scl = rest.scl;
        }
        let Some(item) = item else {
            return Ok(());
        };
        tree.req_anim_all(JObjId(0), item.animation_frame);
        tree.anim_all::<melee_ft::fighter::RetailTrig>(JObjId(0));
        (logic.model_pose)(item, tree);
        // lb_8000C2F8 attaches position and orientation, not the owner's scale.
        // Keep article scale separate from the fighter's animated hand scale.
        let mut matrix = hand;
        for column in 0..3 {
            let length = gekko_math::msl::sqrtf(
                hand.0[0][column] * hand.0[0][column]
                    + hand.0[1][column] * hand.0[1][column]
                    + hand.0[2][column] * hand.0[2][column],
            );
            if length > 0.0 {
                for row in 0..3 {
                    matrix.0[row][column] *= logic.held_scale / length;
                }
            }
        }
        tree.get_mut(self.attachment).flags |= hsd_anim::jobj::JOBJ_USER_DEF_MTX;
        tree.copy_mtx(self.attachment, &matrix);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Buttons, Character, GameAssets, Inputs, MatchConfig, PlayerConfig, Port, Seed, Stage,
    };

    #[test]
    fn held_articles_follow_hands_and_ignore_capture_frequency() {
        let files =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../harness/roms/files");
        if !melee_test_support::require_files([files.join("PlCo.dat")]) {
            return;
        }
        let config = MatchConfig::versus(
            Stage::FinalDestination,
            [
                PlayerConfig::new(Port::P1, Character::Fox),
                PlayerConfig::new(Port::P2, Character::Marth),
            ],
        )
        .with_seed(Seed(42));
        let assets = GameAssets::load(files, &config).unwrap();
        let mut game = Match::new(&assets, config).unwrap();
        let mut frequent = Presentation::new(&game).unwrap();
        let mut sparse = Presentation::new(&game).unwrap();
        let mut saw_held = false;
        let mut saw_recoil = false;
        for tick in 0..600 {
            let mut input = Inputs::default();
            if tick >= 220 && tick % 60 < 30 {
                input.0[0].buttons = Buttons::B;
            }
            game.step(&input).unwrap();
            frequent.capture(&game).unwrap();
            if tick % 7 == 0 {
                sparse.capture(&game).unwrap();
                assert_eq!(frequent.visibility(), sparse.visibility(), "tick {tick}");
                for (index, (a, b)) in frequent
                    .matrices()
                    .iter()
                    .zip(sparse.matrices())
                    .enumerate()
                {
                    assert_eq!(a, b, "tick {tick}, matrix {index}");
                }
            }
            for index in 0..frequent.models.len() {
                let (previous, current) = frequent.models.split_at_mut(index);
                let ModelSource::Held(held) = &current[0].source else {
                    continue;
                };
                if !held.visible {
                    continue;
                }
                saw_held = true;
                let fighter = &game.engine.state().fighters[held.owner].0;
                let part = crate::scene_items::SceneItems::logic(held.kind)
                    .held_part
                    .unwrap();
                let bone = assets.inner.fighters[held.owner].parts.joint(part).unwrap();
                let joint = fighter
                    .skeleton
                    .bone(fighter.animation.root, usize::from(bone))
                    .unwrap();
                let hand = previous[held.owner].pose.matrix(joint);
                let attachment = current[0].pose.matrix(held.attachment);
                for row in 0..3 {
                    assert_eq!(attachment.0[row][3], hand.0[row][3]);
                }
                for item in game.engine.state().items.iter() {
                    if let ItemScratch::Held(state) = &item.scratch {
                        saw_recoil |= state.recoil_pose_frame > 0;
                    }
                }
            }
        }
        assert!(saw_held && saw_recoil);
        game.reset(Seed(7)).unwrap();
        frequent.capture(&game).unwrap();
        let fresh = Presentation::new(&game).unwrap();
        assert_eq!(frequent.visibility(), fresh.visibility());
        // Inactive article pose caches are intentionally invisible after reset.
        for (index, visible) in fresh.visibility().iter().enumerate() {
            if *visible {
                let offset = fresh.meshes[index].matrix_offset as usize;
                assert_eq!(frequent.matrices()[offset], fresh.matrices()[offset]);
            }
        }
    }
}

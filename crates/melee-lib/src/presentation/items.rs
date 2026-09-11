//! Animated articles seek their authored pose at the simulation's current frame.
use super::*;
use melee_it::{ItemCore, ItemDispatch};
use melee_types::ItemKind;

pub(super) struct ArticleModel {
    pub copy: usize,
    pub owner: usize,
    pub kind: ItemKind,
    pub visible: bool,
    selected: usize,
    attachment: JObjId,
    rest: JObjTree,
    states: Vec<JObjTree>,
}
impl ArticleModel {
    pub fn new(
        archive: &Archive,
        visual: &hsd_archive::desc::item_visual::ItemVisual,
        owner: usize,
        kind: ItemKind,
        copy: usize,
    ) -> Result<Self, PresentationError> {
        let (rest, root) =
            hsd_anim::load::load_joint_tree(archive, &visual.model).map_err(error)?;
        let attachment = rest
            .bone(root, visual.attachment_bone)
            .ok_or_else(|| error("article attachment bone missing"))?;
        let mut states = Vec::new();
        for state in visual
            .states
            .iter()
            .take(if copy == 0 { visual.states.len() } else { 0 })
        {
            if let Some(shape) = &state.shape {
                if shape.has_animation(archive).map_err(error)? {
                    return Err(error("held article shape animation unsupported"));
                }
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
        if copy != 0 {
            states.push(rest.clone());
        }
        Ok(Self {
            copy,
            owner,
            kind,
            visible: false,
            selected: 0,
            attachment,
            rest,
            states,
        })
    }
    pub fn texture_states(
        &self,
        joint: JObjId,
        display: usize,
        texture: usize,
    ) -> impl Iterator<Item = &hsd_anim::tobj::TObj> {
        self.states.iter().filter_map(move |tree| {
            tree.dobj(joint)?
                .get(display)?
                .mobj
                .as_ref()?
                .textures
                .get(texture)
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
        self.visible = false;
        let logic = crate::scene_items::SceneItems::logic(self.kind);
        let animation = item.map_or(0, |item| {
            logic.states[usize::from(item.motion)].animation_id
        });
        if animation < 0 {
            self.visible = false;
            return Ok(());
        }
        self.selected = if self.copy == 0 {
            animation as usize
        } else {
            0
        };
        let tree = self
            .states
            .get_mut(self.selected)
            .ok_or_else(|| error("article animation missing"))?;
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
        self.visible = (logic.model_pose)(item, tree, self.copy);
        if logic.held_part.is_none() {
            return Ok(());
        }
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
    use melee_it::ItemScratch;

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
                let ModelSource::Article(held) = &current[0].source else {
                    continue;
                };
                if !held.visible
                    || crate::scene_items::SceneItems::logic(held.kind)
                        .held_part
                        .is_none()
                {
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
    #[test]
    fn illusion_afterimages_follow_both_history_entries_and_clone_cleanly() {
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
        let mut view = Presentation::new(&game).unwrap();
        let mut sparse = Presentation::new(&game).unwrap();
        let mut saw_secondary = false;
        let mut saw_trailing_only = false;
        for tick in 0..400 {
            let mut inputs = Inputs::default();
            if (220..225).contains(&tick) {
                inputs.0[0] = crate::ControllerState::from_origin_adjusted(
                    Buttons::B,
                    [80, 0],
                    [0, 0],
                    [0, 0],
                );
            }
            game.step(&inputs).unwrap();
            view.capture(&game).unwrap();
            if tick % 7 == 0 {
                sparse.capture(&game).unwrap();
                assert_eq!(view.visibility(), sparse.visibility());
                for (index, (a, b)) in view.matrices().iter().zip(sparse.matrices()).enumerate() {
                    assert_eq!(a, b, "tick {tick}, matrix {index}");
                }
            }
            for item in game.engine.state().items.iter() {
                let ItemScratch::Afterimage(state) = &item.scratch else {
                    continue;
                };
                if !state.secondary_visible {
                    continue;
                }
                saw_secondary = true;
                saw_trailing_only |= item.motion == 2;
                let model = view.models.iter_mut().find(|m| matches!(&m.source,ModelSource::Article(a) if a.kind==item.kind && a.copy==1 && a.visible)).unwrap();
                let pose = model.pose.matrix(JObjId(0));
                assert_eq!(
                    [pose.0[0][3], pose.0[1][3], pose.0[2][3]],
                    [
                        state.secondary_position.x,
                        state.secondary_position.y,
                        state.secondary_position.z
                    ]
                );
                let cloned = game.clone();
                let mut cloned_view = Presentation::new(&cloned).unwrap();
                cloned_view.capture(&cloned).unwrap();
                assert_eq!(view.visibility(), cloned_view.visibility());
                for (mesh, visible) in view.meshes.iter().zip(view.visibility()) {
                    if *visible {
                        assert_eq!(
                            view.matrices()[mesh.matrix_offset as usize],
                            cloned_view.matrices()[mesh.matrix_offset as usize]
                        );
                    }
                }
            }
        }
        assert!(saw_secondary && saw_trailing_only);
    }
}

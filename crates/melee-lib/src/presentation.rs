//! Optional renderer-independent visual resources and reusable pose capture.
//! Construction decodes assets; capture only reads a compatible healthy match.
use crate::{Match, MatchStatus, PlayerConfig};
mod effects;
mod fighters;
mod items;
mod lighting;
pub use effects::EffectDraw;
use melee_it::ItemDispatch;
mod materials;
mod shadows;
mod sprites;
mod stage;
mod view;
pub use stage::Fog;
pub use view::ViewCamera;
#[cfg(test)]
mod stage_tests;
use hsd_anim::jobj::{JObjId, JObjTree, MatrixPose, JOBJ_HIDDEN};
pub use hsd_archive::visual::{PixelState, Texture, TextureCombiner, TextureLayer, Vertex};
use hsd_archive::{
    desc::JObjDesc,
    visual::{MatrixBinding, Polygon, TextureDecoder},
    Archive,
};
pub use lighting::DirectionalLight;
pub use sprites::{Sprite, SpriteShape};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Debug)]
pub struct PresentationError(String);
impl std::fmt::Display for PresentationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for PresentationError {}
fn error(e: impl std::fmt::Display) -> PresentationError {
    PresentationError(e.to_string())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FaceCulling {
    None,
    Front,
    Back,
    Both,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Billboard {
    None,
    ViewPlane,
    ViewPoint,
}
pub struct Mesh {
    /// Camera-facing geometry, evaluated by the consumer without changing bones.
    pub billboard: Billboard,
    /// Fighter slot whose geometry casts a planar floor shadow.
    pub shadow_owner: Option<usize>,
    /// Foreground stage geometry may receive floor shadows.
    pub shadow_receiver: bool,
    pub culling: FaceCulling,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub material: Arc<Material>,
    /// The vertices' local matrix selectors are relative to this offset.
    pub matrix_offset: u32,
    /// Opaque group selecting the active world transforms for this mesh.
    pub instance_group: usize,
    /// Stage background geometry is composited before the world, without depth writes.
    pub background: bool,
    /// Drawn after other opaque geometry but before fighters' opaque meshes,
    /// without depth writes: Mr. Game & Watch's outline
    /// (ftDrawCommon_80080E18_inline0).
    pub underlay: bool,
}
/// Renderer-independent material values. Meshes retain authored initial values;
/// `Presentation::materials` supplies the current values without reparsing bytes.
#[derive(Clone)]
pub struct Material {
    /// Post-texture color overlay; alpha is its interpolation weight.
    pub overlay: [f32; 4],
    /// Image variants uploaded once; each texture selects an entry from its bank.
    pub texture_banks: Vec<Arc<[Arc<Texture>]>>,
    pub ambient: [f32; 3],
    pub specular: [f32; 3],
    pub shininess: f32,
    pub textures: Vec<TextureLayer>,
    pub diffuse: [f32; 4],
    pub render_mode: u32,
    pub pixel: PixelState,
}
enum Binding {
    Rigid(JObjId),
    Envelope(Vec<(JObjId, f32)>),
}
struct Part {
    model: usize,
    owner: JObjId,
    display: usize,
    bindings: Vec<Binding>,
}
struct Model {
    pose: MatrixPose,
    source: ModelSource,
    instance_group: usize,
    images: hsd_archive::visual::DecodedImages,
}

enum ModelSource {
    Fighter(usize),
    Effect(u32),
    Article(items::ArticleModel),
    /// A Ground model by scheduler key; `form` selects a Pokemon Stadium form
    /// archive. The template poses a model the simulation has not created yet.
    Stage {
        key: u8,
        map: u8,
        form: Option<usize>,
        template: Option<Box<melee_gr::last::animation::BackgroundAnimation>>,
    },
    Item(items::ItemPose),
    /// An animated item whose model lives in the stage archive; its
    /// `owner` is a slot among the live items of its kind.
    StageItem(items::ArticleModel),
}
impl ModelSource {
    fn tree<'a>(&'a self, game: &'a Match) -> &'a JObjTree {
        match self {
            Self::Article(held) | Self::StageItem(held) => held.tree(),
            Self::Effect(descriptor) => {
                game.assets
                    .inner
                    .effect_resources
                    .visual_models()
                    .find(|model| model.descriptor == *descriptor)
                    .expect("prepared effect model")
                    .tree
            }
            Self::Fighter(slot) => &game.engine.state().fighters[*slot].0.skeleton,
            Self::Stage { key, template, .. } => {
                self::stage::animation(game, *key, template).pose_tree()
            }
            Self::Item(pose) => pose.tree(),
        }
    }
}
/// GPU-independent scene data. Immutable meshes are decoded once; matrices and
/// visibility are refreshed in place. A renderer may upload meshes once and
/// copy only the matrix/visibility changes each frame.
pub struct Presentation {
    shadow_floors: [[f32; 4]; 2],
    effects: effects::Effects,
    background_color: [u8; 3],
    lighting: lighting::Lighting,
    sprites: sprites::Sprites,
    assets: Arc<crate::assets::Assets>,
    players: [PlayerConfig; 2],
    meshes: Vec<Mesh>,
    materials: Vec<Material>,
    parts: Vec<Part>,
    models: Vec<Model>,
    matrices: Vec<[[f32; 4]; 4]>,
    visible: Vec<bool>,
    instances: Vec<[[f32; 4]; 4]>,
    instance_ranges: Vec<std::ops::Range<u32>>,
    camera_targets: [Option<[f32; 2]>; 2],
    view: ViewCamera,
    fog_desc: Option<melee_gr::desc::FogDesc>,
    fog: Option<Fog>,
    /// Per fighter slot: model-part groups and draw gates.
    fighter_parts: Vec<fighters::FighterParts>,
}
impl Presentation {
    pub fn new(game: &Match) -> Result<Self, PresentationError> {
        if game.status() == MatchStatus::Faulted {
            return Err(error("faulted match"));
        }
        let mut result = Self {
            shadow_floors: shadows::floors(game),
            effects: effects::Effects::default(),
            background_color: game.assets.inner.stage_desc.initial_fog,
            lighting: lighting::Lighting::new(game)?,
            sprites: sprites::Sprites::new(game)?,
            assets: Arc::clone(&game.assets.inner),
            players: game.config.players.clone(),
            meshes: Vec::new(),
            materials: Vec::new(),
            parts: Vec::new(),
            models: Vec::new(),
            matrices: Vec::new(),
            visible: Vec::new(),
            camera_targets: [None; 2],
            view: ViewCamera::capture(game),
            fog_desc: stage::fog_desc(&game.assets.inner)?,
            fog: None,
            fighter_parts: Vec::new(),
            instances: vec![[
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ]],
            instance_ranges: std::iter::once(0..1).collect(),
        };
        let assets = Arc::clone(&result.assets);
        // Every fighter's model, a player's partner (Nana) included.
        for slot in 0..game.engine.state().fighters.len() {
            let character = &assets.characters[slot];
            let costume = game.engine.state().fighters[slot].0.player.costume;
            let archive = character.costume(costume);
            let desc = character.model_desc(costume);
            let tree = &game.engine.state().fighters[slot].0.skeleton;
            result.add_model(archive, &desc, tree, ModelSource::Fighter(slot))?;
            let parts = fighters::FighterParts::new(character, costume, &desc, tree)?;
            let model = result.models.len() - 1;
            for (mesh, part) in result.meshes.iter_mut().zip(&result.parts) {
                if part.model == model && parts.is_outline(tree.get(part.owner).id, part.display) {
                    let mut material = (*mesh.material).clone();
                    material.pixel.depth_write = false;
                    mesh.material = Arc::new(material);
                    mesh.underlay = true;
                }
            }
            result.fighter_parts.push(parts);
        }
        result.add_stage_models(game)?;
        for (kind, archive, _) in assets.items.visual_models() {
            let visual = &assets.items.get(kind).visual;
            // Item_80267978 (item.c:738-745): an article without a model
            // (Peach's bomber blast, Bowser's flame, Pikachu's thunder...)
            // gets a bare identity JObj, which displays nothing.
            if visual
                .model
                .descendants()
                .iter()
                .all(|d| d.u.dobj().is_none())
            {
                continue;
            }
            if crate::scene_items::SceneItems::logic(kind).model_copies > 0 {
                // One set per fighter slot: Nana's articles are her own.
                for owner in 0..game.engine.state().fighters.len() {
                    for copy in 0..crate::scene_items::SceneItems::logic(kind).model_copies {
                        let held = items::ArticleModel::new(archive, visual, owner, kind, copy)?;
                        let tree = held.tree().clone();
                        result.add_model(
                            archive,
                            &visual.model,
                            &tree,
                            ModelSource::Article(held),
                        )?;
                    }
                }
                continue;
            }
            let pose = items::ItemPose::new(archive, visual, kind)?;
            let tree = pose.tree().clone();
            result.add_model(archive, &visual.model, &tree, ModelSource::Item(pose))?;
        }
        result.add_stage_items()?;
        for model in assets.effect_resources.visual_models() {
            if let Some(shape) = &model.definition.shape {
                if shape
                    .has_animation(effects::archive(&assets, model.bank))
                    .map_err(error)?
                {
                    return Err(error("animated effect shape unsupported"));
                }
            }
            if model
                .definition
                .model
                .descendants()
                .iter()
                .all(|d| d.u.dobj().is_none())
            {
                continue;
            }
            result.add_model(
                effects::archive(&assets, model.bank),
                &model.definition.model,
                model.tree,
                ModelSource::Effect(model.descriptor),
            )?;
        }
        result.prepare_materials(game)?;
        result.effects = effects::Effects::new(&result);
        result.capture(game)?;
        Ok(result)
    }
    /// World positions of fighters with remaining stocks, for caller-owned framing.
    /// Current stage clear color in GX encoded RGB.
    pub fn background_color(&self) -> [u8; 3] {
        self.background_color
    }
    /// Live floor segments [left x, left y, right x, right y], one per fighter.
    /// A reversed x interval denotes no floor beneath that fighter.
    pub fn shadow_floors(&self) -> &[[f32; 4]; 2] {
        &self.shadow_floors
    }
    pub fn camera_targets(&self) -> &[Option<[f32; 2]>; 2] {
        &self.camera_targets
    }
    /// The stage fog for the captured tick, if any.
    pub fn fog(&self) -> Option<Fog> {
        self.fog
    }
    /// The retail main camera for the captured tick.
    pub fn view_camera(&self) -> &ViewCamera {
        &self.view
    }
    pub fn sprites(&self) -> &[Sprite] {
        &self.sprites.live
    }
    pub fn ambient_light(&self) -> [f32; 3] {
        self.lighting.ambient
    }
    pub fn directional_lights(&self) -> &[DirectionalLight] {
        &self.lighting.lights
    }
    pub fn sprite_capacity(&self) -> usize {
        self.sprites.live.capacity()
    }
    pub fn sprite_textures(&self) -> &[Arc<Texture>] {
        &self.sprites.textures
    }
    pub fn meshes(&self) -> &[Mesh] {
        &self.meshes
    }
    /// Current material values, indexed in the same order as meshes.
    pub fn materials(&self) -> &[Material] {
        &self.materials
    }
    pub fn instances(&self) -> &[[[f32; 4]; 4]] {
        &self.instances
    }
    pub fn instance_range(&self, group: usize) -> std::ops::Range<u32> {
        self.instance_ranges[group].clone()
    }
    pub fn matrices(&self) -> &[[[f32; 4]; 4]] {
        &self.matrices
    }
    pub fn visibility(&self) -> &[bool] {
        &self.visible
    }
    pub fn capture(&mut self, game: &Match) -> Result<(), PresentationError> {
        if game.status() == MatchStatus::Faulted {
            return Err(error("faulted match"));
        }
        if !Arc::ptr_eq(&self.assets, &game.assets.inner) || self.players != game.config.players {
            return Err(error(
                "presentation belongs to different match assets or players",
            ));
        }
        for (player, target) in self.camera_targets.iter_mut().enumerate() {
            let fighter = &game.engine.state().player_fighter(player).0;
            *target = (fighter.player.stocks > 0)
                .then_some([fighter.physics.position.x, fighter.physics.position.y]);
        }
        if let crate::scene_stage::SceneStage::FinalDestination(stage) = &game.engine.state().stage
        {
            self.background_color = stage.ground.fog;
        }

        self.view = ViewCamera::capture(game);
        self.fog = stage::fog(game, &self.fog_desc);
        self.shadow_floors = shadows::floors(game);
        self.sprites.capture(game)?;
        self.lighting.capture(game.tick());
        for index in 0..self.models.len() {
            let (previous, remaining) = self.models.split_at_mut(index);
            let model = &mut remaining[0];
            if let ModelSource::Article(held) = &mut model.source {
                let fighter = &game.engine.state().fighters[held.owner].0;
                let item = game.engine.state().items.iter().find(|item| {
                    item.kind == held.kind
                        && item.owner == Some(fighter.player.id)
                        && item.owner_secondary == fighter.player.secondary
                        && !item.destroyed
                });
                let hand = if item.is_some()
                    && crate::scene_items::SceneItems::logic(held.kind)
                        .held_part
                        .is_some()
                {
                    let part = crate::scene_items::SceneItems::logic(held.kind)
                        .held_part
                        .unwrap();
                    let bone = self.assets.fighters[held.owner]
                        .parts
                        .joint(part)
                        .ok_or_else(|| error("held article owner part missing"))?;
                    let joint = fighter
                        .skeleton
                        .bone(fighter.animation.root, usize::from(bone))
                        .ok_or_else(|| error("held article owner joint missing"))?;
                    previous[held.owner].pose.matrix(joint)
                } else {
                    hsd_types::Mtx::default()
                };
                held.capture(item, hand)?;
                // ftLib_800868D4: the item in hand hides with its invisible
                // or hidden owner, or while a subaction hides it (x221E_b3).
                if item.is_some_and(|item| {
                    fighter
                        .held_item
                        .as_ref()
                        .is_some_and(|h| h.item == item.id)
                }) && (fighter.effect_state.invisible
                    || fighter.commands.fighter_hidden
                    || fighter.commands.held_item_hidden)
                {
                    held.visible = false;
                }
            }
            if let ModelSource::StageItem(held) = &mut model.source {
                stage::capture_item(game, held)?;
            }
            if let ModelSource::Item(pose) = &mut model.source {
                let kind = pose.kind;
                let live = |item: &&melee_it::ItemCore| item.kind == kind && !item.destroyed;
                let items = &game.engine.state().items;
                pose.capture(items.iter().find(live));
                let start = self.instance_ranges[model.instance_group].start;
                let mut count = 0;
                for item in items.iter().filter(live) {
                    let mut matrix = hsd_types::Mtx::default();
                    hsd_anim::mtx::hsd_mtx_srt(
                        &mut matrix,
                        &item.model_scale,
                        &item.rotation,
                        &item.position,
                        None,
                    );
                    self.instances[start as usize + count] = columns(matrix);
                    count += 1;
                }
                self.instance_ranges[model.instance_group] = start..start + count as u32;
            }
            let tree = model.source.tree(game);
            if !model.pose.refresh(tree) {
                return Err(error("presentation skeleton shape changed"));
            }
        }
        for (parts, fighter) in self
            .fighter_parts
            .iter_mut()
            .zip(&game.engine.state().fighters)
        {
            parts.capture(&fighter.0, game.tick().0);
        }
        self.capture_materials(game)?;
        for (slot, fighter) in game.engine.state().fighters.iter().enumerate() {
            let f = &fighter.0;
            if f.shield.active && f.shield.hit.radius > 0.0 {
                let volume = &f.shield.hit;
                let joint = f
                    .skeleton
                    .bone(f.animation.root, volume.bone)
                    .ok_or_else(|| error("shield joint missing"))?;
                let matrix = self.models[slot].pose.matrix(joint);
                let mut position = volume.position;
                if !volume.position_cached {
                    hsd_anim::mtx::mtx_mult_vec(&matrix, &volume.offset, &mut position);
                }
                // The collision radius is local to the shield bone. Its pose
                // carries health, light-shield and fighter scale. Project the
                // transformed sphere onto the presentation's world XY plane.
                let half_size = std::array::from_fn(|axis| {
                    let row = matrix.0[axis];
                    volume.radius
                        * gekko_math::msl::sqrtf(
                            row[0] * row[0] + row[1] * row[1] + row[2] * row[2],
                        )
                });
                self.sprites.push(Sprite {
                    previous_position: [position.x, position.y, position.z],
                    trail_alpha: 1.0,
                    alpha_compare: [0; 2],
                    alpha_mode: 0x3f,
                    position: [position.x, position.y, position.z],
                    half_size,
                    rotation: 0.0,
                    color: [255, 255, 255, f.shield.alpha],
                    environment: [0; 4],
                    texture: 0,
                    flags: 0,
                    shape: SpriteShape::Shield {
                        port: crate::Port::from_index(f.player.id),
                    },
                })?;
            }
        }
        for (i, part) in self.parts.iter().enumerate() {
            let model = &mut self.models[part.model];
            let tree = model.source.tree(game);
            let mut ancestor = Some(part.owner);
            let mut hidden = match &model.source {
                ModelSource::Stage { key, .. } => !stage::live(game, *key),
                ModelSource::Article(held) | ModelSource::StageItem(held) => !held.visible,
                ModelSource::Fighter(slot) => {
                    self.fighter_parts[*slot].hides(tree.get(part.owner).id, part.display)
                }
                ModelSource::Effect(_) => true,
                _ => false,
            };
            while let Some(id) = ancestor {
                hidden |= tree.get(id).flags & JOBJ_HIDDEN != 0;
                if let ModelSource::Stage { key, .. } = &model.source {
                    hidden |= stage::hides_joint(game, *key, id);
                }
                ancestor = tree.parent(id);
            }
            self.visible[i] = self.meshes[i].culling != FaceCulling::Both
                && !self.instance_ranges[model.instance_group].is_empty()
                && !hidden
                && tree
                    .dobj(part.owner)
                    .and_then(|ds| ds.get(part.display))
                    .is_none_or(|d| d.flags & 1 == 0);
            for (j, binding) in part.bindings.iter().enumerate() {
                let blended =
                    match binding {
                        Binding::Rigid(joint) => model.pose.matrix(*joint),
                        Binding::Envelope(weights) => model
                            .pose
                            .envelope_matrix(part.owner, weights)
                            .ok_or_else(|| error("incomplete visual envelope data"))?,
                    };
                self.matrices[self.meshes[i].matrix_offset as usize + j] = columns(blended);
            }
        }
        self.capture_effects(game)?;
        Ok(())
    }
    fn add_model(
        &mut self,
        archive: &Archive,
        desc: &JObjDesc,
        tree: &JObjTree,
        source: ModelSource,
    ) -> Result<(), PresentationError> {
        let index = self.models.len();
        let ids: BTreeMap<_, _> = tree.ids().map(|id| (tree.get(id).id, id)).collect();
        let instance_group = if matches!(source, ModelSource::Item(_)) {
            let index = self.instance_ranges.len();
            let start = self.instances.len() as u32;
            self.instances.resize(
                self.instances.len() + melee_it::ITEM_CAPACITY,
                [[0.0; 4]; 4],
            );
            self.instance_ranges.push(start..start);
            index
        } else {
            0
        };
        self.models.push(Model {
            pose: MatrixPose::new(tree),
            source,
            instance_group,
            images: BTreeMap::new(),
        });
        let mut decoder = TextureDecoder::new(archive);
        self.add_joints(index, archive, desc, &ids, &mut decoder)?;
        self.models[index].images = decoder.into_images();
        Ok(())
    }
    fn add_joints(
        &mut self,
        model: usize,
        archive: &Archive,
        joint: &JObjDesc,
        ids: &BTreeMap<u32, JObjId>,
        decoder: &mut TextureDecoder<'_>,
    ) -> Result<(), PresentationError> {
        let owner = *ids
            .get(&joint.offset)
            .ok_or_else(|| error("visual joint absent from simulation skeleton"))?;
        let mut next = joint.u.dobj();
        let mut display = 0;
        while let Some(dobj) = next {
            let mut material = Material {
                overlay: [0.0; 4],
                texture_banks: Vec::new(),
                ambient: [1.0; 3],
                specular: [0.0; 3],
                shininess: 0.0,
                textures: Vec::new(),
                diffuse: [1.0; 4],
                render_mode: 0,
                pixel: PixelState::from_render_mode(0),
            };
            if let Some(desc) = &dobj.mobj {
                material.textures = decoder.read_chain(desc.texdesc).map_err(error)?;
                material.render_mode = desc.rendermode;
                material.pixel = match desc.pedesc {
                    Some(offset) => PixelState::read(archive, offset).map_err(error)?,
                    None => PixelState::from_render_mode(desc.rendermode),
                };
                if let Some(mat) = &desc.mat {
                    material.ambient =
                        [mat.ambient.r, mat.ambient.g, mat.ambient.b].map(|v| f32::from(v) / 255.0);
                    material.specular = [mat.specular.r, mat.specular.g, mat.specular.b]
                        .map(|v| f32::from(v) / 255.0);
                    material.shininess = mat.shininess;
                    material.diffuse = [
                        f32::from(mat.diffuse.r) / 255.0,
                        f32::from(mat.diffuse.g) / 255.0,
                        f32::from(mat.diffuse.b) / 255.0,
                        mat.alpha,
                    ];
                }
            }
            let material = Arc::new(material);
            for Polygon {
                vertices,
                indices,
                matrices,
                flags,
            } in hsd_archive::visual::read_polygons(archive, dobj.pobjdesc).map_err(error)?
            {
                let (vertices, indices) = if indices.is_empty() {
                    stage::point_quads(&vertices)
                } else {
                    (vertices, indices)
                };
                let resolve = |id| {
                    ids.get(&id)
                        .copied()
                        .ok_or_else(|| error("unresolved visual skinning joint"))
                };
                let mut bindings = Vec::new();
                for binding in matrices {
                    bindings.push(match binding {
                        MatrixBinding::Owner => Binding::Rigid(owner),
                        MatrixBinding::Joint(id) => Binding::Rigid(resolve(id)?),
                        MatrixBinding::Envelope(weights) => Binding::Envelope(
                            weights
                                .into_iter()
                                .map(|w| Ok((resolve(w.joint)?, w.weight)))
                                .collect::<Result<_, PresentationError>>()?,
                        ),
                    });
                }
                let matrix_offset = self.matrices.len() as u32;
                self.matrices
                    .resize(self.matrices.len() + bindings.len(), [[0.0; 4]; 4]);
                let culling = match flags >> 14 {
                    1 => FaceCulling::Front,
                    2 => FaceCulling::Back,
                    3 => FaceCulling::Both,
                    _ => FaceCulling::None,
                };
                self.meshes.push(Mesh {
                    billboard: if joint.flags & hsd_anim::jobj::JOBJ_BILLBOARD_FIELD
                        == hsd_anim::jobj::JOBJ_BILLBOARD
                    {
                        if joint.flags & hsd_anim::jobj::JOBJ_PBILLBOARD != 0 {
                            Billboard::ViewPoint
                        } else {
                            Billboard::ViewPlane
                        }
                    } else {
                        Billboard::None
                    },
                    shadow_owner: match &self.models[model].source {
                        ModelSource::Fighter(slot) => Some(*slot),
                        ModelSource::Article(article)
                            if crate::scene_items::SceneItems::logic(article.kind)
                                .held_part
                                .is_some() =>
                        {
                            Some(article.owner)
                        }
                        _ => None,
                    },
                    shadow_receiver: matches!(
                        self.models[model].source,
                        ModelSource::Stage { map, .. }
                            if !stage::is_background(self.assets.stage_desc.kind, map)
                    ),
                    culling,
                    vertices,
                    indices,
                    material: Arc::clone(&material),
                    matrix_offset,
                    instance_group: self.models[model].instance_group,
                    background: matches!(
                        self.models[model].source,
                        ModelSource::Stage { map, .. }
                            if stage::is_background(self.assets.stage_desc.kind, map)
                    ),
                    underlay: false,
                });
                self.parts.push(Part {
                    model,
                    owner,
                    display,
                    bindings,
                });
                self.visible.push(true);
            }
            next = dobj.next.as_deref();
            display += 1;
        }
        for subtree in [joint.child.as_deref(), joint.next.as_deref()]
            .into_iter()
            .flatten()
        {
            self.add_joints(model, archive, subtree, ids, decoder)?;
        }
        Ok(())
    }
}

fn columns(matrix: hsd_types::Mtx) -> [[f32; 4]; 4] {
    std::array::from_fn(|col| {
        std::array::from_fn(|row| {
            if row < 3 {
                matrix.0[row][col]
            } else if col == 3 {
                1.0
            } else {
                0.0
            }
        })
    })
}

//! Read-only, tick-indexed evaluation of the stage's authored light set.
//! Uses the existing AObj/FObj interpreter; capture frequency is not a clock.
use super::{error, PresentationError};
use crate::Match;
use hsd_anim::aobj::{AObj, AObjEndCallback, AOBJ_LOOP};
use hsd_archive::desc::light::LightDesc;

/// One active stage light. Infinite lights carry the vector toward the light
/// in `direction`; point lights carry their world position there and set
/// `distance_attenuation`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DirectionalLight {
    /// World-space vector toward the light (infinite), normalized by the
    /// renderer; the light's world position for a point light.
    pub direction: [f32; 3],
    pub color: [f32; 3],
    pub diffuse: bool,
    pub specular: bool,
    /// Point lights only: `GXInitLightDistAttn`'s k0, k1, k2; attenuation is
    /// `1 / (k0 + k1 d + k2 d^2)` at distance `d` from the light.
    pub distance_attenuation: Option<[f32; 3]>,
}

// HSD_LObj flags (lobj.h).
const LOBJ_TYPE: u16 = 3;
const LOBJ_AMBIENT: u16 = 0;
const LOBJ_POINT: u16 = 2;
const LOBJ_DIFFUSE: u16 = 1 << 2;
const LOBJ_SPECULAR: u16 = 1 << 3;
const LOBJ_HIDDEN: u16 = 1 << 5;
/// Set by Ground_801C20E0's override bit c.
const LOBJ_FLAG_400: u16 = 0x400;
/// HSD light animation tracks (forward.h).
const HSD_A_L_LITC_R: u8 = 9;
const HSD_A_L_LITC_B: u8 = 11;
const HSD_A_L_VIS: u8 = 12;
const HSD_A_L_LITC_A: u8 = 22;
/// HSD WObj tracks: the path parameter, then translation x, y, z.
const HSD_A_W_PATH: u8 = 4;

struct Source {
    flags: u16,
    color: [u8; 4],
    color_animation: Option<AObj>,
    base: [f32; 3],
    position_animation: Option<AObj>,
    path: Option<hsd_archive::desc::spline::LinearSpline>,
    path_matrix: hsd_types::Mtx,
    distance_attenuation: [f32; 3],
}
pub(super) struct Lighting {
    pub ambient: [f32; 3],
    pub lights: Vec<DirectionalLight>,
    sources: Vec<Source>,
    /// `Ground_801C445C` reads the head light's WObj for every light, so an
    /// animated position is scaled only when the head has a position.
    scale_animated_positions: Option<f32>,
}
impl Lighting {
    /// `Ground_801C466C` (0x801C466C): the selected map's light set, with
    /// Ground_801C20E0's overrides, positions scaled by the map scale, and
    /// the loop bit of `Ground_801C43C4` on each light's animations.
    pub fn new(game: &Match) -> Result<Self, PresentationError> {
        let assets = &game.assets.inner;
        let archive = &assets.stage;
        let map = super::stage::light_map(assets.stage_desc.kind)?;
        let offset = assets
            .stage_desc
            .models
            .get(map)
            .and_then(|model| model.light_list_offset)
            .ok_or_else(|| error("stage light set missing"))?;
        let mut lights = hsd_archive::visual::read_lights(archive, offset).map_err(error)?;
        apply_overrides(
            &mut lights,
            &melee_gr::desc::read_light_overrides(archive).map_err(error)?,
        );
        let loops = melee_gr::desc::read_light_animation_loops(archive).map_err(error)?;
        let animated = lights.first().is_some_and(|light| light.has_animation_set);
        let scale = assets.stage_desc.parameters.map_scale;
        let mut result = Self {
            ambient: [0.0; 3],
            lights: Vec::with_capacity(lights.len()),
            sources: Vec::with_capacity(lights.len()),
            scale_animated_positions: lights
                .first()
                .is_some_and(|light| light.descriptor.position.is_some())
                .then_some(scale),
        };
        for light in lights {
            let desc = &light.descriptor;
            if desc.flags & LOBJ_TYPE > LOBJ_POINT {
                return Err(error("spot stage light unsupported"));
            }
            if light.interest_animation.is_some() || desc.interest.is_some() {
                return Err(error("stage light interest unsupported"));
            }
            // Ground_801C43C4 decides from the light's first HSD_LightAnim.
            let looped = animated
                && loops
                    .iter()
                    .find(|(animation, _)| {
                        *animation == light.animation_offset && animation.is_some()
                    })
                    .is_some_and(|&(_, looped)| looped);
            let load = |animation: &Option<hsd_archive::desc::AObjDesc>| {
                animation
                    .as_ref()
                    .map(|a| -> Result<AObj, PresentationError> {
                        let mut aobj =
                            AObj::load_desc(&hsd_anim::load::animation_object(a).map_err(error)?);
                        if looped {
                            aobj.set_flags(AOBJ_LOOP);
                        }
                        Ok(aobj)
                    })
                    .transpose()
            };
            let (base, path, path_matrix) =
                position(archive, desc, light.position_animation.as_ref(), scale)?;
            result.sources.push(Source {
                flags: desc.flags,
                color: desc.color,
                color_animation: load(&light.color_animation)?,
                base,
                position_animation: load(&light.position_animation)?,
                path,
                path_matrix,
                distance_attenuation: distance_attenuation(desc),
            });
        }
        result.capture(game.tick());
        Ok(result)
    }
    pub fn capture(&mut self, tick: crate::Tick) {
        self.lights.clear();
        for source in &mut self.sources {
            let mut color = source.color;
            let mut hidden = source.flags & LOBJ_HIDDEN != 0;
            if let Some(animation) = &mut source.color_animation {
                // Requesting the absolute simulation tick also handles reset,
                // replacement, skipped captures and captures on a fresh view.
                animation.req_anim(tick.0 as f32);
                animation.interpret_anim(
                    &mut |track, value| match track {
                        // LObjUpdateFunc: `color = 255.0 * fv` (double, then u8).
                        HSD_A_L_LITC_R..=HSD_A_L_LITC_B => {
                            color[usize::from(track - HSD_A_L_LITC_R)] =
                                (255.0 * f64::from(value)) as u8;
                        }
                        HSD_A_L_LITC_A => color[3] = (255.0 * f64::from(value)) as u8,
                        HSD_A_L_VIS => hidden = value >= 0.5,
                        _ => {}
                    },
                    &mut AObjEndCallback::default(),
                );
            }
            let mut position = source.base;
            if let Some(animation) = &mut source.position_animation {
                animation.req_anim(tick.0 as f32);
                let mut moved = false;
                animation.interpret_anim(
                    &mut |track, value| {
                        moved = true;
                        if track == HSD_A_W_PATH {
                            let p = hsd_anim::spline::linear_point(
                                source.path.as_ref().expect("validated path track"),
                                value.clamp(0.0, 1.0),
                            );
                            // HSD_WObjGetPosition (0x8037D720) resolves spline-local positions
                            // through the referenced JObj matrix before lighting.
                            let mut world = hsd_types::Vec3::ZERO;
                            hsd_anim::mtx::mtx_mult_vec(&source.path_matrix, &p, &mut world);
                            position = [world.x, world.y, world.z];
                        } else {
                            position[usize::from(track - HSD_A_W_PATH - 1)] = value;
                        }
                    },
                    &mut AObjEndCallback::default(),
                );
                if let (true, Some(scale)) = (moved, self.scale_animated_positions) {
                    position = position.map(|v| v * scale);
                }
            }
            let rgb = std::array::from_fn(|i| f32::from(color[i]) / 255.0);
            let kind = source.flags & LOBJ_TYPE;
            if hidden {
                continue;
            }
            if kind == LOBJ_AMBIENT {
                if source.flags & LOBJ_DIFFUSE != 0 {
                    self.ambient = rgb;
                }
                continue;
            }
            // HSD_LObjSetupInit (0x80366F9C): a light neither diffuse nor
            // specular is never activated.
            if source.flags & (LOBJ_DIFFUSE | LOBJ_SPECULAR) == 0 {
                continue;
            }
            self.lights.push(DirectionalLight {
                direction: position,
                color: rgb,
                diffuse: source.flags & LOBJ_DIFFUSE != 0,
                specular: source.flags & LOBJ_SPECULAR != 0,
                distance_attenuation: (kind == LOBJ_POINT).then_some(source.distance_attenuation),
            });
        }
    }
}

/// `Ground_801C20E0` (0x801C20E0): when any light of the set has an override
/// with a bit set, every non-ambient light without one is dropped and the
/// others take the override's diffuse, specular and 0x400 bits.
fn apply_overrides(
    lights: &mut Vec<hsd_archive::visual::Light>,
    overrides: &[melee_gr::desc::LightOverride],
) {
    let find = |light: &hsd_archive::visual::Light| {
        overrides
            .iter()
            .find(|o| o.descriptor == Some(light.descriptor_offset))
            .filter(|o| o.specular || o.diffuse || o.flag_400)
    };
    if !lights.iter().any(|light| find(light).is_some()) {
        return;
    }
    lights.retain_mut(|light| {
        if light.descriptor.flags & LOBJ_TYPE == LOBJ_AMBIENT {
            return true;
        }
        let Some(o) = find(light) else {
            return false;
        };
        let flags = &mut light.descriptor.flags;
        for (set, bit) in [
            (o.diffuse, LOBJ_DIFFUSE),
            (o.specular, LOBJ_SPECULAR),
            (o.flag_400, LOBJ_FLAG_400),
        ] {
            if set {
                *flags |= bit;
            } else {
                *flags &= !bit;
            }
        }
        true
    });
}

/// `setup_point_lightobj` (lobj.c) through `GXInitLightDistAttn`: the
/// descriptor's reference brightness, distance and function (GX_DA_*).
fn distance_attenuation(desc: &LightDesc) -> [f32; 3] {
    let Some(a) = &desc.point_attenuation else {
        return [1.0, 0.0, 0.0];
    };
    let (brightness, distance) = (a.reference_brightness, a.reference_distance);
    let function = if distance < 0.0 || brightness <= 0.0 || brightness >= 1.0 {
        0
    } else {
        a.distance_function
    };
    match function {
        // GX_DA_GENTLE
        1 => [1.0, (1.0 - brightness) / (brightness * distance), 0.0],
        // GX_DA_MEDIUM
        2 => [
            1.0,
            0.5 * (1.0 - brightness) / (brightness * distance),
            0.5 * (1.0 - brightness) / (brightness * distance * distance),
        ],
        // GX_DA_STEEP
        3 => [
            1.0,
            0.0,
            (1.0 - brightness) / (brightness * distance * distance),
        ],
        // GX_DA_OFF
        _ => [1.0, 0.0, 0.0],
    }
}

type Position = (
    [f32; 3],
    Option<hsd_archive::desc::spline::LinearSpline>,
    hsd_types::Mtx,
);
/// The light's world position, scaled as Ground_801C466C scales it, and the
/// spline its position animation follows.
fn position(
    archive: &hsd_archive::Archive,
    desc: &LightDesc,
    animation: Option<&hsd_archive::desc::AObjDesc>,
    scale: f32,
) -> Result<Position, PresentationError> {
    let mut path = None;
    let mut path_matrix = hsd_types::Mtx::IDENTITY;
    let Some(position) = &desc.position else {
        if animation.is_some() {
            return Err(error("animated light without position"));
        }
        return Ok(([0.0; 3], path, path_matrix));
    };
    if position.constraints_offset.is_some() {
        return Err(error("light constraints unsupported"));
    }
    if let Some(a) = animation {
        if a.tracks().any(|track| !(4..=7).contains(&track.type_)) {
            return Err(error("non-XYZ light position animation unsupported"));
        }
        if a.tracks().any(|track| track.type_ == HSD_A_W_PATH) {
            let joint = hsd_archive::desc::JObjDesc::read(archive, a.obj_id).map_err(error)?;
            let (mut tree, root) =
                hsd_anim::load::load_joint_tree(archive, &joint).map_err(error)?;
            tree.setup_matrix(root);
            path_matrix = tree.get(root).mtx;
            let hsd_archive::desc::JObjUnion::Spline(Some(offset)) = joint.u else {
                return Err(error("light path target is not a spline"));
            };
            path = Some(
                hsd_archive::desc::spline::LinearSpline::read(archive, offset).map_err(error)?,
            );
        }
    }
    let p = position.position;
    // Ground_801C466C (0x801C466C): separate fmuls per component.
    Ok(([p.x * scale, p.y * scale, p.z * scale], path, path_matrix))
}

//! Read-only, tick-indexed evaluation of the stage's authored light tracks.
//! Uses the existing AObj/FObj interpreter; capture frequency is not a clock.
use super::{error, PresentationError};
use crate::Match;
use hsd_anim::aobj::{AObj, AObjEndCallback};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DirectionalLight {
    /// World-space vector toward the light, normalized by the renderer.
    pub direction: [f32; 3],
    pub color: [f32; 3],
    pub diffuse: bool,
    pub specular: bool,
}
struct Source {
    base: [f32; 3],
    animation: Option<AObj>,
    path: Option<hsd_archive::desc::spline::LinearSpline>,
}
pub(super) struct Lighting {
    pub ambient: [f32; 3],
    pub lights: Vec<DirectionalLight>,
    sources: Vec<Source>,
}
impl Lighting {
    pub fn new(game: &Match) -> Result<Self, PresentationError> {
        let assets = &game.assets.inner;
        // grLast_StageCallbacks marks map 3 as the selected environment/light
        // set. Match configuration currently validates Final Destination only.
        let offset = assets.stage_desc.models[3]
            .light_list_offset
            .ok_or_else(|| error("Final Destination light set missing"))?;
        let mut result = Self {
            ambient: [0.0; 3],
            lights: Vec::new(),
            sources: Vec::new(),
        };
        for light in hsd_archive::visual::read_lights(&assets.stage, offset).map_err(error)? {
            let desc = light.descriptor;
            if desc.flags & (1 << 5) != 0 {
                continue;
            }
            if light.color_animation.is_some() || light.interest_animation.is_some() {
                return Err(error("light color/interest animation unsupported"));
            }
            let color = std::array::from_fn(|i| f32::from(desc.color[i]) / 255.0);
            match desc.flags & 3 {
                0 => {
                    if desc.flags & 4 != 0 {
                        result.ambient = color;
                    }
                }
                1 => {
                    let source =
                        position_source(&assets.stage, desc.position, light.position_animation)?;
                    let base = source.base;
                    result.sources.push(source);
                    result.lights.push(DirectionalLight {
                        direction: base,
                        color,
                        diffuse: desc.flags & 4 != 0,
                        specular: desc.flags & 8 != 0,
                    });
                }
                _ => return Err(error("non-directional stage light unsupported")),
            }
        }
        Ok(result)
    }
    pub fn capture(&mut self, tick: crate::Tick) {
        for (light, source) in self.lights.iter_mut().zip(&mut self.sources) {
            light.direction = source.base;
            if let Some(animation) = &mut source.animation {
                // Requesting the absolute simulation tick also handles reset,
                // replacement, skipped captures and captures on a fresh view.
                // The interpreter owns loop wrapping and track interpolation.
                animation.req_anim(tick.0 as f32);
                animation.interpret_anim(
                    &mut |track, value| {
                        if track == 4 {
                            let p = hsd_anim::spline::linear_point(
                                source.path.as_ref().expect("validated path track"),
                                value.clamp(0.0, 1.0),
                            );
                            light.direction = [p.x, p.y, p.z];
                        } else {
                            light.direction[usize::from(track - 5)] = value;
                        }
                    },
                    &mut AObjEndCallback::default(),
                );
            }
        }
    }
}

fn position_source(
    archive: &hsd_archive::Archive,
    position: Option<hsd_archive::desc::light::WorldPositionDesc>,
    animation: Option<hsd_archive::desc::AObjDesc>,
) -> Result<Source, PresentationError> {
    let position = position.ok_or_else(|| error("directional light position missing"))?;
    if position.constraints_offset.is_some() {
        return Err(error("light constraints unsupported"));
    }
    let p = position.position;
    let base = [p.x, p.y, p.z];
    let mut path = None;
    let animation = animation
        .map(|a| {
            if a.tracks().any(|track| !(4..=7).contains(&track.type_)) {
                return Err(error("non-XYZ light position animation unsupported"));
            }
            if a.tracks().any(|track| track.type_ == 4) {
                let joint = hsd_archive::desc::JObjDesc::read(archive, a.obj_id).map_err(error)?;
                let hsd_archive::desc::JObjUnion::Spline(Some(offset)) = joint.u else {
                    return Err(error("light path target is not a spline"));
                };
                path = Some(
                    hsd_archive::desc::spline::LinearSpline::read(archive, offset)
                        .map_err(error)?,
                );
            }
            let desc = hsd_anim::load::animation_object(&a).map_err(error)?;
            Ok(AObj::load_desc(&desc))
        })
        .transpose()?;
    Ok(Source {
        base,
        animation,
        path,
    })
}

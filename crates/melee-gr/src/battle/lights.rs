//! Battlefield map 6 selects two static lights with no animation descriptors.
use crate::desc::{ReadResult, StageDesc};
use hsd_archive::Archive;
use hsd_types::Vec3;

// HSD_LObj flags: type in bits 0..1, diffuse bit 2, specular bit 3.
const AMBIENT_DIFFUSE: u16 = 4;
const DIRECTIONAL_DIFFUSE_SPECULAR: u16 = 13;

#[derive(Clone, Debug, PartialEq)]
pub enum Light {
    Ambient {
        color: [u8; 4],
    },
    Point {
        color: [u8; 4],
        position: Vec3,
        attenuation: hsd_archive::desc::light::PointAttenuation,
    },
    Directional {
        color: [u8; 4],
        position: Vec3,
        shininess: f32,
    },
}
/// `Ground_801C466C` (0x801C466C), ground.c:2692-2713: scale the
/// loaded world positions. Fusion audit: only independent fmuls at this site.
pub fn load(archive: &Archive, desc: &StageDesc) -> ReadResult<Vec<Light>> {
    load_model(archive, desc, 6)
}
pub fn load_model(archive: &Archive, desc: &StageDesc, map: usize) -> ReadResult<Vec<Light>> {
    crate::desc::read_static_lights(archive, &desc.models[map])?
        .into_iter()
        .map(|light| {
            assert!(light.class_name.is_none() && light.next_offset.is_none());
            assert_eq!(light.attenuation_flags, 0);
            assert!(light.interest.is_none());
            Ok(match light.flags {
                AMBIENT_DIFFUSE => Light::Ambient { color: light.color },
                DIRECTIONAL_DIFFUSE_SPECULAR => {
                    let position = light.position.expect("directional position");
                    assert!(position.class_name.is_none() && position.constraints_offset.is_none());
                    let scale = desc.parameters.map_scale;
                    Light::Directional {
                        color: light.color,
                        // Retail 0x801C47E0/47EC/47F8: separate fmuls.
                        position: Vec3::new(
                            position.position.x * scale,
                            position.position.y * scale,
                            position.position.z * scale,
                        ),
                        shininess: light.shininess.expect("directional shininess"),
                    }
                }
                14 => {
                    let position = light.position.expect("point position");
                    assert!(position.class_name.is_none() && position.constraints_offset.is_none());
                    let scale = desc.parameters.map_scale;
                    // Ground_801C466C: independent fmuls, as for directional lights.
                    Light::Point {
                        color: light.color,
                        position: Vec3::new(
                            position.position.x * scale,
                            position.position.y * scale,
                            position.position.z * scale,
                        ),
                        attenuation: light.point_attenuation.expect("point attenuation"),
                    }
                }
                _ => unimplemented!(
                    "lobj.c LObjLoad: Battlefield light kind outside ambient/directional"
                ),
            })
        })
        .collect()
}

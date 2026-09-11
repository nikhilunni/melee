//! Read-only psdisp geometry inputs; no particle update, cache mutation or RNG.
use crate::{
    generator::{EmissionShape, Generator},
    particle::{Particle, TORNADO},
    Error,
};
use gekko_math::{
    fma::{fmadds, fmsubs},
    msl::{cosf, sinf, tanf},
};

/// calcTornadoLastPos (8039F89C). Ordinary trails use pos - vel; tornado
/// velocity holds cylindrical coordinates, so their previous point is rebuilt.
pub fn previous_position(p: &Particle, generator: Option<&Generator>) -> Result<[f32; 3], Error> {
    if p.kind & TORNADO == 0 {
        return Ok(std::array::from_fn(|axis| {
            p.position[axis] - p.velocity[axis]
        }));
    }
    let g = generator.ok_or(Error::UnsupportedFeature(
        "tornado display without generator",
    ))?;
    let EmissionShape::Tornado { speed } = g.shape else {
        return Err(Error::UnsupportedFeature("tornado display generator shape"));
    };
    Ok(tornado_previous(
        p.velocity,
        [p.gravity, p.friction],
        g.position,
        speed,
        g.descriptor.gravity,
        g.descriptor.radius,
        g.descriptor.angle,
    ))
}
fn tornado_previous(
    velocity: [f32; 3],
    rotation: [f32; 2],
    position: [f32; 3],
    speed: f32,
    angular_speed: f32,
    radius: f32,
    angle: f32,
) -> [f32; 3] {
    let sin_a = sinf(rotation[0]);
    let sin_b = sinf(rotation[1]);
    let cos_a = cosf(rotation[0]);
    let cos_b = cosf(rotation[1]);
    let z = velocity[2] - speed;
    let theta = velocity[0] - angular_speed;
    // Retail ABS compares before negating, retaining negative zero.
    let radius = if radius < 0.0 { -radius } else { radius };
    let angle = if angle < 0.0 { -angle } else { angle };
    // 8039F978 fmadds, F984 fmuls.
    let radius = fmadds(z, tanf(angle), radius) * velocity[1];
    let x = radius * cosf(theta);
    let y = radius * sinf(theta);
    // F9A8, F9BC/F9CC, F9D8/F9E0; translations are separate fadds.
    [
        position[0] + fmadds(x, cos_b, z * sin_b),
        position[1] + fmadds(cos_b, z * sin_a, fmadds(sin_b, -x * sin_a, y * cos_a)),
        position[2] + fmadds(cos_b, z * cos_a, fmsubs(sin_b, -x * cos_a, y * sin_a)),
    ]
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tornado_trail_reconstructs_prior_cylindrical_point() {
        assert_eq!(
            tornado_previous(
                [0.5, 1.0, 3.0],
                [0.0, 0.0],
                [3.0, 4.0, 5.0],
                1.0,
                0.5,
                -2.0,
                0.0
            ),
            [5.0, 4.0, 7.0]
        );
    }
}

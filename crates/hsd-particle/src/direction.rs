//! hsd_80398F8C (particle.c): rotate velocity onto a cone of fixed aperture.
use crate::rng_sites::DrawLog;
use gekko_math::{
    fma::{fmadds, fmsubs},
    msl::{cosf, sinf, sqrtf},
    HsdRng,
};
use hsd_anim::mtx::InverseTrig;

pub(crate) fn randomize<T: InverseTrig>(
    [x, y, z]: [f32; 3],
    aperture: f32,
    rng: &mut HsdRng,
    draws: &mut DrawLog,
) -> [f32; 3] {
    fn angle<T: InverseTrig>(y: f32, x: f32) -> f32 {
        if f32::from_bits(x.to_bits() & 0x7fff_ffff) < f32::MIN_POSITIVE {
            if y >= 0.0 {
                std::f32::consts::FRAC_PI_2
            } else {
                -std::f32::consts::FRAC_PI_2
            }
        } else {
            T::atan2f(y, x)
        }
    }
    let azimuth = angle::<T>(y, z);
    let (sin_a, cos_a) = (sinf(azimuth), cosf(azimuth));
    // Retail 80399044 fmuls, 80399048 fmadds.
    let elevation = angle::<T>(x, fmadds(y, sin_a, z * cos_a));
    let (sin_e, cos_e) = (sinf(elevation), cosf(elevation));
    // Retail 803990AC fmuls, 803990B8/BC fmadds; inline sqrt uses
    // the same three double fnmsub Newton steps as MSL sqrtf.
    let magnitude = sqrtf(fmadds(z, z, fmadds(x, x, y * y)));
    // 80399124/2C: two double fmuls, then a single frsp at 80399130.
    let random_angle =
        (std::f64::consts::PI * f64::from(draws.draw(rng, 0x8039_9114)) * 2.0) as f32;
    let radial = magnitude * sinf(aperture);
    let radial_cos = radial * cosf(random_angle);
    let radial_sin = radial * sinf(random_angle);
    let axial = magnitude * cosf(aperture);
    // Retail 80399178/80/8C/90/94. Keep each inner product rounded before
    // the outer fmadd, including the negated radial component.
    [
        fmadds(radial_cos, cos_e, axial * sin_e),
        fmadds(
            cos_e,
            axial * sin_a,
            fmadds(sin_e, -radial_cos * sin_a, radial_sin * cos_a),
        ),
        fmadds(
            cos_e,
            axial * cos_a,
            fmsubs(sin_e, -radial_cos * cos_a, radial_sin * sin_a),
        ),
    ]
}

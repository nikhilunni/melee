//! Plain shared data types for the HSD port: Vec3, Mtx, ids, flags. Leaf crate: keep tiny and stable so nothing rebuilds when logic changes
//!
//! See CLAUDE.md for the porting rules that apply to every crate.
//!
//! These are data-only transcriptions of the Dolphin SDK vector and matrix
//! typedefs. They carry no arithmetic: every float operation on them must go
//! through `gekko-math` so FMA contraction and rounding match the retail
//! binary.

/// Two `f32` components.
///
/// Source: `extern/dolphin/include/dolphin/mtx.h` (`Vec2`, also aliased
/// `Point2d`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    /// All components zero.
    pub const ZERO: Vec2 = Vec2 { x: 0.0, y: 0.0 };

    #[inline]
    pub const fn new(x: f32, y: f32) -> Vec2 {
        Vec2 { x, y }
    }
}

/// Three `f32` components.
///
/// Source: `extern/dolphin/include/dolphin/mtx.h` (`Vec`, also aliased
/// `Vec3` and `Point3d`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    /// All components zero.
    pub const ZERO: Vec3 = Vec3 {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    #[inline]
    pub const fn new(x: f32, y: f32, z: f32) -> Vec3 {
        Vec3 { x, y, z }
    }
}

/// Two `s8` components.
///
/// Source: `src/melee/lb/types.h` (`S8Vec2`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct S8Vec2 {
    pub x: i8,
    pub y: i8,
}

impl S8Vec2 {
    /// All components zero.
    pub const ZERO: S8Vec2 = S8Vec2 { x: 0, y: 0 };

    #[inline]
    pub const fn new(x: i8, y: i8) -> S8Vec2 {
        S8Vec2 { x, y }
    }
}

/// A 3x4 row-major affine matrix, `f32 Mtx[3][4]`: three rows of four,
/// with the translation in column 3.
///
/// Source: `extern/dolphin/include/dolphin/mtx.h` (`Mtx`).
///
/// `Default` is the all-zero matrix, matching a zeroed C array; use
/// [`Mtx::IDENTITY`] when an identity is wanted.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Mtx(pub [[f32; 4]; 3]);

impl Mtx {
    /// All entries zero.
    pub const ZERO: Mtx = Mtx([[0.0; 4]; 3]);

    /// Rotation part is the identity and translation is zero.
    pub const IDENTITY: Mtx = Mtx([
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
    ]);
}

impl From<[[f32; 4]; 3]> for Mtx {
    #[inline]
    fn from(m: [[f32; 4]; 3]) -> Mtx {
        Mtx(m)
    }
}

impl From<Mtx> for [[f32; 4]; 3] {
    #[inline]
    fn from(m: Mtx) -> [[f32; 4]; 3] {
        m.0
    }
}

impl From<[f32; 3]> for Vec3 {
    fn from([x, y, z]: [f32; 3]) -> Self {
        Self { x, y, z }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{align_of, size_of};

    #[test]
    fn layouts_match_the_sdk() {
        // sizeof(Vec2) == 8, sizeof(Vec) == 12, sizeof(S8Vec2) == 2,
        // sizeof(Mtx) == 48; all f32 types 4-aligned, S8Vec2 1-aligned.
        assert_eq!(size_of::<Vec2>(), 8);
        assert_eq!(align_of::<Vec2>(), 4);
        assert_eq!(size_of::<Vec3>(), 12);
        assert_eq!(align_of::<Vec3>(), 4);
        assert_eq!(size_of::<S8Vec2>(), 2);
        assert_eq!(align_of::<S8Vec2>(), 1);
        assert_eq!(size_of::<Mtx>(), 48);
        assert_eq!(align_of::<Mtx>(), 4);
    }

    #[test]
    fn defaults_are_zero() {
        assert_eq!(Vec2::default(), Vec2::ZERO);
        assert_eq!(Vec3::default(), Vec3::ZERO);
        assert_eq!(S8Vec2::default(), S8Vec2::ZERO);
        assert_eq!(Mtx::default(), Mtx::ZERO);
        assert_ne!(Mtx::default(), Mtx::IDENTITY);
    }

    #[test]
    fn constructors_and_conversions() {
        let v = Vec3::new(1.0, 2.0, 3.0);
        assert_eq!((v.x, v.y, v.z), (1.0, 2.0, 3.0));
        assert_eq!(Vec2::new(4.0, 5.0), Vec2 { x: 4.0, y: 5.0 });
        assert_eq!(S8Vec2::new(-1, 127), S8Vec2 { x: -1, y: 127 });

        let raw = [
            [1.0, 2.0, 3.0, 4.0],
            [5.0, 6.0, 7.0, 8.0],
            [9.0, 10.0, 11.0, 12.0],
        ];
        let m = Mtx::from(raw);
        assert_eq!(m.0[1][2], 7.0);
        assert_eq!(<[[f32; 4]; 3]>::from(m), raw);
        assert_eq!(Mtx::IDENTITY.0[0][0], 1.0);
        assert_eq!(Mtx::IDENTITY.0[2][3], 0.0);
    }
}

pub mod storage;

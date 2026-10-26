//! The F-Zero stages' shared code: Mute City's and Big Blue's thirty
//! background cars (grfzerocar.c) and the orientation both stages give a
//! car from its track frame (Ground_801C5AEC).
use hsd_anim::jobj::{JObjId, JObjTree};
use hsd_types::{Mtx, Vec3};

/// The racers on the track (grFZeroCar_803E0BD8 has thirty rows).
pub const CARS: usize = 30;

/// grFZeroCar_801CAFBC (0x801CAFBC): the car map's GObj JObj at unit scale
/// with thirty translation joints under it, one per car, each at the
/// ground's scale over its car model. The stages move and turn these
/// joints (`tra_jobj`); a car's stage material item and generators follow
/// them. The car models below them only draw (the display tree is the
/// presentation's), so the simulation keeps the joints alone.
#[derive(Clone)]
pub struct CarJoints {
    tree: JObjTree,
    root: JObjId,
    cars: [JObjId; CARS],
}

impl CarJoints {
    /// The joints as grFZeroCar_801CAFBC leaves them: the root's scale set
    /// to one (HSD_JObjSetScale(root, {1, 1, 1})), each car joint an
    /// HSD_JObjAlloc with an identity matrix and no parent scale, scaled by
    /// `ground_scale` (HSD_JObjGetScale times Ground_801C0498, fmuls).
    pub fn new(ground_scale: f32) -> Self {
        let mut tree = JObjTree::new();
        let root = tree.alloc();
        tree.set_scale(root, &Vec3::new(1.0, 1.0, 1.0));
        let cars = std::array::from_fn(|_| {
            let car = tree.alloc();
            tree.get_mut(car).mtx = Mtx::IDENTITY;
            let scale = tree.scale(car);
            tree.set_scale(
                car,
                &Vec3::new(
                    scale.x * ground_scale,
                    scale.y * ground_scale,
                    scale.z * ground_scale,
                ),
            );
            tree.add_child(root, car);
            car
        });
        Self { tree, root, cars }
    }
    /// HSD_JObjSetTranslate(car joint).
    pub fn set_translate(&mut self, car: usize, translate: Vec3) {
        self.tree.set_translate(self.cars[car], &translate);
    }
    /// HSD_JObjSetRotationX/Y/Z(car joint), in that order.
    pub fn set_rotation(&mut self, car: usize, rotation: Vec3) {
        let joint = self.cars[car];
        self.tree.set_rotation_x(joint, rotation.x);
        self.tree.set_rotation_y(joint, rotation.y);
        self.tree.set_rotation_z(joint, rotation.z);
    }
    pub fn translation(&self, car: usize) -> Vec3 {
        self.tree.translation(self.cars[car])
    }
    /// The Euler rotation a car's stage material item copies
    /// (HSD_JObjGetRotation, it_2E6A_UnkMotion19_Phys).
    pub fn rotation(&self, car: usize) -> Vec3 {
        let q = self.tree.rotation(self.cars[car]);
        Vec3::new(q.x, q.y, q.z)
    }
    pub fn scale(&self, car: usize) -> Vec3 {
        self.tree.scale(self.cars[car])
    }
    /// HSD_JObjSetupMatrix, then the world matrix.
    pub fn matrix(&mut self, car: usize) -> Mtx {
        *self.tree.get_mtx(self.cars[car])
    }
    /// `lb_8000B1CC(car joint, NULL, &out)` (0x8000B1CC): the joint has a
    /// parent, so its matrix is set up and its translation column read.
    pub fn world_position(&mut self, car: usize) -> Vec3 {
        let m = self.matrix(car);
        Vec3::new(m.0[0][3], m.0[1][3], m.0[2][3])
    }
    /// The GObj JObj every car joint hangs from.
    pub fn root_matrix(&mut self) -> Mtx {
        *self.tree.get_mtx(self.root)
    }
}

/// Ground_801C5AEC (0x801C5AEC): lbVector_EulerAnglesFromONB of the frame
/// (`forward`, `side`, `up`), each angle not below 30000 in magnitude set
/// to zero (a NaN fails `ABS(v) < 30000` too). No arithmetic of its own.
#[allow(clippy::neg_cmp_op_on_partial_ord)] // A NaN angle is zeroed too.
pub fn orientation(forward: Vec3, side: Vec3, up: Vec3) -> Vec3 {
    let mut v = melee_lb::orientation::from_basis(forward, side, up);
    for angle in [&mut v.x, &mut v.y, &mut v.z] {
        if !(gekko_math::msl::fabsf(*angle) < 30000.0) {
            *angle = 0.0;
        }
    }
    v
}

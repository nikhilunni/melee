//! A fighter landing on an item (ft_80081A00 and the item's xB54 box):
//! the item's side of the test and the test itself (lb_80014638).
/// Item.xB54 (lb_80014638_arg1_t), refreshed at the end of each collision
/// proc (it_8027574C): how far the item's ECB centre moved this frame, and
/// the box a stomp lands on.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct StompBox {
    /// unk_x / unk_y: the ECB centre's movement (it_80274D04).
    pub shift_x: f32,
    pub shift_y: f32,
    /// x8: the box's left and right X and its top Y (it_80274D6C).
    pub left: f32,
    pub right: f32,
    pub top: f32,
}

/// A fighter's swept stomp bounds (dmg.x1930): its ECB's left and right X
/// at its position and its foot height (cur_pos.y + PlCo x17C).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct StompBounds {
    pub left: f32,
    pub right: f32,
    pub bottom: f32,
}

impl crate::ItemCore {
    /// it_8027574C (8027574C): the ECB centre's movement since the last
    /// pass (it_8026BB88 less it_8026BBCC), then the box (it_80274D6C).
    pub fn update_stomp_box(&mut self) {
        let collision = self.collision.as_ref().expect("item map collision");
        // it_8026BB88 / it_8026BBCC: 0.5 * (top.y + bottom.y), fadds, fmuls.
        let centre = 0.5 * (collision.ecb.top.y + collision.ecb.bottom.y);
        let last_centre = 0.5 * (collision.xe4_ecb.top.y + collision.xe4_ecb.bottom.y);
        let now_x = self.position.x + 0.0;
        let now_y = self.position.y + centre;
        let last_x = collision.last_pos.x + 0.0;
        let last_y = collision.last_pos.y + last_centre;
        self.stomp_box.shift_x = now_x - last_x;
        self.stomp_box.shift_y = now_y - last_y;
        self.set_stomp_box_extent();
    }

    /// it_80274DAC (80274DAC): no movement, then the box.
    pub fn reset_stomp_box(&mut self) {
        self.stomp_box.shift_x = 0.0;
        self.stomp_box.shift_y = 0.0;
        self.set_stomp_box_extent();
    }

    /// it_80274D6C (80274D6C): the ECB's left and right at the item's X;
    /// its top is the right vertex's Y plus the top vertex's, as retail
    /// adds them (fadds, fadds).
    fn set_stomp_box_extent(&mut self) {
        let collision = self.collision.as_ref().expect("item map collision");
        let ecb = collision.ecb;
        self.stomp_box.right = ecb.right.x + self.position.x;
        self.stomp_box.left = ecb.left.x + self.position.x;
        self.stomp_box.top = ecb.right.y + self.position.y + ecb.top.y;
    }
}

/// lb_80014638 (80014638): whether a fighter whose stomp bounds went from
/// `previous` to `current` came down onto `item` this frame. The previous
/// bounds move with the item; the feet must pass from above the box's top
/// to at or below it, overlapping it sideways where they cross.
pub fn lands_on(previous: StompBounds, current: StompBounds, item: StompBox) -> bool {
    use gekko_math::fma::fmadds;
    let start_left = previous.left + item.shift_x;
    let start_right = previous.right + item.shift_x;
    let start_bottom = previous.bottom + item.shift_y;
    if start_bottom < current.bottom {
        return false;
    }
    if start_bottom < item.top || current.bottom > item.top {
        return false;
    }
    let drop = current.bottom - start_bottom;
    let magnitude = if drop < 0.0 { -drop } else { drop };
    let t = if magnitude < 0.01 {
        1.0
    } else {
        (item.top - start_bottom) / drop
    };
    // retail 8001472C / 80014750: fsubs, then fmadds(t, delta, start).
    if fmadds(t, current.left - start_left, start_left) > item.right {
        return false;
    }
    if fmadds(t, current.right - start_right, start_right) < item.left {
        return false;
    }
    true
}

/// A stompable item as a fighter's map proc sees it (ft_80081A00's walk of
/// the item list, in list order).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StompTarget {
    pub item: u32,
    pub position_x: f32,
    pub stomp_box: StompBox,
}

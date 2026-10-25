//! Menu focus movement shared by the hosts: given the on-screen frames of
//! the focusable controls, which one a direction moves to. Both hosts lay
//! out their own menus and hand the frames over, so the D-pad, the stick
//! and the arrow keys move the same way on macOS and the web.

/// A direction of focus movement (`melee_focus_e`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    pub fn from_index(index: u32) -> Option<Self> {
        Some(match index {
            0 => Self::Up,
            1 => Self::Down,
            2 => Self::Left,
            3 => Self::Right,
            _ => return None,
        })
    }
}

/// A control's frame: x, y (top-left origin, y down), width, height.
pub type Frame = [f32; 4];

fn centre(f: &Frame) -> (f32, f32) {
    (f[0] + f[2] / 2.0, f[1] + f[3] / 2.0)
}

/// Spans overlap on the cross axis (same row for left/right).
fn same_row(a: &Frame, b: &Frame) -> bool {
    a[1] < b[1] + b[3] && b[1] < a[1] + a[3]
}

/// The control focus moves to from `from` in `direction`: the nearest
/// ahead, weighting sideways distance three times. Left and right stay in
/// the row (controls overlapping vertically) and wrap at its ends; up and
/// down stop at the edge (`None`).
pub fn next(frames: &[Frame], from: usize, direction: Direction) -> Option<usize> {
    let origin = frames.get(from)?;
    let (fx, fy) = centre(origin);
    let mut best: Option<(usize, f32)> = None;
    for (index, frame) in frames.iter().enumerate() {
        if index == from {
            continue;
        }
        let (x, y) = centre(frame);
        let (dx, dy) = (x - fx, y - fy);
        let (along, across) = match direction {
            Direction::Up => (-dy, dx.abs()),
            Direction::Down => (dy, dx.abs()),
            Direction::Left => (-dx, dy.abs()),
            Direction::Right => (dx, dy.abs()),
        };
        let sideways = matches!(direction, Direction::Left | Direction::Right);
        if along <= 1.0 || (sideways && !same_row(origin, frame)) {
            continue;
        }
        let score = along + across * 3.0;
        if best.is_none_or(|(_, s)| score < s) {
            best = Some((index, score));
        }
    }
    if best.is_none() && matches!(direction, Direction::Left | Direction::Right) {
        // Wrap: the farthest control the other way in the same row.
        let row = frames
            .iter()
            .enumerate()
            .filter(|(i, f)| *i != from && same_row(origin, f));
        let wrapped = if direction == Direction::Right {
            row.min_by(|a, b| centre(a.1).0.total_cmp(&centre(b.1).0))
        } else {
            row.max_by(|a, b| centre(a.1).0.total_cmp(&centre(b.1).0))
        };
        return wrapped.map(|(i, _)| i);
    }
    best.map(|(i, _)| i)
}

#[cfg(test)]
mod tests {
    use super::*;
    use Direction::*;

    /// Two rows: three buttons, then one wide button under the middle.
    const FRAMES: [Frame; 4] = [
        [0.0, 0.0, 100.0, 40.0],
        [120.0, 0.0, 100.0, 40.0],
        [240.0, 0.0, 100.0, 40.0],
        [60.0, 80.0, 220.0, 40.0],
    ];

    #[test]
    fn moves_to_the_nearest_control_ahead() {
        let cases = [
            (0, Right, Some(1)),
            (1, Left, Some(0)),
            (1, Down, Some(3)),
            (3, Up, Some(1)),
            (0, Down, Some(3)),
            // Rows wrap sideways; columns stop at the edge.
            (2, Right, Some(0)),
            (0, Left, Some(2)),
            (0, Up, None),
            (3, Down, None),
            (3, Left, None),
        ];
        for (from, direction, to) in cases {
            assert_eq!(next(&FRAMES, from, direction), to, "{from} {direction:?}");
        }
        assert_eq!(next(&FRAMES, 9, Up), None);
        assert_eq!(next(&[], 0, Up), None);
    }
}

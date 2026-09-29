//! The Belay's rope (itclimbersstring.c) on synthetic attributes.
use ft_iceclimbers::special_hi::rope::{Rope, RopeAttributes};
use hsd_types::Vec3;

fn attributes() -> RopeAttributes {
    RopeAttributes {
        links: 8,
        hanging_links: 5,
        link_length: 1.5,
        minimum_link_length: 1.4,
        gravity: 0.05,
        pay_out_frame: 10,
        hang_frame: 44,
        reel_frame: 60,
    }
}

fn laid_out(at: Vec3) -> Rope {
    let mut rope = Rope::default();
    rope.lay_out(attributes(), at);
    rope
}

#[test]
fn a_new_rope_has_only_its_tail_out() {
    let rope = laid_out(Vec3::new(1.0, 2.0, 3.0));
    let out: Vec<bool> = rope.links[..8].iter().map(|l| l.out).collect();
    assert_eq!(out, [false, false, false, false, false, false, false, true]);
    assert!(rope.links[..8]
        .iter()
        .all(|l| l.position == Vec3::new(1.0, 2.0, 3.0)));
}

#[test]
fn paying_out_stretches_links_from_the_anchor_toward_the_hand() {
    let hand = Vec3::ZERO;
    let mut rope = laid_out(hand);
    rope.start_paying_out(hand);
    // Nana's hand 6 above Popo's: three links come out, one link length
    // apart; the next is only a link length from the hand and stays in.
    rope.pay_out(Vec3::new(0.0, 6.0, 0.0), hand);
    let heights: Vec<f32> = rope.links[..8].iter().map(|l| l.position.y).collect();
    assert_eq!(heights[7], 6.0);
    assert_eq!(&heights[3..7], &[0.0, 1.5, 3.0, 4.5]);
    assert!(rope.links[4..8].iter().all(|l| l.out));
    assert!(!rope.links[3].out);
}

#[test]
fn reeling_ends_once_the_walk_reaches_the_tail() {
    let hand = Vec3::ZERO;
    let mut rope = laid_out(hand);
    rope.start_paying_out(hand);
    rope.pay_out(Vec3::new(0.0, 3.0, 0.0), hand);
    let mut frames = 0;
    while !rope.reel(hand) {
        frames += 1;
        assert!(frames < 100, "the rope never came in");
    }
    assert!(rope.links[..7].iter().all(|l| !l.out));
}

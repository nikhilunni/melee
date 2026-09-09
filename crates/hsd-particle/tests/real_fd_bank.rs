//! Disc-backed provenance for the idle FD generator. No game data is embedded.
use hsd_anim::fobj::FObj;
use hsd_archive::desc::AnimJoint;
use hsd_archive::Archive;
use hsd_particle::bank::ParticleBank;
use std::path::PathBuf;

fn archive() -> Option<Archive> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../harness/roms/files/GrNLa.dat");
    if !path.exists() {
        eprintln!("skipping: {} is absent", path.display());
        return None;
    }
    Some(Archive::parse(&std::fs::read(path).unwrap()).unwrap())
}

#[test]
fn real_fd_bank_has_idle_sphere_generator() {
    let Some(archive) = archive() else {
        return;
    };
    let bank = ParticleBank::from_archive(&archive, "map_ptcl", "map_texg").unwrap();
    assert_eq!(bank.version, 0x42);
    assert_eq!(bank.first_descriptor_id, 30000);
    assert_eq!(bank.descriptors.len(), 5);
    assert_eq!(bank.textures.len(), 3);
    let descriptor = bank.descriptor(30000).unwrap();
    assert_eq!(
        (descriptor.generator_type, descriptor.texture_group),
        (8, 0)
    );
    assert_eq!(
        (descriptor.generator_life, descriptor.particle_life),
        (3999, 4)
    );
    assert_eq!(descriptor.kind, 0x0840_0000);
    assert_eq!(descriptor.radius.to_bits(), (-4000.0f32).to_bits());
    assert_eq!(descriptor.emission_rate.to_bits(), 10.0f32.to_bits());
    assert_eq!(descriptor.parameters[0].to_bits(), 0x3f29_c91f);
    eprintln!("FD bank kind 30000: {descriptor:?}");
}

fn tracks(node: &AnimJoint, output: &mut Vec<hsd_archive::desc::FObjDesc>) {
    for sibling in node.siblings() {
        if let Some(aobj) = sibling.aobjdesc.as_ref() {
            for track in aobj.tracks().filter(|track| track.type_ == 0x28) {
                assert_eq!(aobj.end_frame, 4000.0);
                output.push(track.clone());
            }
        }
        if let Some(child) = sibling.child.as_ref() {
            tracks(child, output);
        }
    }
}

/// `JObjUpdateFunc` (jobj.c, 0x8036FDC0) forwards track 0x28's *bits*,
/// not its numeric float value. `efLib_Cb_DPtcl` sends bank 30 through
/// `grLib_801C99C0` to `hsd_8039EFAC` with link 0 and this generator ID.
#[test]
fn map_four_animation_emits_bank_thirty_kind_30000_once_at_start() {
    let Some(archive) = archive() else {
        return;
    };
    let header = archive.public("map_head").unwrap();
    // gr/types.h: map_head.models at +8; Ground_ModelDesc stride 0x34,
    // joint-animation pointer list at +4. This is the model-set-4 entry.
    let models = archive.link(header + 8).unwrap().unwrap();
    let animations = archive.link(models + 4 * 0x34 + 4).unwrap().unwrap();
    let mut found = Vec::new();
    let mut animation_index = 0;
    while let Some(root) = archive.link(animations + animation_index * 4).unwrap() {
        let animation = AnimJoint::read(&archive, root).unwrap();
        let before = found.len();
        tracks(&animation, &mut found);
        if found.len() != before {
            assert_eq!(animation_index, 0);
        }
        animation_index += 1;
    }
    assert_eq!(animation_index, 11);
    assert_eq!(found.len(), 1);
    let track = &found[0];
    let mut runtime = FObj::new(
        &track.ad,
        track.startframe,
        track.type_,
        track.frac_value,
        track.frac_slope,
    );
    runtime.req_anim(0.0);
    let mut events = Vec::new();
    for tick in 0..4000 {
        runtime.interpret_anim(
            Some(&mut |kind, value| {
                assert_eq!(kind, 0x28);
                let bits = value.to_bits();
                events.push((tick, bits & 0x3f, (bits >> 6) & 0x00ff_ffff));
            }),
            1.0,
        );
    }
    assert_eq!(events, [(0, 30, 30000)]);
    eprintln!("FD map4 animation0 particle spawn events (local tick, bank, kind): {events:?}");
}

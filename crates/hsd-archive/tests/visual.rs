use hsd_archive::{
    desc::{read_public_jobj, JObjDesc},
    visual::read_polygons,
    Archive,
};
use std::path::Path;

#[test]
fn original_fox_and_marth_polygons_decode_with_valid_matrix_palettes() {
    let files = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../harness/roms/files");
    for (file, symbol) in [
        ("PlFxNr.dat", "PlyFox5K_Share_joint"),
        ("PlMsNr.dat", "PlyMars5K_Share_joint"),
    ] {
        if !melee_test_support::require_files([files.join(file)]) {
            return;
        }
        let archive = Archive::parse(&std::fs::read(files.join(file)).unwrap()).unwrap();
        let mut texture_decoder = hsd_archive::visual::TextureDecoder::new(&archive);
        let root = read_public_jobj(&archive, symbol).unwrap();
        let mut joints = Vec::new();
        fn visit<'a>(joint: &'a JObjDesc, out: &mut Vec<&'a JObjDesc>) {
            out.push(joint);
            if let Some(child) = joint.child.as_deref() {
                visit(child, out);
            }
            if let Some(next) = joint.next.as_deref() {
                visit(next, out);
            }
        }
        visit(&root, &mut joints);
        let mut triangles = 0;
        for joint in &joints {
            let mut display = joint.u.dobj();
            while let Some(dobj) = display {
                if let Some(material) = &dobj.mobj {
                    let textures = texture_decoder.read_chain(material.texdesc).unwrap();
                    for t in textures {
                        assert_eq!(
                            t.image.rgba.len(),
                            usize::from(t.image.width) * usize::from(t.image.height) * 4
                        );
                    }
                }
                let polygons = read_polygons(&archive, dobj.pobjdesc)
                    .unwrap_or_else(|e| panic!("{file}, joint {}: {e}", joint.offset));
                for p in polygons {
                    assert_eq!(p.indices.len() % 3, 0);
                    assert!(p.indices.iter().all(|&i| (i as usize) < p.vertices.len()));
                    for binding in &p.matrices {
                        use hsd_archive::visual::MatrixBinding::*;
                        let check = |id| {
                            assert!(
                                joints.iter().any(|j| j.offset == id),
                                "unknown skinning joint {id}"
                            )
                        };
                        match binding {
                            Owner => {}
                            Joint(id) => check(*id),
                            Envelope(weights) => weights.iter().for_each(|w| check(w.joint)),
                        }
                    }
                    triangles += p.indices.len() / 3;
                }
                display = dobj.next.as_deref();
            }
        }
        assert!(triangles > 1000, "{file} lost model geometry: {triangles}");
        eprintln!("{file}: {triangles} triangles, {} joints", joints.len());
    }
}

//! Costume-dependent portion of ftYs_Init_OnLoad (8012B99C).
//! Archive layout stays here; gameplay receives DObj indices and frame counts.
use hsd_archive::{desc::MatAnimJoint, Archive};
use melee_ft::{desc::FighterDescError, fighter::assets::Result};

/// ftParts_8007487C selects each costume's first two visibility groups,
/// falling back to costume zero when that group has no override.
pub fn egg_material_indices(
    data: &Archive,
) -> std::result::Result<Vec<Vec<usize>>, FighterDescError> {
    let root = data
        .public("ftDataYoshi")
        .expect("validated character root");
    let model = data.link(root + 8)?.expect("Yoshi model descriptor");
    let table = data.link(model + 4)?.expect("Yoshi visibility table");
    let mut costumes = Vec::new();
    for costume in 0..6 {
        let mut indices = Vec::new();
        for group in 0..2 {
            let selected = data
                .link(table + costume * 16 + group * 4)?
                .or(data.link(table + group * 4)?)
                .expect("Yoshi egg visibility group");
            let variant = data.link(selected + 4)?.expect("Yoshi egg model variants");
            let count = data.reader().u32(variant + 8)?;
            let list = data.link(variant + 12)?.expect("Yoshi egg DObj list");
            indices.extend(
                data.reader()
                    .slice(list, count)?
                    .iter()
                    .map(|&i| usize::from(i)),
            );
        }
        costumes.push(indices);
    }
    Ok(costumes)
}
/// ftYs_Init_8012B6E8 (8012B6E8): all selected MObj AObjs have the same end
/// frame, and their rate is frozen at zero. No TObj frame count is substituted.
pub fn egg_material_frames(archive: &Archive, symbol: &str, indices: &[usize]) -> Result<f32> {
    let root = archive
        .public(symbol)
        .ok_or("missing costume material animation")?;
    let tree = MatAnimJoint::read(archive, root)?;
    let mut ends = Vec::new();
    fn visit(archive: &Archive, joint: &MatAnimJoint, ends: &mut Vec<Option<f32>>) -> Result<()> {
        let mut material = joint.matanim;
        while let Some(offset) = material {
            let aobj = archive.link(offset + 4)?;
            ends.push(aobj.map(|a| archive.reader().f32(a + 4)).transpose()?);
            material = archive.link(offset)?;
        }
        if let Some(child) = &joint.child {
            visit(archive, child, ends)?;
        }
        if let Some(next) = &joint.next {
            visit(archive, next, ends)?;
        }
        Ok(())
    }
    visit(archive, &tree, &mut ends)?;
    let mut frames = 0.0;
    for &index in indices {
        let end = ends
            .get(index)
            .copied()
            .flatten()
            .ok_or("egg material has no AObj")?;
        if frames == 0.0 {
            frames = end;
        } else if frames != end {
            return Err("yoshi matanim frame not same (ftyoshi.c:97)".into());
        }
    }
    Ok(frames)
}

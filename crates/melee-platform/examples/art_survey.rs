//! Survey every texture in menu and interface archives: decode each GX image
//! to a PNG and write contact sheets and an index.html. For finding menu
//! art on the user's own extracted files; the output is game data, so it
//! goes under `target/` and is never committed.
//!
//! ```sh
//! cargo run -q --release -p melee-platform --example art_survey -- \
//!     harness/roms/files target/art-survey MnSlChr.usd MnSlMap.usd IfAll.usd
//! # what melee_platform::art returns, one sheet per kind, in target/art-survey/api:
//! cargo run -q --release -p melee-platform --example art_survey -- \
//!     harness/roms/files target/art-survey --api
//! ```
use hsd_archive::{desc::material_animation::TextureAnimation, visual::read_image, Archive};
use std::{collections::BTreeMap, fmt::Write as _, path::Path};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [files, out, names @ ..] = args.as_slice() else {
        return Err("usage: art_survey <files dir> <out dir> <archive>...".into());
    };
    std::fs::create_dir_all(out)?;
    if names.first().map(String::as_str) == Some("--api") {
        return api_sheets(Path::new(files), Path::new(out));
    }
    let mut index = String::from(
        "<!doctype html><meta charset=utf-8><title>Art survey</title>\
         <style>body{background:#334;color:#eee;font:12px sans-serif}\
         figure{display:inline-block;margin:4px;vertical-align:top;background:\
         repeating-conic-gradient(#666 0 25%,#999 0 50%) 0/16px 16px}\
         figcaption{background:#223}img{image-rendering:pixelated}</style>",
    );
    for name in names {
        let bytes = std::fs::read(Path::new(files).join(name))?;
        let archive = Archive::parse(&bytes)?;
        let stem = name.replace('.', "_");
        let dir = Path::new(out).join(&stem);
        std::fs::create_dir_all(&dir)?;
        writeln!(index, "<h1>{name}</h1><p>publics: ")?;
        for public in archive.publics() {
            write!(index, "{} @{:#x}; ", public.name, public.offset)?;
        }
        let survey = survey(&archive);
        let mut written = BTreeMap::new();
        let mut emit = |index: &mut String,
                        image: u32,
                        palette: Option<u32>|
         -> Result<(), Box<dyn std::error::Error>> {
            let key = (image, palette);
            if let std::collections::btree_map::Entry::Vacant(slot) = written.entry(key) {
                let file = match read_image(&archive, image, palette) {
                    Ok(texture) => {
                        let file = format!(
                            "{image:06x}_{}_{}x{}_f{}.png",
                            palette.map_or("-".into(), |p| format!("{p:x}")),
                            texture.width,
                            texture.height,
                            format_of(&archive, image)
                        );
                        std::fs::write(
                            dir.join(&file),
                            png(texture.width.into(), texture.height.into(), &texture.rgba),
                        )?;
                        Some((file, texture.width, texture.height))
                    }
                    Err(e) => {
                        eprintln!("{name} {image:#x}: {e}");
                        None
                    }
                };
                slot.insert(file);
            }
            if let Some(Some((file, w, h))) = written.get(&key) {
                write!(
                    index,
                    "<figure><img src='{stem}/{file}' width={}><figcaption>{image:#x} {w}x{h}</figcaption></figure>",
                    (*w).max(32)
                )?;
            }
            Ok(())
        };
        let mut sheets: BTreeMap<String, Vec<Decoded>> = BTreeMap::new();
        let decode = |image: u32, palette: Option<u32>| {
            read_image(&archive, image, palette)
                .ok()
                .map(|t| (t.width, t.height, t.rgba))
        };
        for (at, anim) in &survey.texanims {
            let sheet = sheets.entry(format!("texanim_{at:06x}")).or_default();
            for (i, image) in anim.images.iter().enumerate() {
                let palette = anim.palettes.get(i).or(anim.palettes.first()).copied().flatten();
                if let Some(decoded) = image.and_then(|image| decode(image, palette)) {
                    sheet.push(decoded);
                }
            }
        }
        for &image in &survey.images {
            if let Some(decoded) = decode(image, survey.palette_of.get(&image).copied()) {
                let key = format!("size_{}x{}", decoded.0, decoded.1);
                sheets.entry(key).or_default().push(decoded);
            }
        }
        for (key, images) in &sheets {
            let (w, h, rgba) = contact_sheet(images);
            std::fs::write(dir.join(format!("{key}.png")), png(w, h, &rgba))?;
        }
        for (at, anim) in &survey.texanims {
            writeln!(
                index,
                "<h2>{name} TexAnim @{at:#x} id {} ({} images, {} palettes)</h2>",
                anim.id,
                anim.images.len(),
                anim.palettes.len()
            )?;
            for (i, image) in anim.images.iter().enumerate() {
                let Some(image) = *image else { continue };
                let palette = anim
                    .palettes
                    .get(i)
                    .or(anim.palettes.first())
                    .copied()
                    .flatten();
                emit(&mut index, image, palette)?;
            }
        }
        writeln!(index, "<h2>{name}: every image</h2>")?;
        for &image in &survey.images {
            let palette = survey.palette_of.get(&image).copied();
            emit(&mut index, image, palette)?;
        }
        println!(
            "{name}: {} images, {} texanims, {} decoded",
            survey.images.len(),
            survey.texanims.len(),
            written.values().filter(|f| f.is_some()).count()
        );
    }
    std::fs::write(Path::new(out).join("index.html"), index)?;
    Ok(())
}

/// Contact sheets of what `melee_platform::art` returns, one per piece kind.
fn api_sheets(files: &Path, out: &Path) -> Result<(), Box<dyn std::error::Error>> {
    use melee_platform::art::{Art, ArtFile, Piece};
    use melee_platform::catalog;
    let mut art = Art::new();
    for file in ArtFile::ALL {
        art.insert(file, &std::fs::read(files.join(file.name()))?)?;
    }
    let mut sheets: BTreeMap<&str, Vec<Decoded>> = BTreeMap::new();
    let mut add = |sheet: &'static str, piece: Piece| match art.image(piece) {
        Ok(image) => sheets.entry(sheet).or_default().push((
            image.width as u16,
            image.height as u16,
            image.rgba.clone(),
        )),
        Err(e) => eprintln!("{piece:?}: {e}"),
    };
    for &character in catalog::characters() {
        for costume in 0..character.costume_count() {
            add("portraits", Piece::Portrait(character, costume));
            add("stocks", Piece::Stock(character, costume));
        }
        add("faces", Piece::Face(character));
        add("character_emblems", Piece::CharacterEmblem(character));
    }
    for &stage in catalog::stages() {
        add("stage_icons", Piece::StageIcon(stage));
        add("stage_names", Piece::StageName(stage));
        add("stage_emblems", Piece::StageEmblem(stage));
    }
    let dir = out.join("api");
    std::fs::create_dir_all(&dir)?;
    for (name, images) in &mut sheets {
        if *name == "stage_emblems" {
            // Retail draws these as faint watermarks (alpha peaks at 119).
            for (_, _, rgba) in images.iter_mut() {
                for a in rgba.iter_mut().skip(3).step_by(4) {
                    *a = a.saturating_mul(2);
                }
            }
        }
        let (w, h, rgba) = contact_sheet(images);
        std::fs::write(dir.join(format!("{name}.png")), png(w, h, &rgba))?;
        println!("{name}: {} images", images.len());
    }
    Ok(())
}

/// Width, height and RGBA8 pixels.
type Decoded = (u16, u16, Vec<u8>);

struct Survey {
    images: Vec<u32>,
    palette_of: BTreeMap<u32, u32>,
    texanims: Vec<(u32, TextureAnimation)>,
}

fn u32_at(data: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(data.get(at..at + 4)?.try_into().ok()?))
}
fn u16_at(data: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_be_bytes(data.get(at..at + 2)?.try_into().ok()?))
}
fn format_of(archive: &Archive, image: u32) -> u32 {
    u32_at(archive.data(), image as usize + 8).unwrap_or(99)
}

/// HSD_ImageDesc heuristics: a relocated pixel pointer, sane size and a GX
/// format, then TObjs (image at +0x4C, palette at +0x50) and TexAnims.
fn survey(archive: &Archive) -> Survey {
    let data = archive.data();
    let is_image = |at: u32| -> bool {
        let a = at as usize;
        if !archive.is_relocated_offset(at) {
            return false;
        }
        let (Some(w), Some(h), Some(f), Some(m)) = (
            u16_at(data, a + 4),
            u16_at(data, a + 6),
            u32_at(data, a + 8),
            u32_at(data, a + 12),
        ) else {
            return false;
        };
        (1..=1024).contains(&w)
            && (1..=1024).contains(&h)
            && matches!(f, 0..=6 | 8..=10 | 14)
            && m <= 1
    };
    let mut images = Vec::new();
    for at in (0..data.len() as u32).step_by(4) {
        if is_image(at) {
            images.push(at);
        }
    }
    let set: std::collections::BTreeSet<u32> = images.iter().copied().collect();
    let link = |at: u32| archive.link(at).ok().flatten();
    let mut palette_of = BTreeMap::new();
    let mut texanims = Vec::new();
    for at in (0..data.len() as u32).step_by(4) {
        // TObj: image at +76, palette at +80.
        if let Some(image) = link(at + 76) {
            if set.contains(&image) && archive.is_relocated_offset(at + 76) {
                if let Some(palette) = link(at + 80) {
                    palette_of.entry(image).or_insert(palette);
                }
            }
        }
        // TexAnim: next +0, id +4, aobj +8, images +12, palettes +16, counts +20/+22.
        if let (Some(table), Some(count)) = (link(at + 12), u16_at(data, at as usize + 20)) {
            if count > 0
                && count < 1024
                && link(table).is_some_and(|first| set.contains(&first))
                && u32_at(data, at as usize + 4).is_some_and(|id| id < 8)
            {
                if let Ok(chain) = TextureAnimation::read_chain(archive, Some(at)) {
                    if let Some(first) = chain.into_iter().next() {
                        for (i, image) in first.images.iter().enumerate() {
                            if let (Some(image), Some(Some(palette))) =
                                (image, first.palettes.get(i).or(first.palettes.first()))
                            {
                                palette_of.entry(*image).or_insert(*palette);
                            }
                        }
                        texanims.push((at, first));
                    }
                }
            }
        }
    }
    Survey {
        images,
        palette_of,
        texanims,
    }
}

/// Images in a grid of equal cells, 16 per row, on a checkerboard.
fn contact_sheet(images: &[Decoded]) -> (u32, u32, Vec<u8>) {
    let cw = images.iter().map(|i| u32::from(i.0)).max().unwrap_or(1) + 2;
    let ch = images.iter().map(|i| u32::from(i.1)).max().unwrap_or(1) + 2;
    let cols = (images.len() as u32).clamp(1, 16);
    let rows = (images.len() as u32).div_ceil(cols).max(1);
    let (w, h) = (cw * cols, ch * rows);
    let mut out = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let v = if ((x / 8) + (y / 8)) % 2 == 0 { 90 } else { 140 };
            out[((y * w + x) * 4) as usize..][..4].copy_from_slice(&[v, v, v, 255]);
        }
    }
    for (n, (iw, ih, rgba)) in images.iter().enumerate() {
        let (ox, oy) = ((n as u32 % cols) * cw + 1, (n as u32 / cols) * ch + 1);
        for y in 0..u32::from(*ih) {
            for x in 0..u32::from(*iw) {
                let s = &rgba[((y * u32::from(*iw) + x) * 4) as usize..][..4];
                let d = &mut out[(((oy + y) * w + ox + x) * 4) as usize..][..4];
                let a = u32::from(s[3]);
                for c in 0..3 {
                    d[c] = ((u32::from(s[c]) * a + u32::from(d[c]) * (255 - a)) / 255) as u8;
                }
            }
        }
    }
    (w, h, out)
}

/// A minimal PNG: RGBA8, zlib stored blocks.
fn png(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    let mut raw = Vec::with_capacity(rgba.len() + height as usize);
    for row in rgba.chunks_exact(width as usize * 4) {
        raw.push(0);
        raw.extend_from_slice(row);
    }
    let mut z = vec![0x78, 0x01];
    let mut chunks = raw.chunks(65535).peekable();
    while let Some(block) = chunks.next() {
        z.push(u8::from(chunks.peek().is_none()));
        z.extend((block.len() as u16).to_le_bytes());
        z.extend((!(block.len() as u16)).to_le_bytes());
        z.extend_from_slice(block);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for &byte in &raw {
        a = (a + u32::from(byte)) % 65521;
        b = (b + a) % 65521;
    }
    z.extend(((b << 16) | a).to_be_bytes());
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut ihdr = Vec::new();
    ihdr.extend(width.to_be_bytes());
    ihdr.extend(height.to_be_bytes());
    ihdr.extend([8, 6, 0, 0, 0]);
    for (kind, body) in [(b"IHDR", ihdr), (b"IDAT", z), (b"IEND", Vec::new())] {
        out.extend((body.len() as u32).to_be_bytes());
        let mut crc_input = kind.to_vec();
        crc_input.extend(&body);
        out.extend_from_slice(&crc_input);
        out.extend(crc32(&crc_input).to_be_bytes());
    }
    out
}
fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

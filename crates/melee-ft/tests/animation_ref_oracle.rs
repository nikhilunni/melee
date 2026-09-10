use hsd_anim::{
    aobj::{AObj, AOBJ_FIRST_PLAY, AOBJ_LOOP, AOBJ_NO_ANIM},
    jobj::{JObjTree, JointSpec},
    mtx::InverseTrig,
};
use melee_ft::anim::{FighterAnimation, MotionFlags};
use std::{
    fs,
    path::Path,
    process::{Command, Stdio},
};

struct NoMatrixTracks;
impl InverseTrig for NoMatrixTracks {
    fn atan2f(_: f32, _: f32) -> f32 {
        panic!("oracle has no matrix tracks")
    }
    fn asinf(_: f32) -> f32 {
        panic!("oracle has no matrix tracks")
    }
    fn acosf(_: f32) -> f32 {
        panic!("oracle has no matrix tracks")
    }
}
fn extract_function(source: &str, name: &str) -> String {
    let start = source.find(&format!("void {name}(")).unwrap();
    let body = source[start..].find('{').unwrap() + start;
    let mut nesting = 1;
    let mut end = body + 1;
    while nesting != 0 {
        match source.as_bytes()[end] {
            b'{' => nesting += 1,
            b'}' => nesting -= 1,
            _ => {}
        }
        end += 1;
    }
    format!("{}\n", &source[start..end])
}
#[test]
fn arithmetic_oracle_excerpts_match_decomp() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let decomp = root.join("../../third_party/melee-decomp/src");
    if !decomp.exists() {
        eprintln!("[NON-DATA OMITTED] omitting excerpt comparison: submodule absent");
        return;
    }
    for (path, name, local) in [
        ("melee/ft/ftanim.c", "ftAnim_8006E9B4", "ftanim.c.inc"),
        (
            "sysdolphin/baselib/aobj.c",
            "HSD_AObjInterpretAnim",
            "aobj.c.inc",
        ),
    ] {
        assert_eq!(
            fs::read_to_string(root.join("tests/ref/animation").join(local)).unwrap(),
            extract_function(&fs::read_to_string(decomp.join(path)).unwrap(), name)
        );
    }
}
#[test]
fn frame_step_matches_native_c_100k() {
    let directory = Path::new(env!("CARGO_TARGET_TMPDIR")).join("fighter-animation-oracle");
    fs::create_dir_all(&directory).unwrap();
    let executable = directory.join("driver");
    let compiler = std::env::var("CC").unwrap_or_else(|_| "cc".into());
    let build = Command::new(compiler)
        .args([
            "-std=c99",
            "-O0",
            "-ffp-contract=off",
            "-fno-builtin",
            "-fno-strict-aliasing",
        ])
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/ref/animation/driver.c"))
        .arg("-lm")
        .arg("-o")
        .arg(&executable)
        .output()
        .expect("native C compiler required for the arithmetic oracle");
    assert!(
        build.status.success(),
        "{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let mut seed = 0x41a9_16bdu32;
    let mut random = || {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        seed
    };
    let mut inputs = Vec::new();
    for index in 0..100_000 {
        let frame = if index % 3 == 0 {
            (random() % 65536) as f32 / 32.0 - 128.0
        } else {
            f32::from_bits((random() & 0x807f_ffff) | ((120 + random() % 20) << 23))
        };
        let speed = if index % 3 == 0 {
            (random() % 1024) as f32 / 64.0 - 4.0
        } else {
            f32::from_bits((random() & 0x807f_ffff) | ((100 + random() % 35) << 23))
        };
        let end = (random() % 8192 + 1) as f32 / 16.0;
        let rewind = match index % 5 {
            0 => end,
            1 => end + 1.0,
            _ => -16.0,
        };
        let remainder = f32::from_bits((random() & 0x007f_ffff) | ((110 + random() % 40) << 23));
        let duration = if index % 3 == 0 {
            0.0
        } else {
            (random() % 32 + 1) as f32
        };
        let progress = (random() % 1024) as f32 / 32.0;
        let mut flags = if index % 2 == 0 { AOBJ_LOOP } else { 0 };
        if index % 7 == 0 {
            flags |= AOBJ_FIRST_PLAY;
        }
        if index % 11 == 0 {
            flags |= AOBJ_NO_ANIM;
        }
        inputs.push([
            frame.to_bits(),
            speed.to_bits(),
            end.to_bits(),
            rewind.to_bits(),
            remainder.to_bits(),
            duration.to_bits(),
            progress.to_bits(),
            flags,
            u32::from(index % 4 != 0),
        ]);
    }
    let bytes: Vec<u8> = inputs
        .iter()
        .flatten()
        .flat_map(|x| x.to_ne_bytes())
        .collect();
    let input_path = directory.join("inputs.bin");
    fs::write(&input_path, bytes).unwrap();
    let output = Command::new(&executable)
        .stdin(Stdio::from(fs::File::open(input_path).unwrap()))
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout.len(), inputs.len() * 16);
    for (index, input) in inputs.iter().enumerate() {
        let values = input[..7]
            .iter()
            .copied()
            .map(f32::from_bits)
            .collect::<Vec<_>>();
        let mut tree = JObjTree::new();
        let root = tree.load_joint(&JointSpec::new());
        let mut state = FighterAnimation::new(&tree, root);
        state.motion_id = 2;
        state.frame = values[0];
        state.speed = values[1];
        state.remainder = values[4];
        state.blend_duration = values[5];
        state.blend_progress = values[6];
        state.flags = MotionFlags(if input[8] != 0 {
            MotionFlags::ACCUMULATE_LOOPS
        } else {
            0
        });
        let aobj = AObj {
            flags: input[7],
            curr_frame: values[0],
            framerate: values[1],
            end_frame: values[2],
            rewind_frame: values[3],
            ..AObj::default()
        };
        if values[5] == 0.0 {
            tree.get_mut(root).aobj = Some(aobj);
        } else {
            state.blend_tree.get_mut(root).aobj = Some(aobj);
        }
        state.step::<NoMatrixTracks>(&mut tree);
        let actual = [
            state.frame.to_bits(),
            state.remainder.to_bits(),
            state.blend_progress.to_bits(),
            state.current_aobj(&tree).unwrap().flags,
        ];
        let expected: Vec<_> = output.stdout[index * 16..index * 16 + 16]
            .chunks_exact(4)
            .map(|x| u32::from_ne_bytes(x.try_into().unwrap()))
            .collect();
        assert_eq!(
            actual.as_slice(),
            expected,
            "case {index}: input {input:08x?}, actual {actual:08x?}"
        );
    }
    eprintln!("100000 frame/loop/remainder/blend inputs: zero bit mismatches vs native C");
}

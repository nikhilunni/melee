use std::{path::Path, process::Command};

// Environment cases run in separate processes so parallel tests never mutate
// one another's policy. The child verifies that false really returns early.
#[test]
fn policy_child() {
    let Ok(root) = std::env::var("MELEE_POLICY_FIXTURE") else {
        assert!(melee_test_support::require_files([Path::new(env!(
            "CARGO_MANIFEST_DIR"
        ))
        .join("Cargo.toml")]));
        return;
    };
    if !melee_test_support::require_files([
        Path::new(&root).join("harness/traces/first.tick.expected.jsonl"),
        Path::new(&root).join("harness/traces/second.tick.expected.jsonl"),
    ]) {
        return;
    }
    println!("oracle body reached");
}

#[test]
fn missing_files_fail_unless_explicitly_opted_out() {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("missing-data-policy-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let run = |allow: Option<&str>, override_root: Option<&Path>| {
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args(["--exact", "policy_child", "--nocapture"])
            .env("MELEE_POLICY_FIXTURE", &root)
            .env_remove("MELEE_ALLOW_MISSING_DATA")
            .env_remove("MELEE_TEST_DATA_ROOT");
        if let Some(value) = allow {
            command.env("MELEE_ALLOW_MISSING_DATA", value);
        }
        if let Some(path) = override_root {
            command.env("MELEE_TEST_DATA_ROOT", path);
        }
        let output = command.output().unwrap();
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        (output.status.success(), text)
    };
    for allow in [None, Some(""), Some("0"), Some("true"), Some("1")] {
        let (success, text) = run(allow, None);
        assert_eq!(success, allow == Some("1"), "{text}");
        assert!(text.contains("first.tick.expected.jsonl"), "{text}");
        assert!(!text.contains("second.tick.expected.jsonl"), "{text}");
        assert!(
            text.contains("harness/record.py harness/scenarios/first.toml"),
            "{text}"
        );
        assert!(!text.contains("oracle body reached"), "{text}");
    }
    let traces = root.join("harness/traces");
    std::fs::create_dir_all(&traces).unwrap();
    for name in ["first", "second"] {
        std::fs::write(
            traces.join(format!("{name}.tick.expected.jsonl")),
            "fixture",
        )
        .unwrap();
    }
    let (success, text) = run(None, None);
    assert!(success && text.contains("oracle body reached"), "{text}");
    let empty = root.join("empty");
    std::fs::create_dir(&empty).unwrap();
    let (success, text) = run(None, Some(&empty));
    assert!(!success, "{text}");
    assert!(
        text.contains(
            &empty
                .join("traces/first.tick.expected.jsonl")
                .display()
                .to_string()
        ),
        "{text}"
    );
    let (success, text) = run(Some("1"), Some(&empty));
    assert!(success && !text.contains("oracle body reached"), "{text}");
    // A directory or broken symlink must never count as a file.
    let first = traces.join("first.tick.expected.jsonl");
    std::fs::remove_file(&first).unwrap();
    std::fs::create_dir(&first).unwrap();
    assert!(!run(None, None).0);
    #[cfg(unix)]
    {
        std::fs::remove_dir(&first).unwrap();
        std::os::unix::fs::symlink("absent", &first).unwrap();
        assert!(!run(None, None).0);
    }
    std::fs::remove_dir_all(&root).unwrap();
}

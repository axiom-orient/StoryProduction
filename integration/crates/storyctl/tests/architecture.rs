//! Guard actual dependency/source direction against future core pollution.
use std::path::{Path, PathBuf};
use std::process::Command;
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf()
}
fn rust_sources(directory: &Path) -> Vec<PathBuf> {
    let mut result = vec![];
    for entry in std::fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            result.extend(rust_sources(&path));
        } else if path.extension().is_some_and(|v| v == "rs") {
            result.push(path);
        }
    }
    result.sort();
    result
}
#[test]
fn pure_core_has_no_effects_json_or_owner_dependencies() {
    let core = root().join("crates/story-integration-core");
    for path in rust_sources(&core.join("src")) {
        let source = std::fs::read_to_string(&path).unwrap();
        for forbidden in [
            "std::fs",
            "std::process",
            "std::net",
            "std::time",
            "serde_json",
            "geultu::",
            "sts_workflow::",
            "narrative_courier",
            "Jangmyeondo",
            "VisualScreenCompiler",
        ] {
            assert!(
                !source.contains(forbidden),
                "{} imports forbidden {forbidden}",
                path.display()
            );
        }
    }
    // Cargo's resolved declarations include aliases, build/target/dev dependencies
    // and workspace inheritance that a substring check on one manifest misses.
    let output = Command::new(env!("CARGO"))
        .args([
            "metadata",
            "--format-version",
            "1",
            "--no-deps",
            "--locked",
            "--offline",
            "--manifest-path",
        ])
        .arg(root().join("Cargo.toml"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let metadata: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    for package in metadata["packages"].as_array().unwrap() {
        let dependencies = package["dependencies"].as_array().unwrap();
        if package["name"] == "story-integration-core" {
            assert!(
                dependencies.is_empty(),
                "core must not acquire dependencies"
            );
        }
        for dependency in dependencies {
            let name = dependency["name"].as_str().unwrap();
            for forbidden in [
                "geultu",
                "narrative-courier",
                "sts-workflow",
                "scene-asset",
                "sac-animatic",
                "smc-",
                "promptgen",
                "jangmyeondo",
                "visual-screen-compiler",
            ] {
                assert!(
                    !name.to_lowercase().contains(forbidden),
                    "owner implementation linked into integration: {name}"
                );
            }
            if let Some(path) = dependency["path"].as_str() {
                assert!(
                    Path::new(path)
                        .canonicalize()
                        .unwrap()
                        .starts_with(root().canonicalize().unwrap()),
                    "local dependency escapes integration: {path}"
                );
            }
        }
    }
}

#[test]
fn unsafe_boundary_and_retired_dispatcher_remain_enforced() {
    let runtime = root().join("crates/storyctl/src");
    assert!(std::fs::read_to_string(runtime.join("lib.rs"))
        .unwrap()
        .contains("#![deny(unsafe_code)]"));
    for path in rust_sources(&runtime) {
        if !path.starts_with(runtime.join("effects")) {
            let source = std::fs::read_to_string(&path).unwrap();
            assert!(
                !source.contains("allow(unsafe_code)"),
                "unsafe exemption outside effects: {}",
                path.display()
            );
        }
    }
    for relative in [
        "scripts/storyctl.py",
        "src/story_integration",
        "requirements-optional-schema.txt",
    ] {
        assert!(
            !root().join(relative).exists(),
            "retired runtime returned: {relative}"
        );
    }
}

//! Local runtime-root integrity and assembly fixtures. No component provider
//! or user workspace is called by these tests.

use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    fs,
    os::unix::fs::{symlink, MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
};
use storyctl::effects::hash;
use storyctl::runtime::{self, RootMode};

const SOURCE_STORYCTL: &str = "integration/target/release/storyctl";
const STORYCTL_ENTRY: &str = "bin/storyctl";
const GEULTU_BIN: &str = "geultu/target/release/geultu";
const NVC_BIN: &str = "NarrativeCourier/target/release/narrative-courier-cli";
const SAC_BIN: &str = "SceneAssetCompiler/target/release/sac-animatic";
const SMC_BIN: &str = "SceneMotionCompiler/target/release/smc-animatic";
const PROMPTGEN_BIN: &str = "ImagePromptCompiler/target/release/promptgen";
const PROMPTGEN_PAYLOAD: &str = "ImagePromptCompiler/examples/image-cinematic-storyboard.json";
const BASELINE_CRAFT_PACK: &str = "geultu/assets/craft-packs-story-baseline-v1/craft-pack.json";

struct SourceFixture {
    _temp: tempfile::TempDir,
    root: PathBuf,
}

impl SourceFixture {
    fn new(include_promptgen: bool) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("source");
        fs::create_dir(&root).unwrap();

        for path in [SOURCE_STORYCTL, GEULTU_BIN, NVC_BIN, SAC_BIN, SMC_BIN] {
            write_file(&root, path, &macho_arm64(), 0o755);
        }
        write_file(
            &root,
            "ImagePromptCompiler/schemas/compilation.schema.json",
            br#"{"type":"object","title":"PromptGen producer schema"}"#,
            0o644,
        );
        write_file(
            &root,
            "SceneAssetCompiler/.agents/skills/scene-asset-compiler/schemas/run-request.schema.json",
            br#"{"type":"object","title":"SAC hidden producer schema"}"#,
            0o644,
        );

        let mut entries = Vec::new();
        for index in 0..33 {
            let producer = match index {
                0 => "ImagePromptCompiler/schemas/compilation.schema.json".to_owned(),
                1 => "SceneAssetCompiler/.agents/skills/scene-asset-compiler/schemas/run-request.schema.json".to_owned(),
                value => format!("integration/contracts/producers/p{value:02}.json"),
            };
            if index >= 2 {
                write_file(
                    &root,
                    &producer,
                    format!("{{\"producer\":{index}}}\n").as_bytes(),
                    0o644,
                );
            }
            let mut entry = json!({
                "id": format!("fixture.contract.{index:02}"),
                "owner": "fixture",
                "producer": producer,
                "verification": if index < 30 { "byte_equal" } else { "schema" }
            });
            if index < 30 {
                let consumer = format!("integration/contracts/consumers/c{index:02}.json");
                write_file(
                    &root,
                    &consumer,
                    &fs::read(root.join(&producer)).unwrap(),
                    0o644,
                );
                entry["consumers"] = json!([consumer]);
            }
            entries.push(entry);
        }
        write_json(
            &root,
            "integration/contracts/registry.json",
            &json!({"schemaVersion":"story.contract-registry/1","contracts":entries}),
        );
        write_json(
            &root,
            "integration/contracts/examples/capability-request.prompt-compile.json",
            &json!({"capability":"image.prompt.compile","payload":"ImagePromptCompiler/examples/image-cinematic-storyboard.json"}),
        );
        write_file(
            &root,
            "NarrativeCourier/skills/sts/SKILL.md",
            b"public NVC skill fixture\n",
            0o644,
        );
        write_file(
            &root,
            BASELINE_CRAFT_PACK,
            br#"{"id":"story-baseline-v1"}"#,
            0o644,
        );
        write_file(
            &root,
            "geultu/assets/craft-packs-story-baseline-v1/LICENSE",
            b"Story baseline pack license\n",
            0o644,
        );
        for path in [
            "geultu/LICENSE",
            "NarrativeCourier/LICENSE",
            "NarrativeCourier/NOTICE.md",
            "SceneMotionCompiler/LICENSE",
            "ImagePromptCompiler/LICENSE",
            "ImagePromptCompiler/NOTICE.md",
        ] {
            write_file(
                &root,
                path,
                format!("source notice: {path}\n").as_bytes(),
                0o644,
            );
        }
        write_file(
            &root,
            runtime::SMC_ANIMATIC_RUNTIME_JS,
            b"export function frameFixture(){ return 1; }\n",
            0o644,
        );
        write_file(
            &root,
            runtime::SMC_TRACKB_NAVIGATION_GUARD_JS,
            b"export function navigationFixture(){ return true; }\n",
            0o644,
        );
        if include_promptgen {
            write_file(&root, PROMPTGEN_BIN, &macho_arm64(), 0o755);
            write_json(
                &root,
                PROMPTGEN_PAYLOAD,
                &json!({"kind":"image","output":{"width":16,"height":16}}),
            );
        }
        Self {
            _temp: temp,
            root: root.canonicalize().unwrap(),
        }
    }
}

fn macho_arm64() -> Vec<u8> {
    let mut bytes = vec![0u8; 56];
    bytes[..4].copy_from_slice(&[0xcf, 0xfa, 0xed, 0xfe]);
    bytes[4..8].copy_from_slice(&0x0100_000cu32.to_le_bytes());
    bytes[12..16].copy_from_slice(&2u32.to_le_bytes());
    bytes[16..20].copy_from_slice(&1u32.to_le_bytes());
    bytes[20..24].copy_from_slice(&24u32.to_le_bytes());
    bytes[32..36].copy_from_slice(&0x32u32.to_le_bytes());
    bytes[36..40].copy_from_slice(&24u32.to_le_bytes());
    bytes[40..44].copy_from_slice(&1u32.to_le_bytes());
    bytes
}

fn write_file(root: &Path, relative: &str, bytes: &[u8], mode: u32) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
}

fn write_json(root: &Path, relative: &str, value: &Value) {
    write_file(root, relative, &serde_json::to_vec(value).unwrap(), 0o644);
}

fn child_path(parent: &Path, leaf: &str) -> PathBuf {
    parent.canonicalize().unwrap().join(leaf)
}

fn bundle(fixture: &SourceFixture, include_promptgen: bool) -> (tempfile::TempDir, PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().join("bundle");
    fs::create_dir(&output).unwrap();
    let output = output.canonicalize().unwrap();

    copy_tree(
        &fixture.root,
        "integration/contracts",
        &output,
        "integration/contracts",
    );
    copy_tree(
        &fixture.root,
        "NarrativeCourier/skills/sts",
        &output,
        "NarrativeCourier/skills/sts",
    );
    copy_tree(
        &fixture.root,
        "geultu/assets/craft-packs-story-baseline-v1",
        &output,
        "geultu/assets/craft-packs-story-baseline-v1",
    );
    for (source, destination) in [
        (SOURCE_STORYCTL, STORYCTL_ENTRY),
        (GEULTU_BIN, GEULTU_BIN),
        (NVC_BIN, NVC_BIN),
        (SAC_BIN, SAC_BIN),
        (SMC_BIN, SMC_BIN),
    ] {
        copy_file(&fixture.root, source, &output, destination);
    }
    for path in [
        "geultu/LICENSE",
        "NarrativeCourier/LICENSE",
        "NarrativeCourier/NOTICE.md",
        "SceneMotionCompiler/LICENSE",
        "ImagePromptCompiler/LICENSE",
        "ImagePromptCompiler/NOTICE.md",
    ] {
        copy_file(&fixture.root, path, &output, path);
    }
    for path in [
        runtime::SMC_ANIMATIC_RUNTIME_JS,
        runtime::SMC_TRACKB_NAVIGATION_GUARD_JS,
    ] {
        copy_file(&fixture.root, path, &output, path);
    }
    if include_promptgen {
        copy_file(&fixture.root, PROMPTGEN_BIN, &output, PROMPTGEN_BIN);
        copy_file(&fixture.root, PROMPTGEN_PAYLOAD, &output, PROMPTGEN_PAYLOAD);
    }
    let registry: Value = serde_json::from_slice(
        &fs::read(fixture.root.join("integration/contracts/registry.json")).unwrap(),
    )
    .unwrap();
    for entry in registry["contracts"].as_array().unwrap() {
        let producer = entry["producer"].as_str().unwrap();
        if !output.join(producer).exists() {
            copy_file(&fixture.root, producer, &output, producer);
        }
        for consumer in entry["consumers"].as_array().into_iter().flatten() {
            let consumer = consumer.as_str().unwrap();
            if !output.join(consumer).exists() {
                copy_file(&fixture.root, consumer, &output, consumer);
            }
        }
    }
    set_child_directory_modes(&output);
    let mut file_entries = BTreeMap::new();
    collect_manifest_files(&output, &output, &mut file_entries);
    let manifest_files: Vec<_> = file_entries.into_values().collect();
    write_file(
        &output,
        "runtime-manifest.json",
        &serde_json::to_vec(&json!({
            "schemaVersion":"story.runtime-manifest/1",
            "supportedTarget":"macos-arm64",
            "includePromptgen":include_promptgen,
            "componentVersions": if include_promptgen {
                json!({"storyctl":"0.1.0","geultu":"5.0.0","promptgen":"1.5.0"})
            } else {
                json!({"storyctl":"0.1.0","geultu":"5.0.0"})
            },
            "qualification":{"integrity":"local_integrity_only","commercialRedistribution":"UNKNOWN","signing":"UNKNOWN"},
            "files":manifest_files
        }))
        .unwrap(),
        0o444,
    );
    fs::set_permissions(&output, fs::Permissions::from_mode(0o555)).unwrap();
    (temp, output)
}

fn copy_file(source_root: &Path, source: &str, output_root: &Path, destination: &str) {
    let source_path = source_root.join(source);
    let destination_path = output_root.join(destination);
    fs::create_dir_all(destination_path.parent().unwrap()).unwrap();
    let bytes = fs::read(&source_path).unwrap();
    fs::write(&destination_path, bytes).unwrap();
    let source_mode = fs::symlink_metadata(&source_path).unwrap().mode() & 0o777;
    fs::set_permissions(
        destination_path,
        fs::Permissions::from_mode(source_mode & !0o222),
    )
    .unwrap();
}

fn copy_tree(source_root: &Path, source: &str, output_root: &Path, destination: &str) {
    let source_path = source_root.join(source);
    for entry in fs::read_dir(source_path).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name().into_string().unwrap();
        let source_relative = format!("{source}/{name}");
        let destination_relative = format!("{destination}/{name}");
        let metadata = fs::symlink_metadata(entry.path()).unwrap();
        if metadata.is_dir() {
            copy_tree(
                source_root,
                &source_relative,
                output_root,
                &destination_relative,
            );
        } else {
            assert!(metadata.is_file());
            copy_file(
                source_root,
                &source_relative,
                output_root,
                &destination_relative,
            );
        }
    }
}

fn set_child_directory_modes(path: &Path) {
    for entry in fs::read_dir(path).unwrap() {
        let entry = entry.unwrap();
        let metadata = fs::symlink_metadata(entry.path()).unwrap();
        if metadata.is_dir() {
            set_child_directory_modes(&entry.path());
            fs::set_permissions(entry.path(), fs::Permissions::from_mode(0o555)).unwrap();
        }
    }
}

fn collect_manifest_files(root: &Path, path: &Path, entries: &mut BTreeMap<String, Value>) {
    for entry in fs::read_dir(path).unwrap() {
        let entry = entry.unwrap();
        let metadata = fs::symlink_metadata(entry.path()).unwrap();
        if metadata.is_dir() {
            collect_manifest_files(root, &entry.path(), entries);
        } else {
            assert!(metadata.is_file());
            let bytes = fs::read(entry.path()).unwrap();
            let relative = entry
                .path()
                .strip_prefix(root)
                .unwrap()
                .to_str()
                .unwrap()
                .replace('\\', "/");
            entries.insert(
                relative.clone(),
                json!({
                    "path":relative,
                    "size":bytes.len(),
                    "sha256":hash::sha256_bytes(&bytes),
                    "unixMode":metadata.mode() & 0o777
                }),
            );
        }
    }
}

fn overwrite_manifest(root: &Path, value: &Value) {
    let path = root.join("runtime-manifest.json");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
    fs::set_permissions(
        root.join("runtime-manifest.json"),
        fs::Permissions::from_mode(0o444),
    )
    .unwrap();
}

fn overwrite_bundle_file(root: &Path, relative: &str, bytes: &[u8]) {
    let path = root.join(relative);
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    fs::write(&path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o444)).unwrap();
}

fn manifest(root: &Path) -> Value {
    serde_json::from_slice(&fs::read(root.join("runtime-manifest.json")).unwrap()).unwrap()
}

#[test]
fn explicit_source_root_remains_valid_with_a_bin_entry() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    write_file(&source, "integration/contracts/registry.json", b"{}", 0o644);
    write_file(&source, STORYCTL_ENTRY, &macho_arm64(), 0o755);
    assert_eq!(
        runtime::check_root(&source.canonicalize().unwrap()).unwrap(),
        RootMode::Source
    );

    let missing_marker = temp.path().join("missing-marker");
    write_file(&missing_marker, STORYCTL_ENTRY, &macho_arm64(), 0o755);
    assert!(runtime::check_root(&missing_marker.canonicalize().unwrap()).is_err());
}

#[test]
fn assemble_preserves_pins_hidden_schema_and_optional_promptgen_boundary() {
    let source_without_promptgen = SourceFixture::new(false);
    let (_without_temp, without) = bundle(&source_without_promptgen, false);
    assert_eq!(runtime::check_root(&without).unwrap(), RootMode::Installed);
    let report = runtime::verify(&without).unwrap();
    assert_eq!(report["status"], "PASS");
    assert_eq!(report["contractCount"], 33);
    assert_eq!(report["consumerPinCount"], 30);
    assert_eq!(report["includePromptgen"], false);
    assert!(!without.join(PROMPTGEN_BIN).exists());
    assert!(!without.join(PROMPTGEN_PAYLOAD).exists());
    assert!(without
        .join("ImagePromptCompiler/schemas/compilation.schema.json")
        .is_file());
    assert!(without
        .join("SceneAssetCompiler/.agents/skills/scene-asset-compiler/schemas/run-request.schema.json")
        .is_file());
    assert!(without.join(runtime::SMC_ANIMATIC_RUNTIME_JS).is_file());
    assert!(without
        .join(runtime::SMC_TRACKB_NAVIGATION_GUARD_JS)
        .is_file());
    assert!(without.join("ImagePromptCompiler/LICENSE").is_file());
    assert!(without.join("ImagePromptCompiler/NOTICE.md").is_file());

    let source_with_promptgen = SourceFixture::new(true);
    let (_with_temp, with) = bundle(&source_with_promptgen, true);
    assert_eq!(runtime::verify(&with).unwrap()["includePromptgen"], true);
    assert!(with.join(PROMPTGEN_BIN).is_file());
    assert!(with.join(PROMPTGEN_PAYLOAD).is_file());
    assert_eq!(fs::read(with.join("bin/storyctl")).unwrap(), macho_arm64());
}

#[test]
fn assembly_rejects_missing_required_and_optional_inputs_before_reserving_output() {
    let fixture = SourceFixture::new(false);
    let output = child_path(fixture._temp.path(), "missing-promptgen-bundle");
    assert!(runtime::assemble(&fixture.root, &output, true).is_err());
    assert!(!output.exists());

    let missing_required = fixture.root.join(NVC_BIN);
    fs::remove_file(&missing_required).unwrap();
    let output = child_path(fixture._temp.path(), "missing-nvc-bundle");
    assert!(runtime::assemble(&fixture.root, &output, false).is_err());
    assert!(!output.exists());

    let fixture = SourceFixture::new(false);
    fs::remove_file(fixture.root.join(runtime::SMC_ANIMATIC_RUNTIME_JS)).unwrap();
    let output = child_path(fixture._temp.path(), "missing-smc-runtime-bundle");
    assert!(runtime::assemble(&fixture.root, &output, false).is_err());
    assert!(!output.exists());
}

#[test]
fn verify_requires_both_smc_renderer_resources_and_checks_their_exact_bytes() {
    for path in [
        runtime::SMC_ANIMATIC_RUNTIME_JS,
        runtime::SMC_TRACKB_NAVIGATION_GUARD_JS,
    ] {
        let fixture = SourceFixture::new(false);
        let (_temp, output) = bundle(&fixture, false);
        let asset = output.join(path);
        let parent = asset.parent().unwrap();
        fs::set_permissions(parent, fs::Permissions::from_mode(0o755)).unwrap();
        fs::remove_file(&asset).unwrap();
        fs::set_permissions(parent, fs::Permissions::from_mode(0o555)).unwrap();
        assert!(runtime::verify(&output).is_err(), "missing {path}");

        let fixture = SourceFixture::new(false);
        let (_temp, output) = bundle(&fixture, false);
        let asset = output.join(path);
        let mut corrupt = fs::read(&asset).unwrap();
        corrupt.push(b' ');
        overwrite_bundle_file(&output, path, &corrupt);
        assert!(runtime::verify(&output).is_err(), "modified {path}");

        let fixture = SourceFixture::new(false);
        let (_temp, output) = bundle(&fixture, false);
        let mut value = manifest(&output);
        value["files"]
            .as_array_mut()
            .unwrap()
            .retain(|entry| entry["path"] != path);
        overwrite_manifest(&output, &value);
        assert!(
            runtime::verify(&output).is_err(),
            "unlisted required {path}"
        );
    }
}

#[test]
fn verify_rejects_modified_bytes_modes_symlinks_and_unlisted_files() {
    let fixture = SourceFixture::new(false);
    let (_temp, output) = bundle(&fixture, false);
    let license = output.join("geultu/LICENSE");
    let original = fs::read(&license).unwrap();
    overwrite_bundle_file(&output, "geultu/LICENSE", b"modified\n");
    assert!(runtime::verify(&output).is_err());
    overwrite_bundle_file(&output, "geultu/LICENSE", &original);

    fs::set_permissions(&license, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(runtime::verify(&output).is_err());
    fs::set_permissions(&license, fs::Permissions::from_mode(0o444)).unwrap();

    let external = output.parent().unwrap().join("external-license");
    fs::write(&external, b"external\n").unwrap();
    let geultu_dir = output.join("geultu");
    fs::set_permissions(&geultu_dir, fs::Permissions::from_mode(0o755)).unwrap();
    fs::remove_file(&license).unwrap();
    symlink(&external, &license).unwrap();
    fs::set_permissions(&geultu_dir, fs::Permissions::from_mode(0o555)).unwrap();
    assert!(runtime::verify(&output).is_err());
    fs::set_permissions(&geultu_dir, fs::Permissions::from_mode(0o755)).unwrap();
    fs::remove_file(&license).unwrap();
    fs::copy(&external, &license).unwrap();
    fs::set_permissions(&license, fs::Permissions::from_mode(0o444)).unwrap();
    fs::set_permissions(&geultu_dir, fs::Permissions::from_mode(0o555)).unwrap();

    fs::set_permissions(&output, fs::Permissions::from_mode(0o755)).unwrap();
    write_file(&output, "unexpected/extra.txt", b"extra\n", 0o644);
    fs::set_permissions(output.join("unexpected"), fs::Permissions::from_mode(0o555)).unwrap();
    fs::set_permissions(&output, fs::Permissions::from_mode(0o555)).unwrap();
    assert!(runtime::verify(&output).is_err());
}

#[test]
fn verify_rejects_unknown_manifest_fields_duplicate_paths_and_unsafe_paths() {
    let fixture = SourceFixture::new(false);
    let (_temp, output) = bundle(&fixture, false);

    let mut value = manifest(&output);
    value["unexpected"] = json!(true);
    overwrite_manifest(&output, &value);
    assert!(runtime::verify(&output).is_err());

    let (_temp, output) = bundle(&fixture, false);
    let mut value = manifest(&output);
    value["componentVersions"]
        .as_object_mut()
        .unwrap()
        .remove("storyctl");
    overwrite_manifest(&output, &value);
    assert!(runtime::verify(&output).is_err());

    let (_temp, output) = bundle(&fixture, false);
    let mut value = manifest(&output);
    let first = value["files"][0].clone();
    value["files"].as_array_mut().unwrap().push(first);
    value["files"]
        .as_array_mut()
        .unwrap()
        .sort_by(|left, right| left["path"].as_str().cmp(&right["path"].as_str()));
    overwrite_manifest(&output, &value);
    assert!(runtime::verify(&output).is_err());

    let (_temp, output) = bundle(&fixture, false);
    let mut value = manifest(&output);
    value["files"][0]["path"] = json!("../../outside");
    overwrite_manifest(&output, &value);
    assert!(runtime::verify(&output).is_err());
}

#[test]
fn assemble_rejects_wrong_architecture_platform_and_truncated_macho_commands() {
    let fixture = SourceFixture::new(false);
    let geultu = fixture.root.join(GEULTU_BIN);
    let mut wrong_arch = macho_arm64();
    wrong_arch[4..8].copy_from_slice(&0x0100_0007u32.to_le_bytes());
    fs::write(&geultu, wrong_arch).unwrap();
    fs::set_permissions(&geultu, fs::Permissions::from_mode(0o755)).unwrap();
    let wrong_output = child_path(fixture._temp.path(), "wrong-arch");
    assert!(runtime::assemble(&fixture.root, &wrong_output, false).is_err());
    assert!(!wrong_output.exists());

    let fixture = SourceFixture::new(false);
    let geultu = fixture.root.join(GEULTU_BIN);
    let mut wrong_platform = macho_arm64();
    wrong_platform[40..44].copy_from_slice(&2u32.to_le_bytes());
    fs::write(&geultu, wrong_platform).unwrap();
    fs::set_permissions(&geultu, fs::Permissions::from_mode(0o755)).unwrap();
    let wrong_platform_output = child_path(fixture._temp.path(), "wrong-platform");
    assert!(runtime::assemble(&fixture.root, &wrong_platform_output, false).is_err());
    assert!(!wrong_platform_output.exists());

    let fixture = SourceFixture::new(false);
    let geultu = fixture.root.join(GEULTU_BIN);
    let mut invalid_command_size = macho_arm64();
    invalid_command_size[36..40].copy_from_slice(&4u32.to_le_bytes());
    fs::write(&geultu, invalid_command_size).unwrap();
    fs::set_permissions(&geultu, fs::Permissions::from_mode(0o755)).unwrap();
    let invalid_command_size_output = child_path(fixture._temp.path(), "invalid-command-size");
    assert!(runtime::assemble(&fixture.root, &invalid_command_size_output, false).is_err());
    assert!(!invalid_command_size_output.exists());

    let fixture = SourceFixture::new(false);
    let geultu = fixture.root.join(GEULTU_BIN);
    let mut truncated = macho_arm64();
    truncated[20..24].copy_from_slice(&64u32.to_le_bytes());
    fs::write(&geultu, truncated).unwrap();
    fs::set_permissions(&geultu, fs::Permissions::from_mode(0o755)).unwrap();
    let truncated_output = child_path(fixture._temp.path(), "truncated-macho");
    assert!(runtime::assemble(&fixture.root, &truncated_output, false).is_err());
    assert!(!truncated_output.exists());

    let fixture = SourceFixture::new(false);
    let existing = fixture._temp.path().join("existing");
    fs::create_dir(&existing).unwrap();
    assert!(runtime::assemble(&fixture.root, &existing.canonicalize().unwrap(), false).is_err());
    assert!(fs::read_dir(&existing).unwrap().next().is_none());
}

#[test]
fn runtime_manifest_contains_no_absolute_or_checkout_path() {
    let fixture = SourceFixture::new(false);
    let (_temp, output) = bundle(&fixture, false);
    let bytes = fs::read(output.join("runtime-manifest.json")).unwrap();
    let text = String::from_utf8(bytes).unwrap();
    assert!(!text.contains(fixture.root.to_str().unwrap()));
    let value: Value = serde_json::from_str(&text).unwrap();
    let paths: Vec<_> = value["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["path"].as_str().unwrap())
        .collect();
    assert!(paths.windows(2).all(|window| window[0] < window[1]));
    assert_eq!(value["qualification"]["integrity"], "local_integrity_only");
    assert_eq!(
        value["qualification"]["commercialRedistribution"],
        "UNKNOWN"
    );
    assert_eq!(value["qualification"]["signing"], "UNKNOWN");
}

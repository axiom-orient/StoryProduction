//! Verification and local assembly of the macOS arm64 Story runtime root.
//!
//! The manifest is an integrity record for one local bundle. It is not a
//! signature, a publisher identity, or a commercial redistribution grant.

use crate::{
    contracts,
    effects::{files, hash, json, process, MAX_BYTES},
    error::{Error, Result},
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Read,
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::{Component, Path, PathBuf},
    time::Duration,
};

pub const STORYCTL_ENTRY: &str = "bin/storyctl";
pub const RUNTIME_MANIFEST: &str = "runtime-manifest.json";
pub const GEULTU_BIN: &str = "geultu/target/release/geultu";
pub const NVC_BIN: &str = "NarrativeCourier/target/release/narrative-courier-cli";
pub const SAC_BIN: &str = "SceneAssetCompiler/target/release/sac-animatic";
pub const SMC_BIN: &str = "SceneMotionCompiler/target/release/smc-animatic";
pub const PROMPTGEN_BIN: &str = "ImagePromptCompiler/target/release/promptgen";
pub const GEULTU_ASSETS_ROOT: &str = "geultu/assets";
pub const BASELINE_PACK_ROOT: &str = "geultu/assets/craft-packs-story-baseline-v1";
pub const SMC_ANIMATIC_RUNTIME_JS: &str =
    "SceneMotionCompiler/packages/production/src/animatic-runtime.js";
pub const SMC_TRACKB_NAVIGATION_GUARD_JS: &str =
    "SceneMotionCompiler/packages/contracts/src/trackb-navigation-guard.js";

const SOURCE_STORYCTL_BIN: &str = "integration/target/release/storyctl";
const CONTRACT_ROOT: &str = "integration/contracts";
const CONTRACT_REGISTRY: &str = "integration/contracts/registry.json";
const NVC_SKILLS_ROOT: &str = "NarrativeCourier/skills/sts";
const BASELINE_PACK_MANIFEST: &str = "geultu/assets/craft-packs-story-baseline-v1/craft-pack.json";
const PROMPTGEN_FIXTURE: &str = "ImagePromptCompiler/examples/image-cinematic-storyboard.json";
const PROMPTGEN_FIXTURE_REQUEST: &str =
    "integration/contracts/examples/capability-request.prompt-compile.json";
const MAX_RUNTIME_BINARY_BYTES: u64 = 128 * 1024 * 1024;
const MAX_RUNTIME_BINARY_TOTAL_BYTES: u64 = 256 * 1024 * 1024;
const MAX_RUNTIME_FILES: usize = 1024;
const MAX_MANIFEST_BYTES: usize = MAX_BYTES;
const VERSION_TIMEOUT: Duration = Duration::from_secs(2);
const VERSION_OUTPUT_LIMIT_BYTES: usize = 4096;
const TARGET: &str = "macos-arm64";
const MACHO_MAGIC_64_LE: [u8; 4] = [0xcf, 0xfa, 0xed, 0xfe];
const CPU_TYPE_ARM64: u32 = 0x0100_000c;
const MH_EXECUTE: u32 = 0x2;
const LC_BUILD_VERSION: u32 = 0x32;
const PLATFORM_MACOS: u32 = 1;
const LC_VERSION_MIN_MACOSX: u32 = 0x24;
const MAX_MACHO_LOAD_COMMANDS: u32 = 65_536;

const REQUIRED_BINARIES: [(&str, &str); 5] = [
    (STORYCTL_ENTRY, "storyctl"),
    (GEULTU_BIN, "geultu"),
    (NVC_BIN, "narrative-courier-cli"),
    (SAC_BIN, "sac-animatic"),
    (SMC_BIN, "smc-animatic"),
];

const REQUIRED_COMPONENT_NOTICES: [&str; 5] = [
    "geultu/LICENSE",
    "NarrativeCourier/LICENSE",
    "NarrativeCourier/NOTICE.md",
    "SceneMotionCompiler/LICENSE",
    "ImagePromptCompiler/LICENSE",
];

const REQUIRED_RENDERER_RESOURCES: [&str; 2] =
    [SMC_ANIMATIC_RUNTIME_JS, SMC_TRACKB_NAVIGATION_GUARD_JS];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RootMode {
    Source,
    Installed,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct RuntimeFile {
    path: String,
    size: u64,
    sha256: String,
    unix_mode: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct RuntimeQualification {
    integrity: String,
    commercial_redistribution: String,
    signing: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct RuntimeManifest {
    schema_version: String,
    supported_target: String,
    include_promptgen: bool,
    component_versions: BTreeMap<String, String>,
    qualification: RuntimeQualification,
    files: Vec<RuntimeFile>,
}

#[derive(Clone, Debug)]
struct FileSnapshot {
    bytes: Vec<u8>,
    manifest: RuntimeFile,
}

#[derive(Clone, Debug)]
struct FilePlan {
    source_path: String,
    manifest: RuntimeFile,
}

pub fn check_root(root: &Path) -> Result<RootMode> {
    let directory = files::Directory::open(root, false)?;
    if directory.exists(RUNTIME_MANIFEST)? {
        verify(root)?;
        directory.verify_identity()?;
        return Ok(RootMode::Installed);
    }

    // Explicit source roots remain valid when their tree happens to contain
    // a `bin/storyctl` path. The implicit executable resolver owns the
    // narrower fail-closed check for its exact current_exe path.
    if !directory.exists(CONTRACT_REGISTRY)? {
        return Err(Error::not_started(
            "selected Story root lacks integration/contracts/registry.json",
        ));
    }
    files::open_verified(&root.join(CONTRACT_REGISTRY))?;
    directory.verify_identity()?;
    Ok(RootMode::Source)
}

pub fn verify(root: &Path) -> Result<Value> {
    require_supported_host()?;
    let root = files::non_symlink_path(root)?;
    let root_directory = files::Directory::open(&root, false)?;
    let manifest_path = root.join(RUNTIME_MANIFEST);
    let manifest_snapshot = snapshot_file(&manifest_path, MAX_MANIFEST_BYTES as u64)?;
    if manifest_snapshot.manifest.unix_mode != 0o444 {
        return Err(Error::not_applied(
            "runtime manifest must be a regular read-only file with mode 0444",
        ));
    }
    let value = json::parse_object(&manifest_snapshot.bytes)?;
    let manifest: RuntimeManifest = serde_json::from_value(value)?;
    validate_manifest(&manifest)?;
    let (contract_count, consumer_pin_count) = verify_contract_registry(&root)?;
    verify_files(&root, &manifest.files, manifest.include_promptgen, true)?;
    root_directory.verify_identity()?;

    Ok(json!({
        "status": "PASS",
        "schemaVersion": manifest.schema_version,
        "supportedTarget": manifest.supported_target,
        "fileCount": manifest.files.len(),
        "contractCount": contract_count,
        "consumerPinCount": consumer_pin_count,
        "includePromptgen": manifest.include_promptgen,
        "componentVersions": manifest.component_versions,
        "qualification": {
            "integrity": manifest.qualification.integrity,
            "commercialRedistribution": manifest.qualification.commercial_redistribution,
            "signing": manifest.qualification.signing
        }
    }))
}

pub fn assemble(source_root: &Path, output: &Path, include_promptgen: bool) -> Result<Value> {
    require_supported_host()?;
    let source_root = files::non_symlink_path(source_root)?;
    let source_root = source_root.canonicalize()?;
    if check_root(&source_root)? != RootMode::Source {
        return Err(Error::not_applied(
            "runtime assembly requires a Story source root",
        ));
    }
    let source_directory = files::Directory::open(&source_root, false)?;
    verify_contract_registry(&source_root)?;

    let output = prospective_output_path(output)?;
    if output.starts_with(&source_root) || source_root.starts_with(&output) {
        return Err(Error::not_applied(
            "runtime output must be separate from the Story source root",
        ));
    }

    let mut files_by_path: BTreeMap<String, FilePlan> = BTreeMap::new();
    add_file(
        &source_root,
        SOURCE_STORYCTL_BIN,
        STORYCTL_ENTRY,
        &mut files_by_path,
    )?;
    for (path, _) in REQUIRED_BINARIES.iter().skip(1) {
        add_file(&source_root, path, path, &mut files_by_path)?;
    }

    add_tree(
        &source_root,
        CONTRACT_ROOT,
        CONTRACT_ROOT,
        &mut files_by_path,
    )?;
    for path in registered_contract_paths(&source_root)? {
        add_file(&source_root, &path, &path, &mut files_by_path)?;
    }
    add_tree(
        &source_root,
        NVC_SKILLS_ROOT,
        NVC_SKILLS_ROOT,
        &mut files_by_path,
    )?;
    add_tree(
        &source_root,
        BASELINE_PACK_ROOT,
        BASELINE_PACK_ROOT,
        &mut files_by_path,
    )?;
    for path in REQUIRED_COMPONENT_NOTICES {
        add_file(&source_root, path, path, &mut files_by_path)?;
    }
    // SMC's native renderer reads these exact files from its selected owner
    // root and compares them with the copies embedded in its executable.
    for path in REQUIRED_RENDERER_RESOURCES {
        add_file(&source_root, path, path, &mut files_by_path)?;
    }
    // PromptGen's producer schemas are in the pinned contracts above, so its
    // actual available notices ship even when its executable is omitted.
    add_file(
        &source_root,
        "ImagePromptCompiler/NOTICE.md",
        "ImagePromptCompiler/NOTICE.md",
        &mut files_by_path,
    )?;

    if include_promptgen {
        add_file(
            &source_root,
            PROMPTGEN_BIN,
            PROMPTGEN_BIN,
            &mut files_by_path,
        )?;
        add_file(
            &source_root,
            PROMPTGEN_FIXTURE,
            PROMPTGEN_FIXTURE,
            &mut files_by_path,
        )?;
    }
    admit_file_plans(&files_by_path)?;

    let planned_files: Vec<RuntimeFile> = files_by_path
        .values()
        .map(|plan| plan.manifest.clone())
        .collect();
    validate_file_entries(&planned_files, include_promptgen)?;

    // Reserve only a new destination. Existing output is never overwritten or
    // treated as a replay, including an empty directory.
    if !files::reserve_directory(&output)? {
        return Err(Error::not_applied(
            "runtime output already exists; choose a new empty destination",
        ));
    }
    let output_directory = files::Directory::open(&output, false)?;
    for plan in files_by_path.values() {
        let source = source_root.join(&plan.source_path);
        let maximum = maximum_file_size(&plan.manifest.path);
        let snapshot = snapshot_file(&source, maximum)?;
        if snapshot.manifest.size != plan.manifest.size
            || snapshot.manifest.sha256 != plan.manifest.sha256
            || (snapshot.manifest.unix_mode & !0o222) != plan.manifest.unix_mode
        {
            return Err(Error::not_applied(format!(
                "runtime source changed after its initial inspection: {}",
                plan.source_path
            )));
        }
        if is_binary_path(&plan.manifest.path) {
            validate_macho_macos_arm64(&snapshot.bytes)?;
        }
        output_directory.immutable(&plan.manifest.path, &snapshot.bytes)?;
        set_file_mode(&output, &plan.manifest.path, plan.manifest.unix_mode)?;
    }
    output_directory.verify_identity()?;
    source_directory.verify_identity()?;
    verify_contract_registry(&output)?;
    set_tree_readonly(&output, Path::new(""))?;
    verify_files(&output, &planned_files, include_promptgen, false)?;

    // Query only binaries whose CLI exposes a documented, no-effect
    // `--version` path. The copied payload is checked before it is launched.
    let manifest = RuntimeManifest {
        schema_version: "story.runtime-manifest/1".into(),
        supported_target: TARGET.into(),
        include_promptgen,
        component_versions: reported_component_versions(&output, include_promptgen)?,
        qualification: RuntimeQualification {
            integrity: "local_integrity_only".into(),
            commercial_redistribution: "UNKNOWN".into(),
            signing: "UNKNOWN".into(),
        },
        files: planned_files,
    };
    validate_manifest(&manifest)?;
    verify_files(&output, &manifest.files, manifest.include_promptgen, false)?;

    // The still-private root remains writable only long enough to publish the
    // final manifest. Seal the root immediately afterward.
    let manifest_bytes = json::stable_bytes(&serde_json::to_value(&manifest)?)?;
    if manifest_bytes.len() > MAX_MANIFEST_BYTES {
        return Err(Error::not_applied(
            "runtime manifest exceeds the bounded JSON size limit",
        ));
    }
    output_directory.immutable(RUNTIME_MANIFEST, &manifest_bytes)?;
    set_file_mode(&output, RUNTIME_MANIFEST, 0o444)?;
    set_directory_mode(&output, 0o555)?;
    output_directory.verify_identity()?;
    verify(&output)
}

fn require_supported_host() -> Result<()> {
    if !cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        return Err(Error::not_started(
            "Story runtime bundles currently support macOS arm64 only",
        ));
    }
    Ok(())
}

fn prospective_output_path(output: &Path) -> Result<PathBuf> {
    let absolute = std::path::absolute(output)?;
    let parent = absolute
        .parent()
        .ok_or_else(|| Error::not_applied("runtime output has no parent directory"))?;
    let leaf = absolute
        .file_name()
        .ok_or_else(|| Error::not_applied("runtime output has no final path component"))?;
    let canonical_parent = files::non_symlink_path(parent)?.canonicalize()?;
    let result = canonical_parent.join(leaf);
    safe_relative(
        leaf.to_str()
            .ok_or_else(|| Error::not_applied("runtime output path must use valid UTF-8"))?,
    )?;
    Ok(result)
}

fn verify_contract_registry(root: &Path) -> Result<(usize, usize)> {
    let report = contracts::verify_contract_registry(root, &root.join(CONTRACT_REGISTRY))?;
    let entries = report["contracts"]
        .as_array()
        .ok_or_else(|| Error::not_applied("contract registry report is invalid"))?;
    let consumer_count = entries
        .iter()
        .map(|entry| entry["consumers"].as_array().map_or(0, Vec::len))
        .sum();
    Ok((entries.len(), consumer_count))
}

fn registered_contract_paths(root: &Path) -> Result<BTreeSet<String>> {
    let bytes = files::read_bytes_limited(&root.join(CONTRACT_REGISTRY), MAX_MANIFEST_BYTES)?;
    let registry = json::parse_object(&bytes)?;
    let contracts = registry["contracts"]
        .as_array()
        .ok_or_else(|| Error::not_applied("contract registry requires a contracts array"))?;
    let mut paths = BTreeSet::new();
    for entry in contracts {
        let producer = entry["producer"]
            .as_str()
            .ok_or_else(|| Error::not_applied("contract producer path is invalid"))?;
        safe_relative(producer)?;
        paths.insert(producer.to_owned());
        if let Some(consumers) = entry.get("consumers") {
            for consumer in consumers
                .as_array()
                .ok_or_else(|| Error::not_applied("contract consumers must be an array"))?
            {
                let path = consumer
                    .as_str()
                    .ok_or_else(|| Error::not_applied("contract consumer path is invalid"))?;
                safe_relative(path)?;
                paths.insert(path.to_owned());
            }
        }
    }
    Ok(paths)
}

fn add_file(
    source_root: &Path,
    source_relative: &str,
    output_relative: &str,
    files_by_path: &mut BTreeMap<String, FilePlan>,
) -> Result<()> {
    safe_relative(source_relative)?;
    safe_relative(output_relative)?;
    let snapshot = snapshot_file(
        &source_root.join(source_relative),
        maximum_file_size(output_relative),
    )?;
    validate_source_mode(snapshot.manifest.unix_mode, Path::new(source_relative))?;
    if is_binary_path(output_relative) {
        validate_macho_macos_arm64(&snapshot.bytes)?;
        if snapshot.manifest.unix_mode & 0o100 == 0 {
            return Err(Error::not_started(format!(
                "runtime executable lacks owner execute permission: {source_relative}"
            )));
        }
    }
    let mut manifest = snapshot.manifest;
    manifest.path = output_relative.to_owned();
    manifest.unix_mode &= !0o222;
    if let Some(existing) = files_by_path.get(output_relative) {
        if existing.manifest.size != manifest.size
            || existing.manifest.sha256 != manifest.sha256
            || existing.manifest.unix_mode != manifest.unix_mode
        {
            return Err(Error::not_applied(format!(
                "runtime input path is selected with conflicting bytes or modes: {output_relative}"
            )));
        }
        return Ok(());
    }
    files_by_path.insert(
        output_relative.to_owned(),
        FilePlan {
            source_path: source_relative.to_owned(),
            manifest,
        },
    );
    admit_file_plans(files_by_path)?;
    Ok(())
}

fn add_tree(
    source_root: &Path,
    source_relative: &str,
    output_relative: &str,
    files_by_path: &mut BTreeMap<String, FilePlan>,
) -> Result<()> {
    safe_relative(source_relative)?;
    safe_relative(output_relative)?;
    let source = source_root.join(source_relative);
    let directory = files::Directory::open(&source, false)?;
    let mut children = Vec::new();
    for entry in fs::read_dir(&source)? {
        let entry = entry?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| Error::not_applied("runtime source path is not valid UTF-8"))?;
        safe_leaf(&name)?;
        children.push(name);
    }
    children.sort();
    for name in children {
        let child_source_relative = format!("{source_relative}/{name}");
        let child_output_relative = format!("{output_relative}/{name}");
        let child_source = source_root.join(&child_source_relative);
        let metadata = fs::symlink_metadata(&child_source)?;
        if metadata.file_type().is_symlink() {
            return Err(Error::not_applied(format!(
                "runtime source tree contains a symlink: {child_source_relative}"
            )));
        }
        if metadata.is_dir() {
            add_tree(
                source_root,
                &child_source_relative,
                &child_output_relative,
                files_by_path,
            )?;
        } else if metadata.is_file() {
            add_file(
                source_root,
                &child_source_relative,
                &child_output_relative,
                files_by_path,
            )?;
        } else {
            return Err(Error::not_applied(format!(
                "runtime source tree contains a non-regular entry: {child_source_relative}"
            )));
        }
    }
    directory.verify_identity()?;
    Ok(())
}

fn maximum_file_size(path: &str) -> u64 {
    if is_binary_path(path) {
        MAX_RUNTIME_BINARY_BYTES
    } else {
        MAX_BYTES as u64
    }
}

fn admit_file_plans(files_by_path: &BTreeMap<String, FilePlan>) -> Result<()> {
    admit_file_records(files_by_path.values().map(|plan| &plan.manifest))
}

fn admit_file_records<'a>(files: impl Iterator<Item = &'a RuntimeFile>) -> Result<()> {
    let mut count = 0usize;
    let mut binary_total = 0u64;
    let mut resource_total = 0u64;
    for file in files {
        count = count
            .checked_add(1)
            .ok_or_else(|| Error::not_applied("runtime file count overflow"))?;
        // The final manifest is also one regular file in the installed root.
        let final_count = count
            .checked_add(1)
            .ok_or_else(|| Error::not_applied("runtime file count overflow"))?;
        if final_count > MAX_RUNTIME_FILES {
            return Err(Error::not_applied(
                "runtime file count exceeds its bounded manifest policy",
            ));
        }
        if is_binary_path(&file.path) {
            if file.size > MAX_RUNTIME_BINARY_BYTES {
                return Err(Error::not_applied(format!(
                    "runtime binary exceeds its per-file limit: {}",
                    file.path
                )));
            }
            binary_total = binary_total
                .checked_add(file.size)
                .ok_or_else(|| Error::not_applied("runtime binary size sum overflow"))?;
            if binary_total > MAX_RUNTIME_BINARY_TOTAL_BYTES {
                return Err(Error::not_applied(
                    "runtime binaries exceed the aggregate size limit",
                ));
            }
        } else {
            if file.size > MAX_BYTES as u64 {
                return Err(Error::not_applied(format!(
                    "runtime resource exceeds the transport-sized file limit: {}",
                    file.path
                )));
            }
            resource_total = resource_total
                .checked_add(file.size)
                .ok_or_else(|| Error::not_applied("runtime resource size sum overflow"))?;
            if resource_total > MAX_BYTES as u64 {
                return Err(Error::not_applied(
                    "runtime resources exceed the bounded aggregate snapshot budget",
                ));
            }
        }
    }
    Ok(())
}

fn snapshot_file(path: &Path, maximum: u64) -> Result<FileSnapshot> {
    let mut file = files::open_verified(path)?;
    let before = file.metadata()?;
    if !before.is_file() || before.len() > maximum {
        return Err(Error::not_applied(format!(
            "runtime input is not a regular file or exceeds the size limit: {}",
            path.display()
        )));
    }
    let source_mode = before.mode() & 0o7777;
    let mut bytes = Vec::with_capacity(before.len() as usize);
    (&mut file)
        .take(maximum.saturating_add(1))
        .read_to_end(&mut bytes)?;
    let after = file.metadata()?;
    if bytes.len() as u64 != before.len()
        || after.len() != before.len()
        || after.dev() != before.dev()
        || after.ino() != before.ino()
        || after.mtime() != before.mtime()
        || after.mtime_nsec() != before.mtime_nsec()
        || after.ctime() != before.ctime()
        || after.ctime_nsec() != before.ctime_nsec()
    {
        return Err(Error::not_applied("runtime input changed while reading"));
    }
    Ok(FileSnapshot {
        manifest: RuntimeFile {
            path: String::new(),
            size: bytes.len() as u64,
            sha256: hash::sha256_bytes(&bytes),
            unix_mode: source_mode,
        },
        bytes,
    })
}

fn validate_manifest(manifest: &RuntimeManifest) -> Result<()> {
    if manifest.schema_version != "story.runtime-manifest/1" {
        return Err(Error::not_applied("unsupported runtime manifest schema"));
    }
    if manifest.supported_target != TARGET {
        return Err(Error::not_started(
            "runtime manifest target is not macOS arm64",
        ));
    }
    if manifest.qualification.integrity != "local_integrity_only"
        || manifest.qualification.commercial_redistribution != "UNKNOWN"
        || manifest.qualification.signing != "UNKNOWN"
    {
        return Err(Error::not_applied(
            "runtime qualification must preserve local-integrity-only, commercial UNKNOWN, and signing UNKNOWN",
        ));
    }
    validate_file_entries(&manifest.files, manifest.include_promptgen)?;
    for (name, version) in &manifest.component_versions {
        if !matches!(name.as_str(), "storyctl" | "geultu" | "promptgen")
            || version.is_empty()
            || version.len() > 128
            || !version
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'+'))
        {
            return Err(Error::not_applied(
                "runtime manifest contains an invalid component version",
            ));
        }
    }
    if !manifest.component_versions.contains_key("storyctl")
        || !manifest.component_versions.contains_key("geultu")
        || (manifest.include_promptgen && !manifest.component_versions.contains_key("promptgen"))
    {
        return Err(Error::not_applied(
            "runtime manifest is missing a reliably observable component version",
        ));
    }
    if !manifest.include_promptgen && manifest.component_versions.contains_key("promptgen") {
        return Err(Error::not_applied(
            "runtime manifest reports PromptGen when its optional binary is omitted",
        ));
    }
    Ok(())
}

fn validate_file_entries(files: &[RuntimeFile], include_promptgen: bool) -> Result<()> {
    if files.is_empty() {
        return Err(Error::not_applied("runtime manifest has no files"));
    }
    let mut previous: Option<&str> = None;
    for entry in files {
        safe_relative(&entry.path)?;
        if entry.path == RUNTIME_MANIFEST {
            return Err(Error::not_applied(
                "runtime manifest cannot hash itself as a bundled file",
            ));
        }
        if previous.is_some_and(|path| path >= entry.path.as_str()) {
            return Err(Error::not_applied(
                "runtime manifest file paths must be sorted and unique",
            ));
        }
        previous = Some(&entry.path);
        if !hash::is_sha256(&entry.sha256) {
            return Err(Error::not_applied(format!(
                "runtime manifest has an invalid SHA-256 for {}",
                entry.path
            )));
        }
        validate_bundled_mode(entry.unix_mode, Path::new(&entry.path))?;
    }
    admit_file_records(files.iter())?;

    let expected_paths: BTreeSet<&str> = files.iter().map(|entry| entry.path.as_str()).collect();
    for required in required_paths(include_promptgen) {
        if !expected_paths.contains(required) {
            return Err(Error::not_started(format!(
                "runtime manifest is missing required file {required}"
            )));
        }
    }
    if !include_promptgen
        && (expected_paths.contains(PROMPTGEN_BIN) || expected_paths.contains(PROMPTGEN_FIXTURE))
    {
        return Err(Error::not_applied(
            "PromptGen binary or fixture is present while the optional lane is omitted",
        ));
    }
    Ok(())
}

fn validate_source_mode(mode: u32, path: &Path) -> Result<()> {
    if mode & !0o777 != 0 || mode & 0o022 != 0 || mode & 0o400 == 0 {
        return Err(Error::not_applied(format!(
            "runtime file mode is unsafe: {}",
            path.display()
        )));
    }
    if is_binary_path(&path.to_string_lossy()) && mode & 0o100 == 0 {
        return Err(Error::not_started(format!(
            "runtime executable is not owner-executable: {}",
            path.display()
        )));
    }
    Ok(())
}

fn validate_bundled_mode(mode: u32, path: &Path) -> Result<()> {
    if mode & !0o777 != 0 || mode & 0o222 != 0 || mode & 0o400 == 0 {
        return Err(Error::not_applied(format!(
            "runtime bundle contains a writable or unreadable file: {}",
            path.display()
        )));
    }
    if is_binary_path(&path.to_string_lossy()) && mode & 0o100 == 0 {
        return Err(Error::not_started(format!(
            "runtime executable is not owner-executable: {}",
            path.display()
        )));
    }
    Ok(())
}

fn verify_files(
    root: &Path,
    files: &[RuntimeFile],
    include_promptgen: bool,
    has_manifest: bool,
) -> Result<()> {
    validate_file_entries(files, include_promptgen)?;

    for entry in files {
        let snapshot = snapshot_file(&root.join(&entry.path), maximum_file_size(&entry.path))?;
        validate_bundled_mode(snapshot.manifest.unix_mode, Path::new(&entry.path))?;
        if snapshot.manifest.size != entry.size
            || snapshot.manifest.sha256 != entry.sha256
            || snapshot.manifest.unix_mode != entry.unix_mode
        {
            return Err(Error::not_applied(format!(
                "runtime file integrity mismatch: {}",
                entry.path
            )));
        }
        if is_binary_path(&entry.path) {
            validate_macho_macos_arm64(&snapshot.bytes)?;
        }
    }

    let (actual_files, actual_directories) = inventory(root, has_manifest)?;
    let mut expected_files: BTreeSet<String> =
        files.iter().map(|entry| entry.path.clone()).collect();
    if has_manifest {
        expected_files.insert(RUNTIME_MANIFEST.to_owned());
    }
    if actual_files != expected_files {
        let unexpected = actual_files
            .difference(&expected_files)
            .next()
            .or_else(|| expected_files.difference(&actual_files).next())
            .cloned()
            .unwrap_or_default();
        return Err(Error::not_applied(format!(
            "runtime file inventory differs from its manifest: {unexpected}"
        )));
    }
    let expected_directories = parent_directories(&expected_files);
    if actual_directories != expected_directories {
        return Err(Error::not_applied(
            "runtime directory inventory differs from its file paths",
        ));
    }
    Ok(())
}

fn required_paths(include_promptgen: bool) -> Vec<&'static str> {
    let mut paths = vec![
        STORYCTL_ENTRY,
        GEULTU_BIN,
        NVC_BIN,
        SAC_BIN,
        SMC_BIN,
        CONTRACT_REGISTRY,
        "NarrativeCourier/skills/sts/SKILL.md",
        BASELINE_PACK_MANIFEST,
        "geultu/assets/craft-packs-story-baseline-v1/LICENSE",
        "geultu/LICENSE",
        "NarrativeCourier/LICENSE",
        "NarrativeCourier/NOTICE.md",
        "SceneMotionCompiler/LICENSE",
        SMC_ANIMATIC_RUNTIME_JS,
        SMC_TRACKB_NAVIGATION_GUARD_JS,
        "ImagePromptCompiler/LICENSE",
        "ImagePromptCompiler/NOTICE.md",
    ];
    if include_promptgen {
        paths.extend([PROMPTGEN_BIN, PROMPTGEN_FIXTURE, PROMPTGEN_FIXTURE_REQUEST]);
    }
    paths
}

fn inventory(
    root: &Path,
    require_readonly_root: bool,
) -> Result<(BTreeSet<String>, BTreeSet<String>)> {
    let mut files = BTreeSet::new();
    let mut directories = BTreeSet::new();
    inventory_directory(
        root,
        Path::new(""),
        require_readonly_root,
        &mut files,
        &mut directories,
    )?;
    Ok((files, directories))
}

fn inventory_directory(
    root: &Path,
    relative: &Path,
    require_readonly_root: bool,
    files_seen: &mut BTreeSet<String>,
    directories_seen: &mut BTreeSet<String>,
) -> Result<()> {
    let directory_path = root.join(relative);
    let metadata = fs::symlink_metadata(&directory_path)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(Error::not_applied(
            "runtime directory is a symlink or non-directory",
        ));
    }
    validate_directory_mode(
        metadata.mode(),
        relative,
        relative.as_os_str().is_empty() && require_readonly_root,
    )?;
    let directory = files::Directory::open(&directory_path, false)?;
    for entry in fs::read_dir(&directory_path)? {
        let entry = entry?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| Error::not_applied("runtime contains a non-UTF-8 path"))?;
        safe_leaf(&name)?;
        let child_relative = relative.join(&name);
        let child_string = path_string(&child_relative)?;
        let child_path = root.join(&child_relative);
        let metadata = fs::symlink_metadata(&child_path)?;
        if metadata.file_type().is_symlink() {
            return Err(Error::not_applied(format!(
                "runtime contains a symlink: {child_string}"
            )));
        }
        if metadata.is_dir() {
            let mode = metadata.mode();
            validate_directory_mode(mode, &child_relative, true)?;
            files::Directory::open(&child_path, false)?;
            directories_seen.insert(child_string);
            inventory_directory(
                root,
                &child_relative,
                require_readonly_root,
                files_seen,
                directories_seen,
            )?;
        } else if metadata.is_file() {
            files::open_verified(&child_path)?;
            files_seen.insert(child_string);
        } else {
            return Err(Error::not_applied(format!(
                "runtime contains a non-regular path: {child_string}"
            )));
        }
    }
    directory.verify_identity()?;
    Ok(())
}

fn parent_directories(files: &BTreeSet<String>) -> BTreeSet<String> {
    let mut result = BTreeSet::new();
    for file in files {
        let mut parent = Path::new(file).parent();
        while let Some(path) = parent {
            if path.as_os_str().is_empty() {
                break;
            }
            result.insert(path.to_string_lossy().replace('\\', "/"));
            parent = path.parent();
        }
    }
    result
}

fn set_file_mode(root: &Path, relative: &str, mode: u32) -> Result<()> {
    safe_relative(relative)?;
    let file = files::open_verified(&root.join(relative))?;
    file.set_permissions(fs::Permissions::from_mode(mode))?;
    file.sync_all()?;
    let parent = Path::new(relative).parent().unwrap_or(Path::new(""));
    files::open_directory(&root.join(parent))?.sync_all()?;
    Ok(())
}

fn set_directory_mode(path: &Path, mode: u32) -> Result<()> {
    let directory = files::open_directory(path)?;
    directory.set_permissions(fs::Permissions::from_mode(mode))?;
    directory.sync_all()?;
    if let Some(parent) = path.parent() {
        files::open_directory(parent)?.sync_all()?;
    }
    Ok(())
}

fn set_tree_readonly(root: &Path, relative: &Path) -> Result<()> {
    let path = root.join(relative);
    let directory = files::Directory::open(&path, false)?;
    for entry in fs::read_dir(&path)? {
        let entry = entry?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| Error::not_applied("runtime directory has a non-UTF-8 path"))?;
        safe_leaf(&name)?;
        let child_relative = relative.join(name);
        let child_path = root.join(&child_relative);
        let metadata = fs::symlink_metadata(&child_path)?;
        if metadata.file_type().is_symlink() {
            return Err(Error::not_applied("runtime tree contains a symlink"));
        }
        if metadata.is_dir() {
            set_tree_readonly(root, &child_relative)?;
        } else if !metadata.is_file() {
            return Err(Error::not_applied(
                "runtime tree contains a non-regular entry",
            ));
        }
    }
    directory.verify_identity()?;
    if !relative.as_os_str().is_empty() {
        set_directory_mode(&path, 0o555)?;
    }
    Ok(())
}

fn validate_directory_mode(mode: u32, path: &Path, require_readonly: bool) -> Result<()> {
    let permissions = mode & 0o777;
    if mode & 0o7000 != 0
        || permissions & 0o500 != 0o500
        || permissions & 0o022 != 0
        || (require_readonly && permissions & 0o222 != 0)
    {
        return Err(Error::not_applied(format!(
            "runtime directory mode is unsafe: {}",
            path.display()
        )));
    }
    Ok(())
}

fn validate_macho_macos_arm64(bytes: &[u8]) -> Result<()> {
    if bytes.len() < 32 || bytes[..4] != MACHO_MAGIC_64_LE {
        return Err(Error::not_started(
            "runtime executable is not a thin little-endian 64-bit Mach-O",
        ));
    }
    let cpu = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
    let file_type = u32::from_le_bytes(bytes[12..16].try_into().unwrap());
    if cpu != CPU_TYPE_ARM64 || file_type != MH_EXECUTE {
        return Err(Error::not_started(
            "runtime executable is not an arm64 MH_EXECUTE binary",
        ));
    }

    let ncmds = read_macho_u32(bytes, 16)?;
    let sizeofcmds = read_macho_u32(bytes, 20)? as usize;
    if ncmds == 0
        || ncmds > MAX_MACHO_LOAD_COMMANDS
        || sizeofcmds > bytes.len().saturating_sub(32)
        || ncmds as usize > sizeofcmds / 8
    {
        return Err(Error::not_started(
            "runtime Mach-O load-command table is outside its bounds",
        ));
    }
    let command_end = 32usize
        .checked_add(sizeofcmds)
        .ok_or_else(|| Error::not_started("runtime Mach-O load-command bounds overflow"))?;
    let mut cursor = 32usize;
    let mut macos_platform = false;
    let mut non_macos_platform = false;
    let mut legacy_macos_platform = false;
    for _ in 0..ncmds {
        if cursor.checked_add(8).is_none_or(|end| end > command_end) {
            return Err(Error::not_started(
                "runtime Mach-O load command is truncated",
            ));
        }
        let command = read_macho_u32(bytes, cursor)?;
        let command_size = read_macho_u32(bytes, cursor + 4)? as usize;
        if command_size < 8
            || !command_size.is_multiple_of(4)
            || cursor
                .checked_add(command_size)
                .is_none_or(|end| end > command_end)
        {
            return Err(Error::not_started(
                "runtime Mach-O load command has invalid size or bounds",
            ));
        }
        match command {
            LC_BUILD_VERSION => {
                if command_size < 24 {
                    return Err(Error::not_started(
                        "runtime Mach-O build-version command is truncated",
                    ));
                }
                let platform = read_macho_u32(bytes, cursor + 8)?;
                if platform == PLATFORM_MACOS {
                    macos_platform = true;
                } else {
                    non_macos_platform = true;
                }
            }
            LC_VERSION_MIN_MACOSX => {
                if command_size < 16 {
                    return Err(Error::not_started(
                        "runtime Mach-O macOS minimum-version command is truncated",
                    ));
                }
                legacy_macos_platform = true;
            }
            _ => {}
        }
        cursor += command_size;
    }
    if cursor != command_end {
        return Err(Error::not_started(
            "runtime Mach-O load-command byte count does not match the header",
        ));
    }
    if non_macos_platform || !(macos_platform || legacy_macos_platform) {
        return Err(Error::not_started(
            "runtime executable does not declare a macOS platform",
        ));
    }
    Ok(())
}

fn read_macho_u32(bytes: &[u8], offset: usize) -> Result<u32> {
    let end = offset
        .checked_add(4)
        .ok_or_else(|| Error::not_started("runtime Mach-O field offset overflow"))?;
    let field = bytes
        .get(offset..end)
        .ok_or_else(|| Error::not_started("runtime Mach-O field is truncated"))?;
    Ok(u32::from_le_bytes(field.try_into().unwrap()))
}

fn is_binary_path(path: &str) -> bool {
    REQUIRED_BINARIES.iter().any(|(binary, _)| *binary == path) || path == PROMPTGEN_BIN
}

fn reported_component_versions(
    root: &Path,
    include_promptgen: bool,
) -> Result<BTreeMap<String, String>> {
    let mut versions = BTreeMap::new();
    for (path, name) in [(STORYCTL_ENTRY, "storyctl"), (GEULTU_BIN, "geultu")] {
        versions.insert(name.to_owned(), reported_version(root, path, name)?);
    }
    if include_promptgen {
        versions.insert(
            "promptgen".to_owned(),
            reported_version(root, PROMPTGEN_BIN, "promptgen")?,
        );
    }
    Ok(versions)
}

fn reported_version(root: &Path, relative_binary: &str, expected_name: &str) -> Result<String> {
    let binary = root.join(relative_binary);
    let output = process::invoke(&process::ProcessSpec {
        argv: vec![binary.to_string_lossy().into_owned(), "--version".into()],
        input: Vec::new(),
        cwd: root.to_owned(),
        timeout: VERSION_TIMEOUT,
        env: BTreeMap::new(),
        pass_fds: Vec::new(),
        max_output_bytes: VERSION_OUTPUT_LIMIT_BYTES,
    })
    .map_err(|mut error| {
        error.message = format!(
            "could not observe {expected_name} version through its read-only --version command: {}",
            error.message
        );
        error
    })?;
    if !output.status.success() {
        return Err(Error::not_started(format!(
            "{expected_name} --version exited with {}",
            output.status
        )));
    }
    let text = std::str::from_utf8(&output.stdout)
        .map_err(|_| Error::not_applied(format!("{expected_name} version output is not UTF-8")))?
        .trim();
    if text.lines().count() != 1 {
        return Err(Error::not_applied(format!(
            "{expected_name} --version output is not one line"
        )));
    }
    let (name, version) = text.split_once(' ').ok_or_else(|| {
        Error::not_applied(format!("{expected_name} --version output has no version"))
    })?;
    if name != expected_name
        || version.is_empty()
        || !version
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'+'))
    {
        return Err(Error::not_applied(format!(
            "{expected_name} --version output is not a recognized version"
        )));
    }
    Ok(version.to_owned())
}

fn safe_relative(value: &str) -> Result<()> {
    if value.is_empty()
        || value.starts_with('/')
        || value.contains(['\\', ':', '\0'])
        || value
            .split('/')
            .any(|part| part.is_empty() || matches!(part, "." | ".."))
        || !Path::new(value)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
    {
        return Err(Error::not_applied(
            "runtime path is not a safe relative path",
        ));
    }
    Ok(())
}

fn safe_leaf(value: &str) -> Result<()> {
    if value.is_empty() || value == "." || value == ".." || value.contains(['/', '\\', ':', '\0']) {
        return Err(Error::not_applied(
            "runtime path contains an unsafe component",
        ));
    }
    Ok(())
}

fn path_string(path: &Path) -> Result<String> {
    let value = path
        .to_str()
        .ok_or_else(|| Error::not_applied("runtime path is not valid UTF-8"))?
        .replace('\\', "/");
    safe_relative(&value)?;
    Ok(value)
}

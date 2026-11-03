use super::files::NEXT_NONCE;
use super::reference::{self, ensure_reference_identity, open_reference, reference_file_identity};
use super::transport::validate_api_key;
use super::*;
use promptgen_core::image::{ImagePromptRequest, ImageReferenceRole, ImageTaskMode};
use std::fs::File;
use std::path::Path;
use std::time::Duration;

use base64::Engine as _;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::Ordering;

use promptgen_core::image::compile_image_prompt;
use promptgen_core::json::parse;

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn test_root(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "promptgen-openai-test-{label}-{}-{}",
        std::process::id(),
        NEXT_NONCE.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&path).unwrap();
    path
}

fn edit_request() -> ImagePromptRequest {
    ImagePromptRequest::from_json(
        parse(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/image-edit-product.json"
        )))
        .unwrap(),
    )
    .unwrap()
}

fn fixture() -> PathBuf {
    repository_root().join("tests/fixtures/valid-1024x1536.png")
}

fn run_with_test_key(test_name: &str) -> bool {
    if std::env::var_os("OPENAI_API_KEY").is_some() {
        return true;
    }
    let status = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", test_name, "--nocapture"])
        .env("OPENAI_API_KEY", "sk-test-only")
        .status()
        .unwrap();
    assert!(status.success(), "child test failed: {test_name}");
    false
}

fn run_without_test_key(test_name: &str) -> bool {
    if std::env::var_os("OPENAI_NO_KEY_CHILD").is_some() {
        return true;
    }
    let status = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", test_name, "--nocapture"])
        .env_remove("OPENAI_API_KEY")
        .env("OPENAI_NO_KEY_CHILD", "1")
        .status()
        .unwrap();
    assert!(status.success(), "child test failed: {test_name}");
    false
}

#[test]
fn api_key_rules_reject_blank_and_control_values() {
    assert_eq!(
        validate_api_key("   ").unwrap_err().code,
        "OPENAI_API_KEY_INVALID"
    );
    assert_eq!(
        validate_api_key("sk-test\nvalue").unwrap_err().code,
        "OPENAI_API_KEY_INVALID"
    );
    assert_eq!(validate_api_key("sk-test").unwrap(), "sk-test");
}

#[cfg(unix)]
fn fake_curl(root: &Path, status: u16, response: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;

    let script = root.join("fake-curl.sh");
    let body = format!(
        "#!/bin/sh\nset -eu\nprintf '%s' \"$*\" > \"$0.argv\"\ncat > \"$0.stdin\"\nresponse=$(awk -F '\"' '$1 == \"output = \" {{print $2}}' \"$0.stdin\")\nheaders=$(awk -F '\"' '$1 == \"dump-header = \" {{print $2}}' \"$0.stdin\")\nprompt_path=$(sed -n 's/^form = \"prompt=<\\(.*\\)\"$/\\1/p' \"$0.stdin\")\nimage_path=$(sed -n 's/^form = \"image\\[\\]=@\\(.*\\);type=.*\"$/\\1/p' \"$0.stdin\")\ncat \"$prompt_path\" > \"$0.prompt\"\ncp \"$image_path\" \"$0.image\"\nprintf '%s' '{}' > \"$response\"\nprintf '%s\\n' 'HTTP/1.1 {} OK' 'x-request-id: req-fixture' > \"$headers\"\nprintf '%s' '{}'\n",
        response, status, status
    );
    fs::write(&script, body).unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    script
}

#[cfg(unix)]
#[test]
fn fixture_proves_single_base_mapping_and_secret_boundary() {
    if !run_with_test_key("tests::fixture_proves_single_base_mapping_and_secret_boundary") {
        return;
    }
    let root = test_root("success");
    let encoded = base64::engine::general_purpose::STANDARD.encode(fs::read(fixture()).unwrap());
    let response = format!("{{\"data\":[{{\"b64_json\":\"{encoded}\"}}]}}");
    let script = fake_curl(&root, 200, &response);
    let reference = root.join("user-source.png");
    fs::copy(fixture(), &reference).unwrap();
    let output = root.join("output/result.png");
    let request = edit_request();
    let compilation = compile_image_prompt(&request);
    assert!(compilation.is_valid());
    let receipt = execute_image_edit(
        &compilation,
        &request,
        &reference,
        &OpenAiRunConfig {
            curl_binary: script.clone(),
            output_path: output.clone(),
            timeout: Duration::from_secs(5),
            overwrite: false,
        },
    )
    .unwrap();
    assert_eq!(receipt.reference_index, 1);
    assert_eq!(receipt.reference_role, "base");
    assert_eq!(receipt.http_status, 200);
    assert_eq!(receipt.request_id.as_deref(), Some("req-fixture"));
    assert_eq!(fs::read(&output).unwrap(), fs::read(fixture()).unwrap());
    let stdin = fs::read_to_string(script.with_extension("sh.stdin")).unwrap();
    let argv = fs::read_to_string(script.with_extension("sh.argv")).unwrap();
    let prompt = fs::read_to_string(script.with_extension("sh.prompt")).unwrap();
    let sent_image = fs::read(script.with_extension("sh.image")).unwrap();
    assert_eq!(
        argv.split_whitespace().collect::<Vec<_>>(),
        ["--disable", "--config", "-"]
    );
    assert!(stdin.contains("request = \"POST\""));
    assert!(stdin.contains("url = \"https://api.openai.com/v1/images/edits\""));
    assert!(stdin.contains("form = \"model=gpt-image-2\""));
    assert!(stdin.contains("form = \"image[]=@"));
    assert!(stdin.contains("type=image/png\""));
    assert!(stdin.contains("form = \"prompt=<"));
    assert!(stdin.contains("form = \"output_format=png\""));
    assert!(stdin.contains("form = \"n=1\""));
    assert_eq!(stdin.matches("form = \"image[]=@").count(), 1);
    assert!(!stdin.contains("input_fidelity"));
    assert!(!stdin.contains(reference.to_string_lossy().as_ref()));
    assert!(!argv.contains("sk-test-only"));
    assert!(!argv.contains("compiled"));
    assert_eq!(prompt, compilation.prompt.as_deref().unwrap());
    assert_eq!(sent_image, fs::read(&reference).unwrap());
    let receipt_json = receipt.to_json().to_compact_string();
    assert!(!receipt_json.contains("sk-test-only"));
    assert!(!receipt_json.contains("user-source.png"));
    assert!(!receipt_json.contains(compilation.prompt.as_deref().unwrap()));
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn reference_output_alias_fails_before_transport_and_preserves_reference() {
    if !run_with_test_key(
        "tests::reference_output_alias_fails_before_transport_and_preserves_reference",
    ) {
        return;
    }
    use std::os::unix::fs::PermissionsExt;

    let root = test_root("reference-output-alias");
    let reference = root.join("source.png");
    fs::copy(fixture(), &reference).unwrap();
    let before = fs::read(&reference).unwrap();
    let curl = root.join("must-not-run.sh");
    fs::write(
        &curl,
        "#!/bin/sh\nprintf invoked > \"$0.invoked\"\nexit 99\n",
    )
    .unwrap();
    fs::set_permissions(&curl, fs::Permissions::from_mode(0o755)).unwrap();
    let request = edit_request();
    let compilation = compile_image_prompt(&request);
    let config = OpenAiRunConfig {
        curl_binary: curl.clone(),
        output_path: root.join("nested/../source.png"),
        timeout: Duration::from_secs(5),
        overwrite: true,
    };
    let error = execute_image_edit(&compilation, &request, &reference, &config).unwrap_err();
    assert_eq!(error.code, "OPENAI_REFERENCE_OUTPUT_ALIAS");
    assert_eq!(fs::read(&reference).unwrap(), before);
    assert!(!curl.with_extension("sh.invoked").exists());
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn reference_output_symlink_parent_alias_fails_before_transport() {
    if !run_with_test_key("tests::reference_output_symlink_parent_alias_fails_before_transport") {
        return;
    }
    use std::os::unix::fs::PermissionsExt;

    let root = test_root("reference-output-symlink-parent-alias");
    let real_parent = root.join("real-parent");
    fs::create_dir(&real_parent).unwrap();
    let alias_parent = root.join("alias-parent");
    std::os::unix::fs::symlink(&real_parent, &alias_parent).unwrap();
    let reference = real_parent.join("source.png");
    fs::copy(fixture(), &reference).unwrap();
    let before = fs::read(&reference).unwrap();
    let curl = root.join("must-not-run.sh");
    fs::write(
        &curl,
        "#!/bin/sh\nprintf invoked > \"$0.invoked\"\nexit 99\n",
    )
    .unwrap();
    fs::set_permissions(&curl, fs::Permissions::from_mode(0o755)).unwrap();
    let request = edit_request();
    let compilation = compile_image_prompt(&request);
    let config = OpenAiRunConfig {
        curl_binary: curl.clone(),
        output_path: alias_parent.join("source.png"),
        timeout: Duration::from_secs(5),
        overwrite: true,
    };
    let error = execute_image_edit(&compilation, &request, &reference, &config).unwrap_err();
    assert_eq!(error.code, "OPENAI_REFERENCE_OUTPUT_ALIAS");
    assert_eq!(fs::read(&reference).unwrap(), before);
    assert!(!curl.with_extension("sh.invoked").exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn unsupported_contracts_fail_before_curl_and_publication() {
    if !run_with_test_key("tests::unsupported_contracts_fail_before_curl_and_publication") {
        return;
    }
    let root = test_root("contract");
    let request = edit_request();
    let compilation = compile_image_prompt(&request);
    let script = root.join("must-not-run");
    fs::write(&script, "#!/bin/sh\nexit 99\n").unwrap();
    let config = OpenAiRunConfig::new(&script, root.join("output.png"));
    let mut generate = request.clone();
    generate.task_mode = ImageTaskMode::Generate;
    assert_eq!(
        execute_image_edit(&compilation, &generate, &fixture(), &config)
            .unwrap_err()
            .code,
        "OPENAI_MODE_UNSUPPORTED"
    );
    let mut many = request.clone();
    many.references.push(request.references[0].clone());
    assert_eq!(
        execute_image_edit(&compilation, &many, &fixture(), &config)
            .unwrap_err()
            .code,
        "OPENAI_REFERENCE_COUNT"
    );
    let mut wrong_role = request.clone();
    wrong_role.references[0].role = ImageReferenceRole::Style;
    assert_eq!(
        execute_image_edit(&compilation, &wrong_role, &fixture(), &config)
            .unwrap_err()
            .code,
        "OPENAI_REFERENCE_CONTRACT"
    );
    assert!(!root.join("output.png").exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn input_and_http_failures_never_publish() {
    if !run_with_test_key("tests::input_and_http_failures_never_publish") {
        return;
    }
    let root = test_root("failures");
    let request = edit_request();
    let compilation = compile_image_prompt(&request);
    let output = root.join("output.png");
    let bad = root.join("bad.bin");
    fs::write(&bad, b"not-an-image").unwrap();
    let script = root.join("must-not-run");
    fs::write(&script, "#!/bin/sh\nexit 99\n").unwrap();
    let config = OpenAiRunConfig::new(&script, &output);
    assert_eq!(
        execute_image_edit(&compilation, &request, &bad, &config)
            .unwrap_err()
            .code,
        "OPENAI_REFERENCE_SIGNATURE"
    );
    let script = fake_curl(&root, 500, "{\"error\":{\"message\":\"no\"}}");
    let config = OpenAiRunConfig::new(&script, &output);
    assert_eq!(
        execute_image_edit(&compilation, &request, &fixture(), &config)
            .unwrap_err()
            .code,
        "OPENAI_HTTP_STATUS"
    );
    assert!(!output.exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn malformed_json_and_wrong_png_fail_closed() {
    if !run_with_test_key("tests::malformed_json_and_wrong_png_fail_closed") {
        return;
    }
    let root = test_root("malformed");
    let request = edit_request();
    let compilation = compile_image_prompt(&request);
    let output = root.join("output.png");
    let script = fake_curl(&root, 200, "not-json");
    let config = OpenAiRunConfig::new(&script, &output);
    assert_eq!(
        execute_image_edit(&compilation, &request, &fixture(), &config)
            .unwrap_err()
            .code,
        "OPENAI_RESPONSE_JSON"
    );
    let script = fake_curl(&root, 200, "{\"data\":[{\"b64_json\":\"bm90LXBuaA==\"}]}");
    let config = OpenAiRunConfig::new(&script, &output);
    assert_eq!(
        execute_image_edit(&compilation, &request, &fixture(), &config)
            .unwrap_err()
            .code,
        "OPENAI_INVALID_PNG"
    );
    assert!(!output.exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn missing_key_fails_before_reference_or_curl() {
    if !run_without_test_key("tests::missing_key_fails_before_reference_or_curl") {
        return;
    }
    let root = test_root("missing-key");
    let request = edit_request();
    let compilation = compile_image_prompt(&request);
    let config = OpenAiRunConfig::new(root.join("never"), root.join("output.png"));
    assert_eq!(
        execute_image_edit(&compilation, &request, &fixture(), &config)
            .unwrap_err()
            .code,
        "OPENAI_API_KEY_MISSING"
    );
    assert!(!root.join("output.png").exists());
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn timeout_kills_fake_curl_without_publication() {
    if !run_with_test_key("tests::timeout_kills_fake_curl_without_publication") {
        return;
    }
    use std::os::unix::fs::PermissionsExt;

    let root = test_root("timeout");
    let script = root.join("slow-curl.sh");
    fs::write(&script, "#!/bin/sh\nset -eu\ncat >/dev/null\nsleep 3\n").unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    let request = edit_request();
    let compilation = compile_image_prompt(&request);
    let output = root.join("output.png");
    let config = OpenAiRunConfig {
        curl_binary: script,
        output_path: output.clone(),
        timeout: Duration::from_secs(1),
        overwrite: false,
    };
    assert_eq!(
        execute_image_edit(&compilation, &request, &fixture(), &config)
            .unwrap_err()
            .code,
        "OPENAI_TIMEOUT"
    );
    assert!(!output.exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn oversize_reference_fails_before_curl() {
    if !run_with_test_key("tests::oversize_reference_fails_before_curl") {
        return;
    }
    let root = test_root("oversize");
    let reference = root.join("oversize.png");
    let file = File::create(&reference).unwrap();
    file.set_len(reference::MAX_REFERENCE_BYTES).unwrap();
    let request = edit_request();
    let compilation = compile_image_prompt(&request);
    let config = OpenAiRunConfig::new(root.join("never"), root.join("output.png"));
    assert_eq!(
        execute_image_edit(&compilation, &request, &reference, &config)
            .unwrap_err()
            .code,
        "OPENAI_REFERENCE_SIZE"
    );
    assert!(!root.join("output.png").exists());
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn symlink_reference_is_rejected() {
    if !run_with_test_key("tests::symlink_reference_is_rejected") {
        return;
    }
    let root = test_root("symlink");
    let link = root.join("link.png");
    std::os::unix::fs::symlink(fixture(), &link).unwrap();
    let request = edit_request();
    let compilation = compile_image_prompt(&request);
    let config = OpenAiRunConfig::new(root.join("never"), root.join("output.png"));
    assert_eq!(
        execute_image_edit(&compilation, &request, &link, &config)
            .unwrap_err()
            .code,
        "OPENAI_REFERENCE_INPUT"
    );
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn changed_reference_identity_is_rejected_after_open() {
    let root = test_root("changed-reference-identity");
    let source = root.join("source.png");
    let moved = root.join("moved.png");
    fs::copy(fixture(), &source).unwrap();
    let initial_metadata = fs::symlink_metadata(&source).unwrap();
    let initial = reference_file_identity(&initial_metadata);
    let input = open_reference(&source).unwrap();
    fs::rename(&source, &moved).unwrap();
    fs::copy(fixture(), &source).unwrap();
    let error = ensure_reference_identity(&source, &input, initial, 0).unwrap_err();
    assert_eq!(error.code, "OPENAI_REFERENCE_CHANGED");
    drop(input);
    fs::remove_dir_all(root).unwrap();
}

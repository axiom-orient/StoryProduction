use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

use promptgen_core::image::{CanvasPlacement, ImagePromptRequest};
use promptgen_core::json::parse;

static NEXT: AtomicU64 = AtomicU64::new(1);

#[test]
fn codex_gif_relative_output_keeps_the_base_image_attached_after_cwd_change() {
    use std::os::unix::fs::PermissionsExt;
    let root = test_root("gif-reference-cwd");
    let mut request = ImagePromptRequest::from_json(
        parse(
            &fs::read_to_string(repository_root().join("examples/image-edit-product.json"))
                .unwrap(),
        )
        .unwrap(),
    )
    .unwrap();
    request.output.model = "gpt-image-2.6-sunburst".into();
    request.output.format = "gif".into();
    request.output.frame_count = Some(9);
    request.output.frame_duration_ms = Some(200);
    request.output.size_policy = Some("provider".into());
    fs::write(
        root.join("input.json"),
        request.to_json().to_pretty_string(),
    )
    .unwrap();
    fs::copy(
        repository_root().join("tests/fixtures/valid-1024x1536.png"),
        root.join("reference.png"),
    )
    .unwrap();
    let provider = root.join("codex-double");
    let script = format!(
        "#!/bin/sh\nset -eu\nif [ \"${{1-}}\" = --version ]; then printf 'codex-cli 0.168.0-alpha.7\\n'; exit 0; fi\nif [ \"${{1-}}\" = login ]; then printf 'Logged in using ChatGPT\\n'; exit 0; fi\nfound=0\nwhile [ \"$#\" -gt 0 ]; do\ncase \"$1\" in --image) test -f \"$2\" || exit 8; found=1; shift 2;; *) shift;; esac\ndone\ntest \"$found\" = 1 || exit 9\nmkdir -p \"$CODEX_HOME/generated_images/gif-thread\"\ncp '{}' \"$CODEX_HOME/generated_images/gif-thread/gif-call.gif\"\nprintf '%s\\n' '{{\"type\":\"thread.started\",\"thread_id\":\"gif-thread\"}}' '{{\"type\":\"turn.started\"}}' '{{\"type\":\"turn.completed\"}}'\n",
        repository_root()
            .join("tests/fixtures/codec-test-nineframes.gif")
            .display()
    );
    fs::write(&provider, script).unwrap();
    fs::set_permissions(&provider, fs::Permissions::from_mode(0o700)).unwrap();
    let result = Command::new(binary())
        .current_dir(&root)
        .args([
            "image",
            "--mode",
            "codex-imagegen",
            "--input",
            "input.json",
            "--image-output",
            "image.gif",
            "--reference-image",
            "reference.png",
            "--codex-binary",
        ])
        .arg(&provider)
        .arg("--codex-home")
        .arg(root.join("home"))
        .args(["--artifact-wait-seconds", "2", "--format", "json"])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(root.join("image.gif").is_file());
    assert!(String::from_utf8_lossy(&result.stdout).contains("chatgpt_subscription"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn codex_auth_options_are_not_silently_ignored_by_other_modes() {
    for mode in ["prompt-only", "openai-image-edit"] {
        let output = Command::new(binary())
            .args([
                "image",
                "--mode",
                mode,
                "--codex-home",
                "/unused-login-home",
            ])
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("openai-image-edit uses OPENAI_API_KEY")
        );
    }
}

#[test]
fn compiles_image_prompt_examples() {
    let root = repository_root();
    for (command, example, marker) in [
        ("image", "image-photo-lut.json", "warm_pastel_filmic"),
        ("image", "image-typography-poster.json", "text_element_1"),
        ("image", "image-pose-transfer.json", "자세 충실도 계약"),
        (
            "image",
            "image-editorial-tier2.json",
            "adult subject aged 25+",
        ),
    ] {
        let output = Command::new(binary())
            .args([command, "--input"])
            .arg(root.join("examples").join(example))
            .args(["--format", "json"])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(stdout.contains("\"status\": \"valid"));
        assert!(stdout.contains(marker));
    }
}

#[test]
fn strict_unknown_field_is_exit_three() {
    let root = test_root("unknown");
    let input = root.join("bad.json");
    let source =
        fs::read_to_string(repository_root().join("examples/image-photo-lut.json")).unwrap();
    fs::write(
        &input,
        source.replacen(
            "\"language\": \"ko\",",
            "\"language\": \"ko\",\n  \"unknown\": true,",
            1,
        ),
    )
    .unwrap();
    let output = Command::new(binary())
        .args(["image", "--input"])
        .arg(input)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    assert!(String::from_utf8_lossy(&output.stderr).contains("unknown field"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn catalog_has_six_representative_outcomes() {
    let output = Command::new(binary())
        .args(["catalog", "--family", "all", "--format", "text"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap().lines().count(), 6);
}

#[test]
fn output_is_not_overwritten_without_force() {
    let root = test_root("overwrite");
    let destination = root.join("result.json");
    fs::write(&destination, "sentinel").unwrap();
    let output = Command::new(binary())
        .args(["schema", "image", "--result"])
        .arg(&destination)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(fs::read_to_string(&destination).unwrap(), "sentinel");
    let output = Command::new(binary())
        .args(["schema", "image", "--result"])
        .arg(&destination)
        .arg("--force")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(
        fs::read_to_string(&destination)
            .unwrap()
            .contains("gpt-image-2")
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn guided_editorial_interview_asks_only_missing_fields_then_compiles() {
    let root = test_root("guided-editorial");
    let first = root.join("first.json");
    fs::write(
        &first,
        r#"{"kind":"image","brief":"30대 성인 모델 1명의 절제된 리조트웨어 세로 화보, 골든아워 해안 테라스.","answers":{}}"#,
    )
    .unwrap();
    let output = Command::new(binary())
        .args(["interview", "--input"])
        .arg(&first)
        .args(["--format", "json"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("\"status\": \"needs_input\""));
    assert!(text.contains("image.wardrobe"));
    assert!(text.contains("30대 성인 모델 1명"));

    let second = root.join("second.json");
    fs::write(
        &second,
        r#"{"kind":"image","brief":"30대 성인 모델 1명의 절제된 리조트웨어 세로 화보, 골든아워 해안 테라스.","answers":{"image.wardrobe":"세이지색 불투명 리넨 테일러드 재킷, 높은 라운드넥 크림 이너, 발목 길이 와이드 팬츠, 여유로운 핏"}}"#,
    )
    .unwrap();
    let output = Command::new(binary())
        .args(["interview", "--input"])
        .arg(&second)
        .args(["--format", "json"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("\"status\": \"ready\""));
    assert!(text.contains("\"render_profile\": \"editorial_flat\""));
    assert!(text.contains("클린 인물·라이프스타일 에디토리얼 이미지"));
    assert!(!text.contains("IMG_EDITORIAL_FLAT_LENGTH"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn mcp_stdio_external_agent_interview_retries_questions_and_returns_prompt() {
    let brief = "30대 성인 모델 1명의 절제된 리조트웨어 세로 화보, 골든아워 해안 테라스.";
    let metadata = r#""_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientInfo":{"name":"promptgen-cli-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}"#;
    let requests = [
        format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"server/discover","params":{{{metadata}}}}}"#
        ),
        format!(
            r#"{{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{{{metadata},"name":"promptgen_interview","arguments":{{"kind":"image","brief":"{brief}","answers":{{}}}}}}}}"#
        ),
        format!(
            r#"{{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{{{metadata},"name":"promptgen_interview","arguments":{{"kind":"image","brief":"{brief}","answers":{{"image.wardrobe":"세이지색 불투명 리넨 테일러드 재킷, 높은 라운드넥 크림 이너, 발목 길이 와이드 팬츠, 여유로운 핏"}}}}}}}}"#
        ),
        r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientInfo":{"name":"promptgen-cli-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}},"name":"promptgen_interview","arguments":{"kind":"audio","brief":"not an image","answers":{}}}}"#.to_owned(),
    ];

    let mut child = Command::new(binary())
        .args(["mcp", "--transport", "stdio"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start MCP stdio server");
    {
        let stdin = child.stdin.as_mut().expect("MCP stdin");
        for request in requests {
            writeln!(stdin, "{request}").expect("write MCP request");
        }
    }
    let output = child.wait_with_output().expect("wait for MCP stdio server");
    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let responses = String::from_utf8(output.stdout).expect("MCP responses are UTF-8");
    let lines = responses.lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), 4, "responses={responses}");
    let discover = lines
        .iter()
        .find(|line| line.contains("\"id\":1"))
        .expect("discover response");
    let needs_input = lines
        .iter()
        .find(|line| line.contains("\"id\":2"))
        .expect("needs_input response");
    let ready = lines
        .iter()
        .find(|line| line.contains("\"id\":3"))
        .expect("ready response");
    let invalid = lines
        .iter()
        .find(|line| line.contains("\"id\":4"))
        .expect("invalid response");
    assert!(discover.contains("\"supportedVersions\":[\"2026-07-28\"]"));
    assert!(needs_input.contains("\"status\":\"needs_input\""));
    assert!(needs_input.contains("\"id\":\"image.wardrobe\""));
    assert!(ready.contains("\"status\":\"ready\""));
    assert!(ready.contains("\"compilation\":{") && ready.contains("\"prompt\":\""));
    assert!(ready.contains("세이지색 불투명 리넨"));
    assert!(invalid.contains("\"error\":{") && invalid.contains("expected image"));
}

#[test]
fn interview_schema_is_exposed_by_cli() {
    let output = Command::new(binary())
        .args(["schema", "interview"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("promptGen guided interview request"));
    assert!(text.contains("\"answers\""));
}

#[test]
fn serve_refuses_non_loopback_bind() {
    let output = Command::new(binary())
        .args(["serve", "--bind", "0.0.0.0:4173"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(4));
    assert!(String::from_utf8_lossy(&output.stderr).contains("WEB_BIND_NOT_LOOPBACK"));
}

#[test]
fn image_prompt_only_rejects_image_output() {
    let root = repository_root();
    let output = Command::new(binary())
        .args(["image", "--input"])
        .arg(root.join("examples/image-photo-lut.json"))
        .args(["--image-output", "unused.png"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn duplicate_result_options_are_rejected_and_output_synonym_is_not_accepted() {
    let root = repository_root();
    let output = Command::new(binary())
        .args(["image", "--input"])
        .arg(root.join("examples/image-photo-lut.json"))
        .args(["--format", "json", "--format", "text"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("provided more than once"));

    let temp = test_root("result-flag-clean-break");
    let first = temp.join("first.json");
    let second = temp.join("second.json");
    let output = Command::new(binary())
        .args(["schema", "image", "--result"])
        .arg(&first)
        .arg("--result")
        .arg(&second)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("option --result was provided more than once")
    );
    assert!(!first.exists());
    assert!(!second.exists());

    let output = Command::new(binary())
        .args(["schema", "image", "--output"])
        .arg(&first)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("unknown option \"--output\""));
    assert!(!first.exists());
    fs::remove_dir_all(temp).unwrap();
}

#[test]
fn image_timeout_bounds_are_fail_closed() {
    let root = repository_root();
    for (option, value, maximum) in [
        ("--timeout-seconds", "3601", "3600"),
        ("--artifact-wait-seconds", "601", "600"),
        ("--timeout-seconds", "0", "3600"),
    ] {
        let output = Command::new(binary())
            .args(["image", "--input"])
            .arg(root.join("examples/image-photo-lut.json"))
            .args([option, value])
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(2),
            "option={option}, value={value}"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains(maximum), "stderr={stderr}");
    }
}

#[cfg(unix)]
#[test]
fn codex_mode_cli_publishes_verified_thread_artifact() {
    use std::os::unix::fs::PermissionsExt;

    let repository = repository_root();
    let root = test_root("cli-codex-success");
    let codex_home = root.join("codex-home");
    let image_output = root.join("output/generated.png");
    let result_output = root.join("output/receipt.json");
    let fixture = repository.join("tests/fixtures/valid-1024x1536.png");
    let script = root.join("fake-codex.sh");
    let input = root.join("input.json");
    let mut request = ImagePromptRequest::from_json(
        parse(
            &fs::read_to_string(repository.join("examples/image-photo-lut.json"))
                .expect("read example"),
        )
        .expect("parse example"),
    )
    .expect("decode example");
    for subject in &mut request.subjects {
        subject.placement = CanvasPlacement::Custom {
            x_percent: 0,
            y_percent: 0,
            width_percent: 100,
            height_percent: 100,
        };
    }
    fs::write(&input, request.to_json().to_pretty_string()).expect("write test input");
    let body = format!(
        "#!/bin/sh\nset -eu\nif [ \"${{1:-}}\" = \"--version\" ]; then echo 'codex-cli integration-test'; exit 0; fi\nresult=''\nprevious=''\nfor argument in \"$@\"; do\n  if [ \"$previous\" = '--output-last-message' ]; then result=\"$argument\"; fi\n  previous=\"$argument\"\ndone\nif [ -n \"$result\" ]; then\n  printf '%s\\n' '{{\"pass\":true,\"summary\":\"all hard constraints pass\",\"subject_counts\":[{{\"id\":\"coffee_cup\",\"expected\":1,\"observed\":1,\"evidence\":\"one cup\",\"instances\":[{{\"x_percent\":50,\"y_percent\":50,\"evidence\":\"cup\"}}]}},{{\"id\":\"beans\",\"expected\":9,\"observed\":9,\"evidence\":\"nine beans\",\"instances\":[{{\"x_percent\":10,\"y_percent\":80,\"evidence\":\"1\"}},{{\"x_percent\":20,\"y_percent\":80,\"evidence\":\"2\"}},{{\"x_percent\":30,\"y_percent\":80,\"evidence\":\"3\"}},{{\"x_percent\":40,\"y_percent\":80,\"evidence\":\"4\"}},{{\"x_percent\":50,\"y_percent\":80,\"evidence\":\"5\"}},{{\"x_percent\":60,\"y_percent\":80,\"evidence\":\"6\"}},{{\"x_percent\":70,\"y_percent\":80,\"evidence\":\"7\"}},{{\"x_percent\":80,\"y_percent\":80,\"evidence\":\"8\"}},{{\"x_percent\":90,\"y_percent\":80,\"evidence\":\"9\"}}]}}],\"text_checks\":[],\"violations\":[],\"repair_instruction\":\"\"}}' > \"$result\"\n  printf '%s\\n' '{{\"type\":\"thread.started\",\"thread_id\":\"thread-validator\"}}' '{{\"type\":\"turn.completed\"}}'\n  exit 0\nfi\nmkdir -p \"$CODEX_HOME/generated_images/thread-cli\"\ncp '{}' \"$CODEX_HOME/generated_images/thread-cli/call-cli.png\"\nprintf '%s\\n' '{{\"type\":\"thread.started\",\"thread_id\":\"thread-cli\"}}' '{{\"type\":\"turn.completed\"}}'\n",
        fixture.display()
    );
    fs::write(&script, body).unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();

    let output = Command::new(binary())
        .args(["image", "--input"])
        .arg(&input)
        .args(["--mode", "codex-imagegen", "--codex-binary"])
        .arg(&script)
        .arg("--codex-home")
        .arg(&codex_home)
        .arg("--image-output")
        .arg(&image_output)
        .arg("--result")
        .arg(&result_output)
        .args(["--timeout-seconds", "5", "--artifact-wait-seconds", "5"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read(&image_output).unwrap(),
        fs::read(&fixture).unwrap()
    );
    let receipt = fs::read_to_string(&result_output).unwrap();
    assert!(receipt.contains("\"thread_id\": \"thread-cli\""));
    assert!(receipt.contains("\"image_call_id\": \"call-cli\""));
    assert!(receipt.contains("\"width\": 1024"));
    assert!(receipt.contains("\"height\": 1536"));
    assert!(receipt.contains("\"fidelity_checks\""));
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn openai_edit_mode_binds_one_reference_and_publishes_verified_png() {
    use std::os::unix::fs::PermissionsExt;

    let repository = repository_root();
    let root = test_root("cli-openai-edit");
    let fixture = repository.join("tests/fixtures/valid-1024x1536.png");
    let reference = root.join("source-reference.png");
    fs::copy(&fixture, &reference).unwrap();
    let curl = root.join("curl");
    let body = r#"#!/bin/sh
set -eu
printf '%s' "$*" > "$0.argv"
cat > "$0.stdin"
response=$(awk -F '"' '$1 == "output = " {print $2}' "$0.stdin")
headers=$(awk -F '"' '$1 == "dump-header = " {print $2}' "$0.stdin")
encoded=$(base64 < "__FIXTURE__" | tr -d '\n')
printf '{"data":[{"b64_json":"%s"}]}' "$encoded" > "$response"
printf '%s\n' 'HTTP/1.1 200 OK' 'x-request-id: req-cli-fixture' > "$headers"
printf '%s' '200'
"#
    .replace("__FIXTURE__", &fixture.display().to_string());
    fs::write(&curl, body).unwrap();
    fs::set_permissions(&curl, fs::Permissions::from_mode(0o755)).unwrap();

    let image_output = root.join("output/generated.png");
    let result_output = root.join("output/receipt.json");
    let existing_path = std::env::var_os("PATH").unwrap_or_default();
    let path = format!("{}:{}", root.display(), existing_path.to_string_lossy());
    let output = Command::new(binary())
        .args(["image", "--input"])
        .arg(repository.join("examples/image-edit-product.json"))
        .args(["--mode", "openai-image-edit", "--image-output"])
        .arg(&image_output)
        .args(["--reference-image"])
        .arg(&reference)
        .args(["--result"])
        .arg(&result_output)
        .env("OPENAI_API_KEY", "sk-test-only")
        .env("PATH", &path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read(&image_output).unwrap(),
        fs::read(&fixture).unwrap()
    );
    let receipt = fs::read_to_string(&result_output).unwrap();
    assert!(receipt.contains("\"backend\": \"openai-images\""));
    assert!(receipt.contains("\"reference_index\": 1"));
    assert!(receipt.contains("\"reference_role\": \"base\""));
    assert!(receipt.contains("\"request_id\": \"req-cli-fixture\""));
    assert!(receipt.contains("\"compiled_prompt_sha256\""));
    assert!(receipt.contains("\"compiled_prompt_chars\""));
    assert!(!receipt.contains("\"compilation\""));
    assert!(!receipt.contains("\"prompt\""));
    assert!(!receipt.contains("sk-test-only"));
    assert!(!receipt.contains("source-reference.png"));

    let text_image_output = root.join("output/generated-text.png");
    let text_result_output = root.join("output/receipt.txt");
    let output = Command::new(binary())
        .args(["image", "--input"])
        .arg(repository.join("examples/image-edit-product.json"))
        .args([
            "--mode",
            "openai-image-edit",
            "--format",
            "text",
            "--image-output",
        ])
        .arg(&text_image_output)
        .args(["--reference-image"])
        .arg(&reference)
        .args(["--result"])
        .arg(&text_result_output)
        .env("OPENAI_API_KEY", "sk-test-only")
        .env("PATH", &path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let text_receipt = fs::read_to_string(&text_result_output).unwrap();
    assert!(text_receipt.contains("compiled_prompt_sha256="));
    assert!(text_receipt.contains("compiled_prompt_chars="));
    assert!(!text_receipt.contains("prompt="));
    assert!(!text_receipt.contains("sk-test-only"));
    assert!(!text_receipt.contains("source-reference.png"));
    let curl_stdin = fs::read_to_string(curl.with_extension("stdin")).unwrap();
    let curl_argv = fs::read_to_string(curl.with_extension("argv")).unwrap();
    assert_eq!(
        curl_argv.split_whitespace().collect::<Vec<_>>(),
        ["--disable", "--config", "-"]
    );
    assert!(curl_stdin.contains("request = \"POST\""));
    assert!(curl_stdin.contains("form = \"output_format=png\""));
    assert!(curl_stdin.contains("form = \"n=1\""));
    assert_eq!(curl_stdin.matches("form = \"image[]=@").count(), 1);
    assert!(!curl_stdin.contains("input_fidelity"));
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn openai_edit_rejects_reference_and_result_aliases_before_transport() {
    use std::os::unix::fs::PermissionsExt;

    let repository = repository_root();
    let root = test_root("cli-openai-alias");
    let fixture = repository.join("tests/fixtures/valid-1024x1536.png");
    let valid_input = repository.join("examples/image-edit-product.json");
    let invalid_input = root.join("invalid-edit.json");
    let invalid_source = fs::read_to_string(&valid_input).unwrap().replacen(
        "\"width\": 1024,",
        "\"width\": 3841,",
        1,
    );
    fs::write(&invalid_input, invalid_source).unwrap();
    let reference = root.join("source-reference.png");
    fs::copy(&fixture, &reference).unwrap();
    let before = fs::read(&reference).unwrap();
    let curl = root.join("curl");
    fs::write(
        &curl,
        "#!/bin/sh\nprintf invoked > \"$0.invoked\"\nexit 99\n",
    )
    .unwrap();
    fs::set_permissions(&curl, fs::Permissions::from_mode(0o755)).unwrap();
    let existing_path = std::env::var_os("PATH").unwrap_or_default();
    let path = format!("{}:{}", root.display(), existing_path.to_string_lossy());
    let image_output = root.join("output/generated.png");
    let reference_output = root.join("nested/../source-reference.png");
    let result_output = root.join("nested/../output/generated.png");

    let run = |input: &PathBuf,
               image: &PathBuf,
               reference_path: &PathBuf,
               result: Option<&PathBuf>,
               force: bool| {
        let mut command = Command::new(binary());
        command
            .args(["image", "--input"])
            .arg(input)
            .args(["--mode", "openai-image-edit", "--image-output"])
            .arg(image)
            .args(["--reference-image"])
            .arg(reference_path)
            .env("OPENAI_API_KEY", "sk-test-only")
            .env("PATH", &path);
        if let Some(result) = result {
            command.args(["--result"]).arg(result);
        }
        if force {
            command.arg("--force");
        }
        command.output().unwrap()
    };

    for force in [false, true] {
        let output = run(&valid_input, &reference_output, &reference, None, force);
        assert_eq!(output.status.code(), Some(2), "force={force}");
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("--reference-image and --image-output"),
            "force={force}, stderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(fs::read(&reference).unwrap(), before, "force={force}");
        assert!(!curl.with_extension("invoked").exists(), "force={force}");
        assert!(!reference_output.exists(), "force={force}");
    }

    for force in [false, true] {
        let output = run(
            &valid_input,
            &image_output,
            &reference,
            Some(&reference_output),
            force,
        );
        assert_eq!(output.status.code(), Some(2), "force={force}");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("--result and --reference-image"),
            "force={force}, stderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(fs::read(&reference).unwrap(), before, "force={force}");
        assert!(!curl.with_extension("invoked").exists(), "force={force}");
        assert!(!image_output.exists(), "force={force}");
    }

    for force in [false, true] {
        let output = run(
            &valid_input,
            &image_output,
            &reference,
            Some(&result_output),
            force,
        );
        assert_eq!(output.status.code(), Some(2), "force={force}");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("--result and --image-output"),
            "force={force}, stderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(fs::read(&reference).unwrap(), before, "force={force}");
        assert!(!curl.with_extension("invoked").exists(), "force={force}");
        assert!(!image_output.exists(), "force={force}");
    }

    let invalid_image_output = root.join("output/invalid-generated.png");
    let output = run(
        &invalid_input,
        &invalid_image_output,
        &reference,
        Some(&reference_output),
        true,
    );
    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("--result and --reference-image"),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read(&reference).unwrap(), before);
    assert!(!curl.with_extension("invoked").exists());
    assert!(!invalid_image_output.exists());

    let real_parent = root.join("real-parent");
    fs::create_dir(&real_parent).unwrap();
    let alias_parent = root.join("alias-parent");
    std::os::unix::fs::symlink(&real_parent, &alias_parent).unwrap();
    let symlink_reference = real_parent.join("source.png");
    fs::copy(&fixture, &symlink_reference).unwrap();
    let symlink_reference_alias = alias_parent.join("source.png");
    let symlink_image_output = real_parent.join("generated.png");
    let symlink_result_output = alias_parent.join("generated.png");
    let symlink_before = fs::read(&symlink_reference).unwrap();

    let output = run(
        &valid_input,
        &symlink_reference_alias,
        &symlink_reference,
        None,
        true,
    );
    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("--reference-image and --image-output")
    );
    assert_eq!(fs::read(&symlink_reference).unwrap(), symlink_before);
    assert!(!curl.with_extension("invoked").exists());
    assert!(!image_output.exists());

    let symlink_distinct_output = root.join("output/symlink-distinct.png");
    let output = run(
        &valid_input,
        &symlink_distinct_output,
        &symlink_reference,
        Some(&symlink_reference_alias),
        true,
    );
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("--result and --reference-image"));
    assert_eq!(fs::read(&symlink_reference).unwrap(), symlink_before);
    assert!(!curl.with_extension("invoked").exists());
    assert!(!symlink_distinct_output.exists());

    let output = run(
        &valid_input,
        &symlink_image_output,
        &symlink_reference,
        Some(&symlink_result_output),
        true,
    );
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("--result and --image-output"));
    assert_eq!(fs::read(&symlink_reference).unwrap(), symlink_before);
    assert!(!curl.with_extension("invoked").exists());
    assert!(!symlink_image_output.exists());

    let symlink_invalid_output = root.join("output/symlink-invalid.png");
    let output = run(
        &invalid_input,
        &symlink_invalid_output,
        &symlink_reference,
        Some(&symlink_reference_alias),
        true,
    );
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("--result and --reference-image"));
    assert_eq!(fs::read(&symlink_reference).unwrap(), symlink_before);
    assert!(!curl.with_extension("invoked").exists());
    assert!(!symlink_invalid_output.exists());

    let success_curl = r#"#!/bin/sh
set -eu
printf invoked > "$0.invoked"
cat > "$0.stdin"
response=$(awk -F '"' '$1 == "output = " {print $2}' "$0.stdin")
headers=$(awk -F '"' '$1 == "dump-header = " {print $2}' "$0.stdin")
encoded=$(base64 < "__FIXTURE__" | tr -d '\n')
printf '{"data":[{"b64_json":"%s"}]}' "$encoded" > "$response"
printf '%s\n' 'HTTP/1.1 200 OK' 'x-request-id: req-cli-symlink-distinct' > "$headers"
printf '%s' '200'
"#
    .replace("__FIXTURE__", &fixture.display().to_string());
    fs::write(&curl, success_curl).unwrap();
    fs::set_permissions(&curl, fs::Permissions::from_mode(0o755)).unwrap();
    let symlink_distinct_output = alias_parent.join("distinct-generated.png");
    let output = run(
        &valid_input,
        &symlink_distinct_output,
        &symlink_reference,
        None,
        true,
    );
    assert!(
        output.status.success(),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read(&symlink_distinct_output).unwrap(),
        fs::read(&fixture).unwrap()
    );
    assert_eq!(fs::read(&symlink_reference).unwrap(), symlink_before);
    assert!(curl.with_extension("invoked").exists());

    fs::remove_dir_all(root).unwrap();
}

fn binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_promptgen"))
}

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
        "promptgen-cli-test-{label}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn subcommand_help_is_clean_success() {
    for args in [
        vec!["image", "--help"],
        vec!["interview", "--help"],
        vec!["serve", "--help"],
        vec!["schema", "--help"],
        vec!["catalog", "--help"],
        vec!["lut-presets", "--help"],
    ] {
        let output = Command::new(binary())
            .args(&args)
            .output()
            .expect("help command should run");
        assert!(output.status.success(), "args={args:?}");
        assert!(
            output.stderr.is_empty(),
            "args={args:?}, stderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(!output.stdout.is_empty(), "args={args:?}");
    }
}

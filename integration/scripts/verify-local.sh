#!/usr/bin/env bash
set -euo pipefail
STORY_VERIFY_ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
cd "$STORY_VERIFY_ROOT"
STORY_CARGO_BIN=${STORY_CARGO_BIN:-cargo}
if ! command -v "$STORY_CARGO_BIN" >/dev/null 2>&1; then
  echo 'not_started: local verification requires Cargo; native production execution does not' >&2
  exit 3
fi
(
  cd geultu
  "$STORY_CARGO_BIN" build --locked --offline --release -p geultu-cli --bin geultu
)
"$STORY_CARGO_BIN" build --locked --offline --release --manifest-path NarrativeCourier/Cargo.toml -p narrative-courier --bin narrative-courier-cli
"$STORY_CARGO_BIN" build --locked --offline --release --manifest-path SceneAssetCompiler/Cargo.toml
"$STORY_CARGO_BIN" build --locked --offline --release --manifest-path SceneMotionCompiler/Cargo.toml -p smc-animatic
"$STORY_CARGO_BIN" fmt --manifest-path integration/Cargo.toml --all --check
"$STORY_CARGO_BIN" clippy --locked --offline --manifest-path integration/Cargo.toml --all-targets -- -D warnings
"$STORY_CARGO_BIN" build --locked --offline --release --manifest-path integration/Cargo.toml
STORY_TEST_STORYCTL="$STORY_VERIFY_ROOT/integration/target/release/storyctl" STORY_REQUIRE_OWNER_TESTS=1 STORY_REQUIRE_MEDIA_TESTS=${STORY_REQUIRE_MEDIA_TESTS:-1} "$STORY_CARGO_BIN" test --locked --offline --manifest-path integration/Cargo.toml
integration/target/release/storyctl verify-contracts
integration/target/release/storyctl doctor --scope text
integration/target/release/storyctl doctor --scope animatic
printf '%s\n' 'PASS: Story native local verification; owner-specific full gates and real provider/XPC qualification are separate'

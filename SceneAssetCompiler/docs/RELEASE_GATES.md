# SceneAssetCompiler 0.13.0 — 15 Release Gates

Release is fail-closed. `PASS` means the named gate actually completed on the source being delivered; historical test counts are not acceptance evidence. Current results belong to [IMPLEMENTATION_STATUS](IMPLEMENTATION_STATUS.md).

1. **Clean source scan** — no symlink, path-escape artifact, AppleDouble, `.DS_Store`, `__pycache__`, or `.pyc` in the release tree.
2. **Python syntax** — every deterministic Python source compiles.
3. **JSON parse** — every schema/template/fixture JSON parses.
4. **JSON Schema 2020-12** — every schema passes meta-validation with `jsonschema`.
5. **Canonical contract fixtures** — run-request, VisualAuthority, GenerationPlan, render contract, PSD GenerationPlan, SceneProductionPlan, GenerationUnit, CharacterAssetContract, PSDGenerationProfile, execution receipt, packaging-preservation evidence, canonical visual-review v3 with render-contract-specific axes, and scene review-lock contracts validate.
6. **Regression suite** — the full `unittest` discovery suite must pass. At least one test must be discovered; the suite must execute with `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests -v`, covering schema/runtime parity, strict Scene Provider v2 parsing, packaged-binary protocol probing, PNG/PerfectPixel boundary cases, bounded JSON framing, timeout/UNKNOWN custody, execution lineage, decoded PNG alpha/canvas/margins, Doctor, immutable attempts/selections, reference ownership, PSD strict postflight, path policies, and selective repair.
7. **Scene CLI smoke** — `scene-plan-check` and `scene-init` produce schema-valid units with optional `LAYOUT_LOCK`, independent `PLACEHOLDER_PHOTO` units, per-slot `PLACEHOLDER_LOCK`, aggregate `PLACEHOLDER_SET_LOCK`, explicit layer/PSD dependencies, and a dedicated `PSD_PACKAGING` unit. Missing or drifted layout evidence blocks asset sealing and PSD entry.
8. **PSD generation-capsule smoke** — single-asset and scene capsules contain the exact GPT PSD rules: native pixel layers, ordinary alpha, no pixel/user/vector-mask substitution, declared Named Saved Paths, correct clipping-path policy, exact layer IDs and sRGB profile request. Packaging retries bind an approved raster substrate, carry an exact attempt/receipt lock, require exactly one PRIMARY PSD, and cannot pass mechanical review without a contained `SUBMITTER_ATTESTED`, hash-bound submitted comparison attestation declaring zero-delta metrics. This gate checks internal consistency and file binding only; actual external comparison remains `[UNVERIFIED]`.
9. **Real PSD postflight regression** — previously generated real PSD fixtures are inspected by the v0.10 read-only validator. Known defects must match their pinned expected findings; new path, canvas, layer, or ICC regressions fail the gate.
10. **Boundary scan** — no direct OpenAI Images API implementation and no local cutout/matting/path-tracing/PSD-writing producer in the canonical SceneAssetCompiler code.
11. **Exact manifests** — run `PYTHONDONTWRITEBYTECODE=1 python3 tools/source_manifest.py verify`. The root `SOURCE_RELEASE_MANIFEST.sha256` is the authority and must report zero missing, extra, malformed, or hash-mismatched entries. The skill and documentation manifests must also be exact. The scope includes source, contracts, tests, fixtures, experiments, documentation, and tools; it excludes only `.git`, Finder metadata, Python bytecode/cache, and the manifest currently being generated. No repository-local build/temp output directories are excluded.
12. **Provider contract lock** — preflight must seal the actual executable SHA-256 and `misegen.provider/2` protocol (`schemaVersion: 2`) before dispatch; the v2 fake and packaged-binary negative probe must pass, and no v1 fallback is permitted.
13. **PerfectPixel evidence binding** — a successful PNG must include a read-only `perfectpixel.asset-inspection/1` result bound to the PNG hash, byte count, dimensions, alpha, standalone CLI/source pin, and reaped process group before visual review; stale/mismatched inspection is terminal `UNKNOWN`.
14. **Consumer boundary** — the renamed SceneMotionCompiler/NarrativeCourier callsites and offline end-to-end boundary must pass with no active retired-product path, module, schema, or environment namespace.
15. **Package integrity** — the requested `SceneAssetCompiler-Standalone-0.13.0` bundle must pass ZIP CRC and have SHA-256 + byte size recorded externally; its contents must include only canonical SceneAssetCompiler sources, pinned provider/PerfectPixel binaries, locks, licenses, and installation instructions.

## Manifest scope and cycle rules

`tools/source_manifest.py` is the only generator/verifier for the release
manifests. It walks regular files in lexical relative-path order. The package
contains no repository-local build/temp output directory; experiments,
fixtures, and returned-image evidence are intentional release inputs.

| Manifest | Walk root | Exclusions |
|---|---|---|
| `SOURCE_RELEASE_MANIFEST.sha256` | package root | the root manifest itself |
| `.agents/skills/scene-asset-compiler/SOURCE_MANIFEST.sha256` | skill root | the skill manifest itself |
| `docs/DOCS_MANIFEST.sha256` | `docs/` | the docs manifest itself |
Across all walked scopes, only `.git`, Finder metadata (`.DS_Store` and
`._*`), Python bytecode/cache (`__pycache__`, `.pyc`, `.pyo`), and each
manifest's own path are omitted. Symlinks are not release inputs and are
rejected by G01; they are never followed for hashing.

## Capability boundary

Actual Codex `$imagegen` PSD production is an external side effect. The repository supplies the production capsule and validates returned bytes; it never assembles a substitute PSD locally. PSD is a real user-observed capability, not globally classified as unsupported. An invocation may return `PREFERRED_OUTPUT_MISSING`; that invocation has not succeeded, but it is not global unsupported evidence. Each invocation must return a real PSD and pass the v0.10 ICC/layer/path contract before that attempt can succeed.

## 게이트 번호와 실제 도구

위15항은 의무 목록이다. `tools/verify_release.py`의 출력 G01–G11과 G12 external 표시는 번호 체계가 다르며 완전한 자동화 커버리지라는 뜻이 아니다. provider lock/PerfectPixel/live consumer/Standalone bundle은 별도 실행 증거가 필요하다. 이 source archive는 외부 실행 binary를 포함하는 Standalone bundle이 아니므로 그 package gate를 PASS로 주장하지 않는다.

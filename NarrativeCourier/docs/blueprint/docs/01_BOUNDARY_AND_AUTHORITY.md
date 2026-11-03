# 01. Boundary & Authority

| Owner | Owns | Must not own |
|---|---|---|
| STS | source fidelity, causality, character/world/knowledge state, ending, screenplay | camera/style/backend selection |
| Human/App | delivery intent, coverage, constraints, explicit backend pin | semantic fabrication |
| DirectorIR | visual grammar, projection operation, viewer-facing content, asset brief, presentation structure | plot/world-rule/ending mutation |
| Runtime | paths, hashes, locks, process lifecycle, cancel, publish atomicity | story/directing DTO or decisions |
| AgentExecutor | bounded reasoning call | canonical project state/publish authority |
| MediaBackend | capability descriptor and asset/render execution | Narrative reinterpretation |
| Verifier | compare artifact/evidence against contracts | silently repair another owner's artifact |

## Ownership rule for shared Runtime

Runtime에 타입/상태를 둘 수 있는 조건은 다음 중 하나다.

1. Runtime 자체가 그 lifecycle/invariant를 소유한다.
2. 두 개 이상의 reachable domain이 실제로 같은 타입을 공유한다.

단순히 “generic해 보인다”는 이유로 domain DTO를 Runtime에 두지 않는다.

## External boundary rule

외부 process/backend는 isolated workspace에만 쓴다. Runtime이 schema/hash/invariant/stale-owner 검증 후에만 canonical path로 publish한다.

## Human authority

- STS Human gate: Narrative meaning 변경 권한.
- DirectorIR review: projection 변경 권한.
- Renderer failure: Narrative meaning 변경 권한 없음.
- 새로운 `INVENT`가 필요하면 DirectorIR에서 만들지 않고 STS로 되돌린다.

# SceneMotionCompiler Composer — 분석

## Verdict·책임 경계

Composer는 검증된 SceneGraph와 로컬 asset map에서 self-contained HTML 문자열을 결정적으로 만든다. Narrative 의미, semantic-manifest 해석/qualification, asset approval, provenance 파일 게시, release custody는 소유하지 않는다. SceneGraph 구조 검증은 SMC schema registry를 사용하고 semantic-manifest 검증은 contracts owner에 위임한다.

| 모듈 | owner | 경계 |
|---|---|---|
| `src/scene-html.js` | HTML composition | SceneGraph·asset 참조·frame 총합을 검증하고 shot/environment/character/dialogue markup과 semantic-manifest embedding을 렌더링한다. 유일한 export는 `composeSceneHtml`이다. |
| `../contracts/src/trackb-semantic-manifest-contract.js` | semantic contract owner | HTML anchor/payload parsing, exact marker attributes, canonical bytes, manifest/entry hashes, occurrence uniqueness 및 meaning qualification을 검증한다. Application과 browser-renderer HTML consumers는 parser를 contracts facade에서 직접 호출한다; Composer는 object precondition만 위임한다. |

## 데이터 흐름·불변식

`SceneGraph + local assets + dimensions → schema/reference/frame checks → contracts semantic-manifest validation → HTML string + hidden non-executable manifest embedding`.

Composer는 semantic payload를 canonical JSON과 base64url로 만들어 숨겨진 비실행 div에 넣는다. Anchor/encoding identity와 raw-byte hashing은 contracts가 제공한다. HTML을 다시 읽는 application/browser-renderer 경로도 contracts parser를 직접 사용해 추출 정책을 한 owner에 둔다. 파일 저장과 provenance/hash 관계는 application owner의 책임이다.

## 구조 판단·검증 범위

HTML parser는 contracts의 단일 owner다. Composer 쪽 parsing adapter와 미사용 package barrel을 제거하고 application, browser-renderer, local tests를 contracts facade로 직접 옮겼다. SceneGraph composition과 validation은 유지한다. 이 clean break 뒤 tests/build/lint/runtime/source-manifest verifier는 실행하지 않았다; current qualification은 **NOT_RUN**이다.

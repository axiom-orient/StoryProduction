# Runtime Storage

프로젝트 루트의 실행 데이터는 `var/` 하나로 관리한다. `var/`는 git에서 제외되며 Studio가 실제로 저장할 때만 생성한다.

| 종류 | 위치 | 보존 규칙 |
| --- | --- | --- |
| 버전 관리 입력 예제 | `examples/` | 재현 가능한 JSON fixture로 유지 |
| Studio의 진행 중 입력 | 브라우저 메모리 | 저장하지 않으며 새로고침·종료 시 사라짐 |
| CLI·Studio 생성 결과 | `var/output/` | 이미지와 receipt 등 다시 만들 수 있는 결과 |
| 빌드 산출물 | `target/` | Cargo가 재생성하며 보관하지 않음 |

Studio 기본 출력은 `var/output/`이다. `promptgen serve --output-dir DIR`를 지정하면 생성 결과는 `DIR/`에 저장한다.

`var/input/`처럼 입력 폴더는 만들지 않는다. 사용자 입력과 참조 자료는 메모리에만 존재하며 저장하지 않는다. 지속되는 것은 생성 결과와 receipt뿐이며 기본 `var/output/`에, `promptgen serve --output-dir DIR` 지정 시에는 `DIR/`에 기록한다.

외부 전달 전에는 `scripts/cleanup_inventory.sh`로 후보와 용량을 확인하고 `scripts/clean.sh`를 실행한다. 이 명령은 재생성 가능한 `target/`과 `var/output/`만 제거한다.

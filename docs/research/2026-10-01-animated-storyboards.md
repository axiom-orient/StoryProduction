# 생성 이미지와 WebM으로 만드는 움직이는 스토리보드

조사일: **2026-10-01, Asia/Seoul**. 요청은 최신 자료를 중심으로 스토리보드를
재미있게 애니메이션하는 방법을 찾는 것이다. 이번 결과는 조사와 도입 설계다.
제품 코드 변경, 라이브러리 설치, 이미지·영상 생성, 실행 실험, 빌드·테스트는 하지 않았다.

## 결론과 추천

**생성 이미지 + 재사용 가능한 레이어 + 프레임 기반 움직임 + 모션 코믹 연출**을
주 경로로 추천한다. 작품마다 종이극장·움직이는 만화·수사 기록 같은 미술 방향을
선택하고, 중요한 행동과 정보 전환에 움직임을 집중한다. 편집 가능한 원본을
유지하면서 WebM을 공유·재생용 산출물로 만든다.

Remotion은 코딩 에이전트가 모션그래픽을 작성하는 방법을 배우고 표현을 탐색하기에
강한 후보다. 현재 StoryProduction에 들어가는 실행 경로는 SMC의 고정 Chromium과
공개 계약을 유지해야 하므로, 라이브러리 전체를 곧바로 새 중앙 엔진으로 넣는
결정은 하지 않는다. 기존 SMC에서 프레임별 합성을 확장하거나 계약을 지키는
별도 renderer adapter로 연결한다. 이 구분은 제작 방법과 현재 통합 제약에서
도출한 설계 판단이며 실험으로 확인한 우열이 아니다.

원본 그림을 그대로 편집할 수 있는 콘티는 대사·소품·인물의 옷·컷의 길이처럼
자주 바뀌는 요소에 유리하다. 복잡한 인체 연기나 회전이 필요한 일부 컷에는
image-to-video 후보를 선택적으로 사용할 수 있다. 모든 패널을 영상 모델에
넘기는 흐름을 기본으로 만들 필요는 없다.

## 1. 최근 확인한 변화

날짜는 **발표일**, **논문 제출/개정일**, **문서가 표시하는 갱신일**을 구별한다.
문서 갱신일만으로 기능이 그날 처음 출시됐다고 해석하지 않는다.

| 시점 | 직접 확인한 1차 자료 | 제품에 주는 의미 |
|---|---|---|
| 2026-09-30 문서 갱신 표시 | [Remotion Coding Agents](https://www.remotion.dev/docs/ai/coding-agents), [Skills](https://www.remotion.dev/docs/ai/skills) | Claude Code와 Codex 등 여러 코딩 에이전트를 명시적으로 지원한다. 특정 모델만의 제작 경로로 묶을 이유가 없다. |
| 2026-09-30 문서 갱신 표시 | [Remotion client-side rendering](https://www.remotion.dev/docs/client-side-rendering) | 브라우저 export는 v4.0.491부터 stable로 기술한다. WebCodecs/Mediabunny를 쓰며 CSS 표현 범위와 브라우저 조건이 있다. |
| 2026-09-22 발표 | [Anthropic Opus 5.5](https://www.anthropic.com/claude-opus-5-5) | 최신 코드 작성 능력의 참고다. 이 발표를 콘티 연출 품질의 Claude/Codex 비교 결과로 바꾸지 않는다. |
| 2026-09월 공개 changelog 항목 | [Runway changelog](https://runway.com/changelog) | 창작 도구·MCP·편집 앱과 생성 모델 연결이 확장되고 있다. 렌더링·작성·생성 provider를 따로 조합할 수 있는 방향이다. |
| 2026-07-26 개정 | [Spatula](https://arxiv.org/abs/2607.10405) | 생성된 모션그래픽을 세밀하게 고치는 속성 제어를 연구한다. 한 번 만든 애니메이션의 부분 수정이 제품 기능이어야 한다는 참고다. |
| 2026-04-26 제출 | [PhysLayer](https://arxiv.org/abs/2604.23574) | 깊이를 고려한 레이어 운동과 물리적 제어를 탐구한다. 평면 이미지 전체를 흔드는 것보다 레이어 관계가 중요한 근거다. |
| 2026-01-28 제출 | [SketchDynamics](https://arxiv.org/abs/2601.20622) | 스케치로 움직임 의도를 표현하고 재수정하는 상호작용을 연구한다. 화살표·카메라 경로·타이밍을 입력으로 쓸 수 있다. |
| 조사일 현재 공식 문서 | [OpenAI Image generation](https://developers.openai.com/api/docs/guides/image-generation), [Image prompting](https://developers.openai.com/api/docs/guides/image-prompting) | 참조 기반 편집과 투명 이미지 생성이 재료 제작에 쓰인다. 인물 일관성·정확한 배치·텍스트에는 여전히 한계가 명시된다. |
| 조사일 현재 공식 문서 | [LTX Supported Models](https://docs.ltx.io/models) | API에 LTX-2.5와 2.3이 함께 있고 endpoint 지원이 다르다. 오래된 2.3 소개 글만 보고 최신 capability를 결정하면 안 된다. |

### Claude의 모션그래픽 사례를 어떻게 해석할 것인가

Remotion의 [공식 showcase](https://www.remotion.dev/prompts)는 코딩 에이전트가 만든
여러 모션그래픽 사례를 보여준다. 공식 [창작 도구 설명](https://www.anthropic.com/news/claude-for-creative-work)도
코드·스크립트·shader·기존 도구 연결을 창작 방법으로 설명한다.

여기서 가져올 핵심은 **그림·벡터·텍스트를 코드로 배치하고 시간에 따라 바꾸는
제작 구조**다. 사례의 시각적 완성도에는 미술 방향, 재료, 타이밍, 수정 과정도
영향을 준다. 바이럴 영상이나 코딩 benchmark로 모든 이야기의 연출 품질을
서열화하지 않는다. Claude와 Codex 모두 이 구조에서 작성자로 참여할 수 있다.

## 2. WebM이 담당하는 일

[WebM 공식 container 설명](https://www.webmproject.org/docs/container/)에 따르면
WebM은 영상과 오디오를 담는 컨테이너다. 움직임을 작성하는 엔진은 별도로 필요하다.

제작물에는 두 가지가 남아야 한다.

| 결과 | 역할 | 보존할 내용 |
|---|---|---|
| 편집 가능한 콘티 composition | 컷별로 고치고 다시 재생 | 원본 이미지, 레이어, 타이밍, sourceRefs, 움직임과 자막의 실제 값 |
| WebM 재생본 | 앱·브라우저·메신저 등에서 보거나 공유 | 해당 composition의 특정 revision, 렌더링 조건, 파일 hash |

WebM만 저장하면 인물 한 명의 위치나 대사 한 줄을 고치는 데 필요한 구조가
사라진다. 객체 선택·이동·컷 타이밍 조정은 composition에서 하고, 영상은 해당
작업물의 재생본으로 제공하는 것이 맞다. 이는 파일 형식에서 도출한 설계 판단이다.

### 불투명 WebM과 투명 WebM

- **기본 결과:** 배경까지 합성한 불투명 WebM. 콘티 작품 전체를 전달한다.
- **선택 결과:** 인물·효과·타이틀 overlay를 담는 alpha WebM. 호스트 합성에 쓴다.
- **공유용 다른 포맷:** 같은 composition에서 명시적으로 만든 MP4 등.
  다른 파일의 확장자만 바꾸는 방식은 아니다.

[Remotion 투명 영상 문서](https://www.remotion.dev/docs/transparent-videos)는 VP8/VP9,
PNG frame, yuva420p 조건과 투명 재생의 브라우저 차이를 설명한다.
[Mediabunny quick start](https://mediabunny.dev/guide/quick-start)는 CanvasSource의
VP9와 alpha 보존 옵션을 보여준다. 두 경로 모두 실제 alpha를 가진 입력이 필요하다.

WebKit은 [Safari 17.4에서 iOS/iPadOS의 WebM 재생 지원](https://webkit.org/blog/15063/webkit-features-in-safari-17-4/)을
추가했다. 그러나 **파일 재생, 특정 codec 인코딩, alpha 재생**은 서로 다른 능력이다.
해당 발표만으로 최신 모든 Apple 환경의 투명 WebM을 보장하지 않는다.
기본 패키지를 불투명으로 합성하면 transparency에 대한 의존을 줄일 수 있다.

## 3. 재미있게 보이는 여섯 가지 연출 방향

다음은 조사에서 도출한 **StoryProduction을 위한 창작 제안**이다. 특정 공급자의
출시 기능 목록이나 연구 실험을 그대로 재현한 결과가 아니다.

### A. 움직이는 만화

페이지에 있던 컷이 열리고, 카메라가 중요한 컷 안으로 들어간다. 말풍선은 인물이
말하는 순간에 나타나고, 시선이 이동한 대상에서 다음 컷이 열린다. 결심하거나
충격을 받는 순간에는 패널 모양·구도를 바꾸며 감정의 전환을 표현한다.

한 프레임 전체를 계속 움직이기보다 **정지 → 행동 한 번 → 여운**을 만든다.
작가의 문장이 잘 보이도록 말풍선·자막은 이미지 속에 굽지 않고 실제 텍스트로
그린다. 폭발·충격선·비 같은 효과는 장면에 있는 사건과 분위기에 맞춘다.

맞는 작품: 대사 중심 단편, 미스터리, 코미디, 감정의 반전이 있는 이야기.

### B. 종이극장과 2.5D 무대

배경·인물·전경 소품을 몇 개의 평면으로 나눈다. 카메라가 움직일 때 가까운 레이어는
크게, 먼 레이어는 작게 이동시킨다. 인물은 종이 인형처럼 위치와 자세를 바꾸고,
문·커튼·그림자·빛 같은 요소가 공간의 깊이를 만든다.

원 그림에 없는 인물의 옆모습이나 가려진 공간을 작은 pan에서 임의로 발명하지
않는다. 큰 회전이 필요하면 별도 자세/시점의 그림을 만든다. 얼굴의 과도한 mesh
변형보다 승인된 포즈 간 전환을 우선한다.

맞는 작품: 동화, 판타지, 기억·회상, 분위기 중심의 짧은 이야기.

### C. 수사 기록처럼 읽는 콘티

사진·원문 인용·장소·관계선이 순서대로 등장한다. 카메라가 단서로 이동하고,
소리나 시선이 다음 사건으로 연결된다. 사건 순서와 독자가 알아가는 순서를
구분해 화면에서 드러낼 수 있다.

중요한 것은 시각적으로 강조하는 정보도 원 씬의 sourceRefs에서 가져오는 것이다.
관계선·인물의 믿음·단서를 새 사실로 확정하지 않는다. 가설은 가설로 표시한다.

맞는 작품: 추리, 스릴러, 복잡한 관계와 시점이 있는 이야기.

### D. 한 컷 안의 미세한 이야기

배경은 멈춰 있고 문 아래의 빛만 변한다. 인물의 시선이 움직인 다음, 카메라가
손에 든 물건으로 천천히 다가간다. 짧은 침묵을 두고 상대의 반응 컷으로 전환한다.

이 방식은 큰 동작이 없어도 장면의 압력과 정보를 전달한다. 소품·시선·빛·거리 중
핵심 하나를 선택한다. 모든 컷에 줌·흔들림·반짝임을 동시에 넣는 방식보다
이야기의 호흡을 분명히 설계할 수 있다.

맞는 작품: 생활극, 심리극, 조용한 긴장, 작가의 생략이 중요한 글.

### E. 문장에서 장면으로 넘어가기

원작의 한 문장이 화면에 나타나고, 그 문장과 연결된 그림으로 시선이 이동한다.
시간이나 장소를 알려 주는 문구는 장면의 일부처럼 배치하며, 필요할 때만 화면을
채운다. 대사의 강조는 음절을 난무시키기보다 의미 단위와 인물의 반응을 맞춘다.

한국어 줄바꿈·고유명사·따옴표를 코드의 실제 문자열로 유지한다. 모든 서술을
큰 타이포그래피로 바꾸지 않고, 작품에 필요한 장면과 문장의 비율을 선택한다.

맞는 작품: 작가의 문체가 핵심인 작품, 에세이 성격의 서사, 오프닝·챕터 전환.

### F. 재생하면서 고치는 콘티

사용자가 컷을 선택하면 그 컷에서 멈추고, “시선을 더 늦게”, “이 대사는 0.5초 더
보여줘”, “배경은 그대로, 인물만 왼쪽으로”를 반영한 후보를 받는다. 전체 영상을
다시 생성하는 대신 해당 타이밍·레이어·컷만 수정한다.

스케치 화살표와 경로도 움직임 의도를 주는 입력으로 쓸 수 있다. 이 생각은
[SketchDynamics](https://arxiv.org/abs/2601.20622)와 [Spatula](https://arxiv.org/abs/2607.10405)의
상호작용 연구에서 영감을 받았지만, 이 저장소에서 검증된 UI는 아니다.
호스트 UI는 별도 owner에 두고 StoryProduction에는 업무 계약을 제공한다.

## 4. 애니메이션 방법의 선택

| 방법 | 잘하는 일 | 필요한 재료 | 수정 단위 | 이 프로젝트의 선택 |
|---|---|---|---|---|
| 패널 hold·cut·카메라 crop | 이야기의 리듬과 컷 전환 | 완성된 패널 그림 | 패널·카메라·duration | 가장 먼저 연결할 바탕 |
| 레이어 2.5D·cutout | 깊이·시선·작은 행동·소품 | 배경/인물/소품 RGBA 또는 승인된 분리 재료 | 레이어·포즈·경로 | 주 표현 방식 |
| SVG·타이포그래피·도형 | 말풍선·관계·설명·전환 | 실제 문자열과 vector graphics | 문구·도형·시간 | 그림과 함께 사용 |
| pose-swap 제한 애니메이션 | 표정 변화·짧은 반응·발걸음 | 동일 인물의 승인된 자세들 | 특정 자세·beat | 인물 형태를 유지할 필요가 있을 때 |
| shader·입자·displacement | 비·빛·안개·물결·꿈 | 원본과 효과 규칙 | 효과·amplitude | 장면의 의미에 맞춰 제한 사용 |
| skeletal/mesh rig | 반복되는 복잡한 관절 운동 | 분리된 몸체·rig·weights | 관절과 타이밍 | 반복 캐릭터가 충분히 쌓였을 때 |
| image-to-video | 복잡한 회전·연기·자연스러운 연속 운동 | reference frame, 필요 시 end frame | 생성 clip 단위 | 필요한 컷만 선택 후보 |
| 3D 장면/카메라 | 실제 깊이·큰 시점 이동 | 3D assets, scene setup | 물체·카메라 | 2.5D로 요구를 표현할 수 없을 때 |

### 생성 이미지에서 레이어 얻기

권고 순서는 미술 reference 결정 → 장면/인물/소품의 재사용 재료 작성 → 필요한 컷
composition → 해당 재료의 검토다. 이미지 생성·편집 호출은 선택한 executor와
기존 SAC 경계를 따른다. 참조 기반 포즈 편집, 실제 alpha 배경, 인물이 없는 clean
plate 같은 재료를 명시적으로 요청한다.

같은 인물이 매 컷마다 새로 생성되면 얼굴·옷·비율이 달라질 수 있다. 기본 reference와
승인된 pose를 반복 사용하고, 새 시점이 필요할 때만 후보를 더 만든다.
OpenAI 공식 문서도 반복 인물의 일관성이 언제나 유지되는 것은 아니라고 명시한다.

완성된 평면 그림에서 인물만 분리할 때는 잘린 영역 뒤에 무엇이 남는지도 필요하다.
인물 cutout만 얻어 배경에 구멍이 난 채로 움직이면 안 된다. clean plate와
전경의 가림 관계를 함께 준비한다. 이는 합성 구조에 관한 설계 요구다.

레이어 분해 기술로는 [Qwen-Image-Layered](https://arxiv.org/abs/2512.15603)가 단일
RGB에서 여러 RGBA 레이어를 만드는 방향을 제시한다. 원 논문은 2025-12 자료이므로
최신 2026 출시라고 쓰지 않는다. 이번 저장소에서는 그 코드를 가져오거나 local
segmentation/PSD 조립을 추가하지 않는다. 현재 SAC가 허용하는 executor로 재료를
얻는 방법과, 향후 별도 provider를 검토하는 방법은 구별한다.

### 이미지 생성과 모션그래픽을 결합하는 연구

[MG-Gen](https://arxiv.org/abs/2504.02361)은 raster 이미지에서 layer/HTML 구조와
animation script를 만들고 영상으로 렌더링하는 연구다. 초기 자료의 vector 변환
설명만 인용하지 않고 2025-07 개정 abstract에서 확인한 현재 방법을 기준으로 읽었다.

[PhysLayer](https://arxiv.org/abs/2604.23574)는 깊이와 움직임 제어를 결합한다.
이들 자료가 주는 방향은 **이미지를 움직임을 제어할 수 있는 구조로 다루기**다.
논문의 성능 수치를 StoryProduction의 성능이나 성공률로 재사용하지 않는다.

## 5. 도구와 렌더러 비교

| 도구 | 공식 자료에서 확인한 역할 | 주의할 경계 | 권고 위치 |
|---|---|---|---|
| [Remotion](https://www.remotion.dev/docs/ai/coding-agents) | React composition과 프레임 기반 영상, 코딩 agent 작성 경로 | runtime state/실시간 CSS를 그대로 영상 프레임으로 취급하지 않음 | 표현 탐색 및 명시적 renderer adapter 후보 |
| [Remotion web-renderer](https://www.remotion.dev/docs/web-renderer/render-media-on-web) | 브라우저에서 WebM/codec/alpha 설정으로 export | CSS subset, codec 가용성, telemetry | 조건이 맞는 외부 host의 export 선택지 |
| [PixiJS](https://pixijs.com/8.x/guides/components/scene-objects/sprite) | 이미지 sprite의 위치·크기·회전·texture 합성 | 실시간 ticker와 재현 가능한 frame clock을 구별 | SMC의 레이어 합성 후보 |
| [GSAP Timeline](https://gsap.com/docs/v3/GSAP/gsap.timeline()) | tween sequencing, pause/seek | seek callback과 부수효과를 명시적으로 다뤄야 함 | 허용된 렌더러 내부의 제한된 움직임 도구 |
| [Motion AI Kit](https://motion.dev/docs/ai-kit) | agent용 최신 docs·예제·transition 도구 | 자체가 WebM encoder는 아님 | 움직임 작성 참고·외부 host 상호작용 |
| [Motion Canvas](https://motion-canvas.io/docs/) | generator 기반 TypeScript animation과 실시간 preview | 별도 editor/exporter 체계와 시간 모델 도입 | 설명형 vector 영상의 대안 |
| [Mediabunny](https://mediabunny.dev/guide/quick-start) | Canvas/video/audio를 파일로 encode/mux | 브라우저 codec 지원이 library만으로 보장되지 않음 | 브라우저의 명시적 WebM export |
| [Rive](https://rive.app/docs/editor/exporting/exporting-for-video-and-static-design) | 편집된 animation을 WebM·영상·PNG 등으로 export | editor/runtime 파일과 cloud render 운영 경계 | 재사용 캐릭터/정교한 vector motion 대안 |
| [Runway image-to-video](https://help.runwayml.com/hc/en-us/articles/48324313115155-Image-to-Video-Prompting-Guide) | 입력 그림의 구도·style을 바탕으로 움직임 생성 | 원치 않는 camera/cut와 reference의 결함 가능성 | 일부 clip 후보 |
| [Firefly image-to-video](https://helpx.adobe.com/ca/firefly/web/work-with-audio-and-video/work-with-video/generate-videos-using-images.html) | first/last frame과 camera 선택 | end frame을 넣으면 motion 옵션이 비활성화되는 등 조합 제약 | 별도 provider 후보 |
| [LTX API](https://docs.ltx.io/models) | image/audio 기반 생성과 모델별 endpoint | 최신 모델이 모든 retake/extend를 지원하는 것은 아님 | 명시적 모델/capability를 가진 선택 adapter |

### Remotion으로 하면 되는 것과 통합 전에 결정할 것

Remotion 공식 [animation 지침](https://www.remotion.dev/docs/animating-properties)은
animation을 현재 frame에서 계산해야 하는 이유를 설명한다. 프레임 48의 위치·
투명도·현재 pose가 같은 plan에서 동일하게 결정되는 방식이 콘티 export에 적합하다.

브라우저 export의 [제약](https://www.remotion.dev/docs/client-side-rendering/limitations)은
전체 CSS screenshot과 자체 canvas 표현의 차이를 명시한다. perspective·일부 mask·
blend 표현 등이 제한되고 Safari/WebKit filter 지원에도 차이가 있다. HTML-in-canvas
선택 경로가 있지만 현재 고정 Chromium의 지원 여부를 확인하지 않은 상태다.

또한 [telemetry 문서](https://www.remotion.dev/docs/telemetry)는 client-side render가
이벤트를 보낸다고 명시한다. [라이선스](https://www.remotion.dev/docs/license/pricing)는
개인/작은 조직과 더 큰 조직의 조건을 구별한다. 완전 로컬·네트워크 차단 renderer에
브라우저 export를 숨은 효과 없이 넣을 수 있다고 단정하지 않는다.

따라서 standalone motion experiment의 장점과 StoryProduction core integration의
조건을 구분한다. 현재 SMC의 locked CDP, artifact identity, human gates를 깨거나
새 Puppeteer/Playwright 자동화를 추가하는 방향을 권고하지 않는다.

## 6. WebM export를 선택하는 세 가지 경로

### 경로 1: 로컬에서 정확한 프레임을 렌더링하고 encode

composition의 frame 0..N-1을 렌더링하고 WebM을 만든다. 본문과 모든 자산을 먼저
준비하고, 공통 camera transform 아래 인물·배경·소품을 배치한다. 같은 plan을
정지 그림, WebM, 선택된 다른 영상 포맷으로 투영할 수 있다.

현재 저장소에 맞는 우선 도입안이다. encoder와 browser를 pin하고, 임시 frame이나
부분 영상은 candidate로 보존한다. 렌더링 완료를 narrative/visual 승인으로 바꾸지 않는다.

### 경로 2: 브라우저에서 frame composition을 encode

Canvas 또는 지원되는 Remotion composition을 WebCodecs/Mediabunny로 내보낸다.
네트워크 서버 없이 export할 수 있는 기술적 선택지다. [VideoEncoder.isConfigSupported](https://developer.mozilla.org/en-US/docs/Web/API/VideoEncoder/isConfigSupported_static)와
[Mediabunny codec 가용성](https://mediabunny.dev/guide/supported-formats-and-codecs)을
고려해야 하며, 재생 가능한 codec이라고 encode도 가능하다고 가정하지 않는다.

실패하면 사용자의 편집 원본과 이미 만들어진 파일을 유지하고 지원되지 않는 경로를
명확히 표시한다. opaque WebM과 alpha WebM도 별도 capability다.

### 경로 3: 재생 화면을 실시간으로 녹화

[Canvas captureStream](https://developer.mozilla.org/en-US/docs/Web/API/HTMLCanvasElement/captureStream)과
[MediaRecorder](https://developer.mozilla.org/en-US/docs/Web/API/MediaRecorder/isTypeSupported_static)를
이용하는 접근이다. 빠른 화면 기록에는 쓸 수 있지만, 프레임 수·카메라·컷을 정확히
재현할 핵심 export로 먼저 선택하지 않는다. 실제시간 재생과 프레임별 평가가 다른
방식이므로 dropped frame·탭 상태·오디오 clock을 다뤄야 한다는 설계 판단이다.

### 오디오와 재생 경험

효과음·대사·침묵은 컷의 시간과 연결한다. 원문에 없는 대사를 임의 생성하지 않는다.
음성을 쓸 경우 실제 생성/사용한 음성 자산과 권한도 기록한다. 음악의 박자만으로
인물의 선택이나 중요한 침묵을 잘라내지 않는다.

[브라우저 autoplay 정책](https://developer.mozilla.org/en-US/docs/Web/Media/Guides/Autoplay)을
고려해 사용자 재생 동작으로 소리를 시작하고, 무음 재생과 실제 대사 자막도 제공한다.
효과와 camera motion을 줄인 표현에서도 장면 정보가 남아야 한다.

## 7. 현재 저장소에서 발견한 재사용 지점과 공백

아래는 이번에 읽은 소스와 계약의 범위다. 동작 실험이나 제품 전체 재검증을 수행한
결과가 아니다. 9월의 다른 검증 기록을 이번 기능의 증거로 가져오지 않는다.

| 현재 근거 | 읽은 사실 | 도입에서 필요한 변화 |
|---|---|---|
| [NVC text storyboard v1](../../NarrativeCourier/contracts/text-storyboard-v1.schema.json) | panelId, composition, staging, sourceRefs, soundNotes, transition이 있다. fps/duration/layer keyframes는 없다. | v1에 필드를 몰래 더하지 말고 새 animatic plan 계약으로 timing/motion 결속 |
| NVC storyboard entrypoint (`../../NarrativeCourier/tools/storyboard.py`, 당시 경로) | 현재는 storyboard_impl 공개 CLI로 위임한다. | 이 owner에서 현재 board에 연결된 timing/animation candidate 작성·검토 경로 |
| [SMC scene HTML composer](../../SceneMotionCompiler/packages/composer/src/scene-html.js) | 이미지, 기본 pan/push, CSS 전환과 절대 frame 기반 재생이 있다. 일부 미배정 자산의 표현도 있다. | 고급 콘티에 필요한 공통 scene camera, exact seek(frame), layer/pose 조작과 실제 자산 상태 |
| [SMC browser renderer exports](../../SceneMotionCompiler/packages/browser-renderer/src/index.js) | 공개 export는 HTML 측정 경로다. | 측정과 별개의 explicit WebM render/encode entrypoint |
| [SMC clip probing](../../SceneMotionCompiler/packages/production/src/io.js) | probeClip은 MP4/MOV 확장자만 허용하고 mov demuxer를 지정한다. | WebM을 기존 clip처럼 받으려면 새 지원·observation 계약 필요. rename으로 우회 금지 |
| [SMC production dispatcher](../../SceneMotionCompiler/packages/production/src/index.js) | motion request, clip ingest/review, editorial, OTIO 단계가 있다. | 합성 animatic은 generative provider clip과 구별되는 renderer 후보로 연결 |
| [SAC visual reference contract](../../SceneAssetCompiler/docs/contracts/visual-reference-pack-v1.schema.json) | 기존 cinematic/sceneGraph/continuity binding에 연결된다. | 텍스트 board 기반 새 자산 계약을 만들거나 승인된 기존 cinematic 경로를 실제로 거칠 것. 가짜 binding 생성 금지 |
| [SAC 규칙](../../SceneAssetCompiler/AGENTS.md) | pixel executor, 참조·cutout 경계, local matting/PSD 조립 금지가 있다. | 레이어 재료를 허용된 executor가 생산. 렌더러는 승인된 재료의 합성·이동만 담당 |

기본 화면의 pan을 이미 고품질 animatic이라고 부를 수는 없다. 현재 구현의 입력·
출력 경계를 활용하면서 필요한 부분을 명확히 추가해야 한다.

## 8. 한 작품을 이렇게 만들면 된다

아래는 제작 흐름 제안이다. 실제 사용자 원고에 실행한 예제가 아니다.

1. **원문과 콘티를 읽고 미술 방향을 정한다.** 감정 중심이면 종이극장,
   대사·반전 중심이면 움직이는 만화, 복잡한 정보면 기록물 방식 등.
2. **한 장의 style reference와 재사용 인물/소품 기준을 만든다.** style reference는
   캐릭터 정체성이나 서사 사실의 자동 승인으로 취급하지 않는다.
3. **배경·인물 포즈·핵심 소품의 그림을 만든다.** 같은 재료를 여러 패널에 재사용한다.
4. **패널별 핵심 움직임을 정한다.** 어디를 언제 보게 할지, 무엇이 변하고 무엇은
   멈춰 있을지, 중요한 행동의 앞뒤 hold와 컷의 타이밍을 후보로 작성한다.
5. **장면 하나를 편집 가능한 composition으로 묶는다.** sourceRefs와 실제 이미지
   hash를 연결하고 가려짐·crop·text를 관리한다.
6. **작품의 호흡을 직접 수정한다.** 수정한 컷의 타이밍과 자산만 갱신한다.
7. **WebM 재생본과 원본 묶음을 만든다.** 인코딩 파일만 남기는 방식으로 끝내지 않는다.
8. **고급 연기가 필요한 컷만 추가 제작한다.** 승인된 시작/끝 그림을 제공해
   image-to-video 후보를 만들고 원문과 맞는지 별도로 판단한다.

### 호출량과 수정 비용을 줄이는 방법

예를 들어 8개 장소 배경, 4명의 인물에 각 3개 자세, 10개 소품, 4개 효과를
따로 만드는 계획이라면 재료 후보는 8 + 12 + 10 + 4 = 34개다. 이는 구조를
설명하기 위한 가정과 계산이며 실제 생성 호출 수나 품질 성공률 보장이 아니다.

핵심은 패널 수마다 얼굴·옷·소품을 다시 생성하지 않는 것이다. 이미지 재료가
유지되는 동안 컷의 길이·카메라·자막 변경은 model generation 없이 renderer에서
다시 만들 수 있다. 수정의 비용을 자산 생성과 렌더링으로 나눌 수 있다.

## 9. 구현 순서와 결정이 바뀌는 조건

### 권고하는 첫 연결

현재 승인된 board → 실제 패널/레이어 자산 → 명시적인 timing/motion plan →
SMC frame composition → opaque WebM candidate → 사람 검토.
이어서 pose-swap, 깊이 레이어, 말풍선, 소리 cue를 추가한다.

처음부터 모든 컷을 shader·skeleton·3D·I2V로 처리하는 플랫폼을 만들지 않는다.
작품의 핵심 장면에서 필요한 표현을 선택하고, 같은 source와 editing 의미를
유지하는 adapter만 확장한다.

### 다른 선택이 더 적합할 때

- 인물이 지속적으로 몸을 돌리거나 물리적으로 상호작용해야 한다면 I2V/rig/3D가 유리할 수 있다.
- 벡터 설명형 콘텐츠가 대부분이면 Motion Canvas가 더 직접적인 대안일 수 있다.
- 반복 캐릭터의 정교한 state animation이 핵심이면 Rive/rig 제작의 투자가 유리할 수 있다.
- 임의 CSS·복잡한 mask가 필요하면 Remotion의 browser export subset보다
  pinned browser frame capture가 적합할 수 있다.
- 배포 host의 browser encode·alpha support가 충분하지 않으면 로컬 export를 유지한다.

위 조건은 후속 구현에서 실제 작품과 지원 조건으로 판단할 기준이다. 이번 조사에서
그림이나 영상을 생성해 비교하거나 codec/runtime 성능을 검증하지 않았다.

## 10. 접근하지 못한 자료와 제외한 주장

- Qwen의 최신 2.1 blog는 본문을 읽을 수 없어 세부 출시 기능의 독립 근거로 쓰지 않았다.
- CVPR 2026 Qwen-Image-Layered 페이지/PDF 검색 결과는 있었으나 본문 요청이
  거절되어, 확인 가능한 원 arXiv 논문을 인용했다.
- Toon Boom timing/camera 설명은 검색됐지만 본문이 403 또는 조회 실패여서
  이번 결론의 핵심 근거로 쓰지 않았다.
- Rive changelog는 timeout으로 읽지 못했다. 읽은 공식 export 문서 범위만 사용했다.
- Reddit의 9월 Claude 사례는 탐색 단서로만 사용했다. 제작 방법이나 모델 우열의
  기술 근거는 공식 문서와 논문으로 한정했다.
- 공급자의 “production-ready”, “world's best”를 이 제품의 품질 보증으로 쓰지 않았다.

관련 결과: [움직이는 콘티 도입 명세](../ANIMATIC_DESIGN_SPEC.md).

# Image interview scenarios

These scenarios define the representative v4 route matrix.

| Case | Brief | Route | Expected blocking decision |
|---|---|---|---|
| C1-01 | 30대 가구 디자이너가 작업실에서 의자를 스케치하는 세로 화보 | C1 | specific wardrobe |
| C4-01 | 무광 알루미늄 스피커 1개와 재생지 패키지 1개 제품 사진 | C4 | product/material guide if absent |
| C4-02 | 개인 기억 앱 아이콘, 별 안의 접힌 종이 핵심 은유 | C4 + `app_icon` | product purpose, one centered metaphor, 32px silhouette |
| C5-01 | 시트러스 음료 출시 한국어 포스터, 캔 1개 | C5 | exact copy when requested; otherwise campaign plan |
| C6-01 | 커피 생산 6단계 인포그래픽 | C6 | exact six steps and reading order |
| C10-01 | 붉은 목도리 여우의 4패널 이야기 | C10 | four beats and identity anchor |
| C11-01 | 고대 수몰 도시에 도착한 탐험가 키아트 | C11 | world focus and scale |

## Routing regressions

| Case | Initial state | Action | Required result |
|---|---|---|---|
| STATE-01 | C1 wardrobe answered | switch to C5 | no wardrobe/face/hair/safety answer remains |
| STATE-02 | slow C1 analysis | send newer C5 analysis | late C1 response is ignored |
| STATE-03 | photo with LUT | switch to illustration | `photo_lut` is absent |
| STATE-04 | legacy style ID | analyze | one removal choice, no silent mapping |
| STATE-05 | invalid category | analyze | category is reasked before route-specific inference |
| STATE-06 | exact text requested | provide two lines | compiled prompt renders ordered quoted lines, never `exact_text_line_*` |

## Hard-constraint regressions

- C4 count: one product and one package.
- C4 app icon: one typed subject with count=1 at center, no text, opaque 1024×1024 PNG.
- C5 count: one hero object and the stated supporting-object count.
- C6 count: every stated data point or step appears once.
- C10 continuity: exact panel count and the same identity anchor in every panel.
- C11 focus: one protagonist unless the brief explicitly asks for more.
- Negative constraints do not become positive subject, medium, lighting, or text inference.

## Output defaults

| ID | Ratio | High quality reason |
|---|---|---|
| C1 | 2:3 | high only for exact text or explicit high-detail request |
| C4 | 3:2 | high only for exact text or explicit high-detail request |
| C4 + `app_icon` | 1:1 | product metaphor and 32px silhouette preservation |
| C5 | 2:3 | campaign hierarchy and possible exact text |
| C6 | 16:9 | dense information and reading order |
| C10 | 2:3 | multi-panel identity and layout |
| C11 | 16:9 | character/environment scale and atmosphere |

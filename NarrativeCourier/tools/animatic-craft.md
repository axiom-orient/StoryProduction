# 움직이는 콘티 연출 draft 작성

`animatic.py source`의 현재 승인 board와 원문 occurrence를 읽는다. 한 panel의
무엇을 볼지, 어느 순간 알아차릴지, 다음 컷으로 왜 이동할지를 먼저 결정한다.
모든 컷을 같은 길이로 채우거나 의미 없는 카메라 흔들림을 넣지 않는다.

`nvc.animatic-plan/1` draft에는 contentHash를 넣지 않는다. source·boardHash·
boardReviewHash는 source 응답 그대로 사용한다. source를 고친다면 STS owner의
수정 경로를 먼저 사용한다. plan이 원문·씬·대사를 새로 소유하지 않는다.

- fps는 12/24/30/60 중 명시적으로 선택한다. canvas는 짝수 width/height와
  불투명 RGB background를 갖는다. durationFrames는 각 컷의 실제 판단 값이다.
- 각 panel은 원 board 순서와 ID를 보존하고 camera, layers, captions를 갖는다.
- track은 frame 0과 durationFrames-1을 포함하고 중복 없이 증가한다.
  durationFrames=1이면 단일 keyframe이다. easing은 다음 keyframe까지 적용하는
  linear, easeInOut 또는 hold다. 마지막 keyframe의 easing은 다음 구간이 없다.
- camera keyframe: frame, x, y, scalePerMille, easing. x/y는 화면 중앙을 기준으로
  이동시키는 카메라 offset(px)이며 scale 1000은 1배다.
- layer: layerId, depthPerMille, poses, motion. 배열 순서가 뒤→앞이다. depth 1000은
  카메라 이동과 같은 offset, 400은 배경이 더 천천히 이동하는 깊이다. 카메라
  zoom은 모든 layer에 공통 적용된다.
- motion keyframe: frame, x, y, scalePerMille, rotationMilliDeg,
  opacityPerMille, easing. x/y는 image의 중심 위치(px), scale 1000은 원 크기,
  rotation 1000은 1도, opacity 1000은 완전 불투명이다.
- poses는 frame 0부터 `{frame, assetId}`를 선언해 필요할 때 그림을 바꾼다.
  등장/반응/시선 전환을 포즈 교체로 표현할 수 있다. 보간으로 얼굴을 변형하지 않는다.
- assets는 사용한 모든 assetId에 width, height, alpha(opaque/cutout), brief를
  작성한다. 사용하지 않은 asset과 빠진 asset은 거절한다. packet hash나 asset
  pack hash를 plan에 다시 넣지 않는다.
- captions는 panel.sourceRefs 안의 semanticId를 순서대로 선택한다. 렌더용
  문자열을 직접 다시 쓰지 않는다. 말풍선이나 번역문을 만들어 원문을 대체하지 않는다.
- 개별 대사를 시간별로 보여 줄 때는 plan의 `captionStyle`에 fontSize,
  maxLines:2, bandHeight, fadeFrames를 정하고 모든 panel에 `captionCues`를 둔다.
  각 cue는 cueId, semanticId, panel 내 startFrame/exclusive endFrame과 원문
  UTF-16 startUtf16/exclusive endUtf16을 선택한다. 대사 순서와 공백 외 모든
  원문 글자를 한 번씩 보존한다. 긴 대사는 문장·의미 단위로 나누고 grapheme을
  쪼개지 않는다. 서로 다른 cue 시간을 겹치지 않는다. 읽기 시간과 동작의
  선후를 검토하고, 하차 뒤 인사를 하차 전에 표시하지 않는다.
  bandHeight는 그림 아래에 예약된다. canvas 높이에 이 영역을 포함하고,
  그림의 중심·카메라는 남은 picture 높이를 기준으로 작성한다.

미술 brief에는 일관된 인물 특징·선·색·빛과 layer의 완성 크기, foreground의
투명 여백, 포즈별 고정 요소를 적는다. cutout은 독립된 실제 PNG로 준비한다.
재생을 상상해 hold→행동→반응→cut의 박자를 만든다. 카메라와 포즈가 같은 사건을
중복 과장하지 않도록 하고, 중요한 대사는 읽을 시간을 남긴다.

사용자 선택이 필요한 의미 변경은 openQuestions에 남긴다. agent가 reviewer
문자열을 만들어 승인한 척하지 않는다. 후보를 실제로 읽고 재생해 판단하는
미학적 검토는 schema/hash 검사와 다른 작업이다.

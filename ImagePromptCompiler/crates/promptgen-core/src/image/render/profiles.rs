use super::super::common::{PromptLanguage, heading, label, quote};
use super::super::model::{
    CinematicStoryboardSpec, ImageConsistencyDimension, ImageConsistencySpec, ImageProfile,
    ImagePromptRequest,
};
use super::visual;

pub(super) fn render_category_profile_contract(request: &ImagePromptRequest) -> String {
    let language = request.language;
    if request.profile == ImageProfile::AppIcon {
        return render_app_icon_profile_contract(request);
    }
    let (title_ko, title_en, directive_ko, directive_en) = match request.profile {
        ImageProfile::Standard => return String::new(),
        ImageProfile::LogoIdentity => (
            "로고 아이덴티티 프로필",
            "LOGO IDENTITY PROFILE",
            "불투명 PNG 콘셉트·브랜드 마크 방향으로만 렌더한다. 형태·비율·획·여백·색 역할과 사용 맥락을 명확히 하되, SVG·벡터·투명 최종 납품물이라고 주장하지 않는다.",
            "Render as an opaque PNG concept and brand-mark direction only. Clarify silhouette, proportion, stroke, spacing, color roles, and usage context; do not claim an SVG, vector, or transparent final asset.",
        ),
        ImageProfile::AppIcon => unreachable!("app_icon is rendered by its dedicated contract"),
        ImageProfile::AppWebUi => (
            "앱·웹 UI 프로필",
            "APP / WEB UI PROFILE",
            if request.text_elements.is_empty() {
                "앱·웹·에이전트 화면의 시각 구조, 탐색, 상태, 컴포넌트 관계와 반응형 안전 영역만 고정한다. 제목·라벨·데이터·차트용 타이포그래피 지시는 생략하고 읽을 수 있는 문자·숫자·의사문자를 만들지 않는다."
            } else {
                "앱 또는 웹 화면의 정보 구조, 상태, 그리드, 컴포넌트 경계, 읽기 순서와 반응형 안전 영역을 관찰 가능한 화면으로 고정한다."
            },
            if request.text_elements.is_empty() {
                "Lock only the visual structure, navigation, state, component relationships, and responsive safe areas of an app, web, or agent screen. Omit title, label, data, and chart typography directives; render no readable text, numerals, or pseudo-writing."
            } else {
                "Lock the app or web screen as an observable interface: information architecture, state, grid, component boundaries, reading order, and responsive safe areas."
            },
        ),
        ImageProfile::InformationDesign => (
            "정보 디자인 프로필",
            "INFORMATION DESIGN PROFILE",
            "정보 단위·위계·범례·단계·수치·연결선을 한 가지 읽기 순서로 조직하고, 장식이 데이터 의미를 가리지 않게 한다.",
            "Organize information units, hierarchy, legends, steps, values, and connectors into one reading order; decoration must not obscure data meaning.",
        ),
        ImageProfile::CharacterPose => (
            "캐릭터·자세 프로필",
            "CHARACTER / POSE PROFILE",
            "텍스트로 지정한 캐릭터 정체성, 관절 사슬, 무게 중심, 실루엣, 손·발·시선과 행동을 먼저 고정하고 과장된 해부학 오류를 금지한다.",
            "Prioritize the text-directed character identity, joint chain, weight, silhouette, hands, feet, gaze, and action; reject distorted anatomy and ambiguous contact.",
        ),
        ImageProfile::PoomsaePose => (
            "품새 자세 프로필",
            "POOMSAE POSE PROFILE",
            "품새 동작을 텍스트로 지시한 단일 인물 시안으로 렌더한다. 발 위치·체중·골반·몸통 회전·팔 각도·손 모양·시선과 도복 실루엣을 관찰 가능한 순서로 고정한다.",
            "Render a single-character, text-directed poomsae concept. Lock foot placement, weight, pelvis, torso rotation, arm angles, hand shape, gaze, and dobok silhouette in observable order.",
        ),
    };
    format!(
        "{}\n- {}: {}\n- {}: {}",
        heading(language, title_ko, title_en),
        label(language, "프로필", "Profile"),
        request.profile.as_str(),
        label(language, directive_ko, directive_en),
        visual(if language == PromptLanguage::Korean {
            directive_ko
        } else {
            directive_en
        })
    )
}

fn render_app_icon_profile_contract(request: &ImagePromptRequest) -> String {
    let language = request.language;
    let subject = request.subjects.first();
    let metaphor = subject
        .map(|value| visual(&value.description))
        .unwrap_or_else(|| label(language, "단일 typed subject", "one typed subject").to_owned());
    let geometry = subject
        .map(|value| {
            format!(
                "{}; {}; {}",
                visual(&value.pose),
                visual(&value.action),
                value
                    .appearance
                    .as_deref()
                    .map(visual)
                    .unwrap_or_else(|| label(language, "간결한 형태", "simple geometry").to_owned())
            )
        })
        .unwrap_or_else(|| label(language, "간결한 중심 기하", "simple core geometry").to_owned());
    let planes = if request.composition.depth_layers.is_empty() {
        label(
            language,
            "단순한 2–3개 시각 평면",
            "simple 2–3 visual planes",
        )
        .to_owned()
    } else {
        request
            .composition
            .depth_layers
            .iter()
            .map(|value| visual(value))
            .collect::<Vec<_>>()
            .join(" | ")
    };
    format!(
        "{}\n- {}: {}\n- {}: {}\n- {}: {}\n- {}: {}\n- {}: {}\n- {}: {}\n- {}: {}\n- {}: {}\n- {}: {}",
        heading(
            language,
            "앱 아이콘 마스터 콘셉트 계약",
            "APP ICON MASTER CONCEPT CONTRACT"
        ),
        label(language, "프로필", "Profile"),
        "app_icon",
        label(language, "제품 목적", "Product purpose"),
        visual(&request.use_case),
        label(language, "핵심 은유", "Core metaphor"),
        metaphor,
        label(language, "컴포넌트 기하", "Component geometry"),
        geometry,
        label(language, "중앙 구성", "Centered composition"),
        label(
            language,
            "하나의 지배적 중심 상징인 단일 typed subject를 정중앙에 배치하고 캔버스를 자연스럽게 채운다",
            "Place the single typed subject as one dominant symbol at the exact center and let the artwork naturally fill the square canvas",
        ),
        label(language, "시각 평면", "Visual planes"),
        planes,
        label(language, "32px 실루엣 검사", "32px silhouette check"),
        label(
            language,
            "32px로 축소해도 하나의 핵심 은유와 강한 실루엣이 즉시 판독되어야 한다",
            "At 32px, one core metaphor and a strong silhouette must remain immediately recognizable",
        ),
        label(language, "금지", "Prohibitions"),
        label(
            language,
            "텍스트·문자·숫자·의사문자·UI 스크린샷·디바이스 목업·둥근 사각형 마스크·앱 아이콘 프레임·워터마크를 렌더하지 않는다",
            "No text, letters, numerals, pseudo-writing, UI screenshot, device mockup, rounded-square mask, app-icon frame, or watermark",
        ),
        label(language, "납품 경계", "Delivery boundary"),
        label(
            language,
            "opaque 1024×1024 PNG 마스터 콘셉트 한 장만 약속한다. SVG·벡터·편집 가능한 레이어·플랫폼 패키지는 주장하지 않는다",
            "Promise one opaque 1024x1024 PNG master concept only; do not claim SVG, vector, editable layers, or a platform package",
        )
    )
}

pub(super) fn render_cinematic_storyboard_contract(
    storyboard: &CinematicStoryboardSpec,
    language: PromptLanguage,
) -> String {
    let panel_lines = storyboard
        .panels
        .iter()
        .enumerate()
        .map(|(index, panel)| {
            format!(
                "- panel_{} id={}; shot_size={}; camera_angle={}; camera_move={}; action={}; body_facing={}; gaze_target={}; emotional_beat={}",
                index + 1,
                quote(&panel.id),
                panel.shot_size,
                quote(&panel.camera_angle),
                quote(&panel.camera_move),
                quote(&panel.action),
                panel.body_facing,
                panel.gaze_target,
                quote(&panel.emotional_beat),
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let anchors = storyboard
        .identity_anchors
        .iter()
        .map(|anchor| quote(anchor))
        .collect::<Vec<_>>()
        .join(", ");
    let reading = match storyboard.screen_direction.as_str() {
        "left_to_right" => label(language, "좌→우", "left to right"),
        "right_to_left" => label(language, "우→좌", "right to left"),
        _ => storyboard.screen_direction.as_str(),
    };
    format!(
        "{}\n- {}: {}\n- {}: {}\n- {}: {}\n- {}: {}\n- {}: {}\n- {}: {}\n{}",
        heading(
            language,
            "텍스트 없는 시네마틱 스토리보드 계약",
            "TEXTLESS CINEMATIC STORYBOARD CONTRACT"
        ),
        label(language, "납품 형태", "Delivery"),
        label(
            language,
            "패널 테두리만 있는 하나의 완성된 스토리보드 시트; 패널 수·순서·간격은 layout과 정확히 일치",
            "one finished storyboard sheet with panel borders only; panel count, order, and gutters exactly match layout"
        ),
        label(language, "레이아웃", "Layout"),
        storyboard.layout,
        label(language, "읽기·화면 방향", "Reading and screen direction"),
        reading,
        label(language, "반복 정체성 앵커", "Recurring identity anchors"),
        anchors,
        label(language, "텍스트 금지", "Text prohibition"),
        label(
            language,
            "패널 번호·캡션·말풍선·대사·로고·워터마크·UI·의사문자를 절대 렌더하지 않는다.",
            "Never render panel numbers, captions, speech balloons, dialogue, logos, watermarks, UI, or pseudo-text."
        ),
        label(language, "연속성", "Continuity"),
        label(
            language,
            "같은 인물의 얼굴 비율·머리·의상·소품·팔레트 역할을 유지한다. 중립 샷 없이 화면 방향을 뒤집지 않으며, 다음 패널의 시선 대상은 앞 패널의 시선선과 공간적으로 연결한다.",
            "Keep the same character face proportions, hair, wardrobe, prop, and palette roles. Do not reverse screen direction without a neutral re-establishing shot; connect each following gaze target spatially to the prior eyeline."
        ),
        panel_lines,
    )
}

pub(super) fn render_consistency_contract(
    consistency: &ImageConsistencySpec,
    language: PromptLanguage,
) -> String {
    let dimensions = consistency
        .dimensions
        .iter()
        .map(|dimension| dimension.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let anchors = consistency
        .shared_anchors
        .iter()
        .map(|anchor| quote(anchor))
        .collect::<Vec<_>>()
        .join(", ");
    let mut lines = vec![
        format!(
            "- {}: {}",
            label(language, "일관성 축", "Consistency dimensions"),
            dimensions
        ),
        format!(
            "- {}: {}",
            label(language, "공유 앵커", "Shared anchors"),
            anchors
        ),
    ];
    if let Some(sequence) = &consistency.sequence {
        lines.push(format!(
            "- {}: {}",
            label(language, "순서", "Sequence"),
            sequence
                .steps
                .iter()
                .enumerate()
                .map(|(index, step)| format!("{}={}", index + 1, quote(step)))
                .collect::<Vec<_>>()
                .join(", ")
        ));
        lines.push(format!(
            "- {}: {}",
            label(language, "이전 결과 조건화", "Condition on previous output"),
            if sequence.condition_on_previous {
                label(language, "예", "yes")
            } else {
                label(language, "아니오", "no")
            }
        ));
    }
    for dimension in &consistency.dimensions {
        let (ko, en) = match dimension {
            ImageConsistencyDimension::MultiView => (
                "멀티뷰: 시점이 달라도 랜드마크 위치·비율·세계 좌표와 카메라 관계를 유지한다.",
                "Multi-view: keep landmark placement, scale, world coordinates, and the declared camera relationship stable across views.",
            ),
            ImageConsistencyDimension::Character => (
                "캐릭터: 얼굴 비율·머리·의상·실루엣·구별 특징과 팔레트 역할을 모든 결과에서 유지한다.",
                "Character: keep face proportions, hair, wardrobe, silhouette, distinguishing features, and palette roles stable in every output.",
            ),
            ImageConsistencyDimension::Temporal => (
                "시간: 선언한 순서대로 행동·감정·소품 상태가 인과적으로 이어지며 상태가 설명 없이 초기화되지 않는다.",
                "Temporal: make action, emotion, and prop state progress causally in the declared order; never reset state without an observable cause.",
            ),
            ImageConsistencyDimension::Semantic => (
                "의미: 객체 정체성·공간 관계·레이아웃 위계·행동 의미를 결과 사이에서 보존한다.",
                "Semantic: preserve object identity, spatial relations, layout hierarchy, and action meaning across outputs.",
            ),
        };
        lines.push(format!("- {}", label(language, ko, en)));
    }
    format!(
        "{}\n{}",
        heading(
            language,
            "멀티 이미지 일관성 계약",
            "MULTI-IMAGE CONSISTENCY CONTRACT"
        ),
        lines.join("\n")
    )
}

pub(super) fn render_pose_fidelity_contract(request: &ImagePromptRequest) -> String {
    let language = request.language;
    let pose_index = request
        .references
        .iter()
        .find(|reference| reference.role.as_str() == "pose")
        .map(|reference| reference.index)
        .unwrap_or(0);
    let identity_index = request
        .references
        .iter()
        .find(|reference| reference.role.as_str() == "subject")
        .map(|reference| reference.index)
        .unwrap_or(0);
    let lines = [
        format!(
            "- {}",
            label(
                language,
                &format!(
                    "역할을 섞지 않는다. image_{pose_index}는 전신 자세·관절 기하·체중·접지·몸통/머리 방향·시선·카메라·가림만 지배한다. image_{identity_index}는 캐릭터 정체성·얼굴·표정·체형·머리·의상·화풍만 지배한다."
                ),
                &format!(
                    "Keep reference roles disjoint. image_{pose_index} controls only full-body pose, joint geometry, weight, foot contact, torso/head facing, gaze, camera, and occlusion. image_{identity_index} controls only character identity, face, expression, proportions, hair, wardrobe, and rendering style."
                )
            )
        ),
        format!(
            "- {}",
            label(
                language,
                "자세 설명과 자세 원본의 실제 픽셀이 충돌하면 자세 원본의 보이는 기하가 최종 권위다. 캐릭터 시트의 기본 자세로 정규화하지 않는다.",
                "If the written pose description conflicts with visible geometry in the pose reference, the pose reference pixels are final authority. Do not normalize toward the character sheet's default pose."
            )
        ),
        format!(
            "- {}",
            label(
                language,
                "좌우 반전 금지. 인물 기준 왼쪽/오른쪽, 카메라에 가까운 팔다리, 보이는 신체 측면을 그대로 유지한다.",
                "Never mirror. Preserve character-local left/right, the limbs nearest the camera, and the visible side of the body."
            )
        ),
        format!(
            "- {}",
            label(
                language,
                "정확도 순서 1: 발 위치·접지·서기 폭과 깊이·체중 중심·골반/몸통 각도·전체 실루엣을 맞춘다.",
                "Fidelity gate 1: match foot placement and contact, stance width/depth, weight center, pelvis/torso angle, and the whole-body silhouette."
            )
        ),
        format!(
            "- {}",
            label(
                language,
                "정확도 순서 2: 어깨-팔꿈치-손목-손과 골반-무릎-발목-발의 관절 사슬, 손모양, 발끝 방향, 겹침과 원근 단축을 맞춘다.",
                "Fidelity gate 2: match shoulder-elbow-wrist-hand and hip-knee-ankle-foot chains, hand shape, toe direction, overlap, and foreshortening."
            )
        ),
        format!(
            "- {}",
            label(
                language,
                "정확도 순서 3: 몸이 바라보는 방향·머리 회전·눈동자 시선·카메라 각도를 맞춘다. 앞 단계가 맞아도 이 단계가 다르면 실패다.",
                "Fidelity gate 3: match body facing, head rotation, eye gaze, and camera angle. A result that passes the earlier gates but fails this gate is still incorrect."
            )
        ),
        format!(
            "- {}",
            label(
                language,
                "교정 시 실패한 가장 이른 단계 하나만 수정하고 이미 맞은 자세·디테일·정체성·출력 잠금은 유지한다.",
                "During correction, change only the earliest failing gate and preserve every already-correct pose, detail, identity, and output lock."
            )
        ),
    ];
    format!(
        "{}\n{}",
        heading(language, "자세 충실도 계약", "POSE FIDELITY CONTRACT"),
        lines.join("\n")
    )
}

'use strict';

const token = document.querySelector('meta[name="geultu-token"]').content;
const MAX_SAMPLES = 32;
const MAX_SAMPLE_BYTES = 2 * 1024 * 1024;
const MAX_CORPUS_BYTES = 20 * 1024 * 1024;
const WRITING_FOCUS_INSTRUCTIONS = {
  action: '원문에 이미 있는 인물의 행동을 선명하게 다듬으세요. 새로운 행동이나 사건은 더하지 마세요.',
  dialogue: '원문에 이미 있는 대사의 말투와 흐름을 살펴보세요. 대사를 새로 쓰거나 의미를 바꾸지 마세요.',
  description: '원문에 이미 있는 장면 묘사를 다듬으세요. 새로운 감각 정보나 사물을 추가하지 마세요.'
};
const state = {
  project: null,
  scene: null,
  voiceProfile: null,
  guide: null,
  guideBasis: null,
  comparison: null,
  comparisonBasis: null,
  pending: null,
  candidatePreparing: false,
  candidatePrepareReadback: null,
  candidatePrepareTerminal: null,
  writingFocus: '',
  metadataDirty: false,
  voiceInputsOpen: true,
  candidateOpen: false,
  libraryOpen: false,
  busy: false,
  navigationBusy: false,
  sceneActionBusy: false,
  sceneSaveInFlight: false,
  sceneSaveConflict: false,
  sceneSaveError: null,
  sceneSaveOutcome: null,
  approvalReadback: null,
  approvalUnknowns: []
};
const $ = id => document.getElementById(id);

function makeId(prefix, separator = '-') {
  if (globalThis.crypto?.randomUUID) return prefix + separator + globalThis.crypto.randomUUID();
  return prefix + separator + Date.now().toString(36) + '-' + Math.random().toString(36).slice(2, 10);
}

function commandId() {
  return makeId('command');
}

function bytesOf(text) {
  return new TextEncoder().encode(text).length;
}

async function sha256Hex(text) {
  const digest = await crypto.subtle.digest('SHA-256', new TextEncoder().encode(text));
  return [...new Uint8Array(digest)].map(byte => byte.toString(16).padStart(2, '0')).join('');
}

function formatBytes(bytes) {
  if (bytes < 1024) return String(bytes) + 'B';
  if (bytes < 1024 * 1024) return String(Math.ceil(bytes / 1024)) + 'KB';
  return (bytes / (1024 * 1024)).toFixed(1) + 'MiB';
}

async function call(name, args = {}) {
  const response = await fetch('/api/tools/call', {
    method: 'POST',
    headers: {'Content-Type': 'application/json', 'X-Geultu-Token': token},
    body: JSON.stringify({name, arguments: args})
  });
  const value = await response.json();
  if (!response.ok || !value.ok) {
    const error = new Error(value.error?.message || '요청을 처리하지 못했습니다 (' + response.status + ').');
    error.code = value.error?.code;
    throw error;
  }
  return value.result;
}

function friendlyError(error) {
  const message = error?.message || '잠시 후 다시 시도해 주세요.';
  if (message.includes('원고가 바뀌어 작업 지시를 다시 만들어야 합니다')) {
    return '작업 지시의 저장 원고 기준이 바뀌었습니다. 최신 원고를 확인한 뒤 작업 지시를 다시 복사해 주세요.';
  }
  if (message.includes('source content revision mismatch')) {
    return '저장 원고가 다른 곳에서 바뀌었습니다. 입력은 남아 있으니 최신 저장본을 확인한 뒤 다시 준비해 주세요.';
  }
  if (message.includes('project revision mismatch')) {
    return '작업이 다른 곳에서 바뀌었습니다. 편집 중인 원고는 그대로 남아 있습니다. 최신 저장본을 확인한 뒤 다시 시도해 주세요.';
  }
  if (message.includes('revision mismatch') || message.includes('source revision')) {
    return '다른 변경이 먼저 저장되었습니다. 편집 중인 원고는 그대로 남아 있습니다. 최신 저장본을 확인한 뒤 다시 시도해 주세요.';
  }
  if (message.includes('only pending candidates have a review subject')) {
    return '대기 중인 수정안을 찾지 못했습니다. 최신 원고를 다시 불러와 주세요.';
  }
  if (error?.code === 'invalid_input' && message.startsWith('scene_id')) {
    return '장면을 추가하지 못했습니다. 새로고침 후 다시 시도해 주세요.';
  }
  if (message.includes('not found')) return '요청한 작업이나 장면을 찾지 못했습니다. 새로고침해 주세요.';
  if (message.includes('exceeds')) return '글 샘플이 허용된 크기를 넘었습니다. 샘플을 나누어 다시 시도해 주세요.';
  if (message.includes('must not be empty') || message.includes('empty')) return '내용을 한 글자 이상 입력해 주세요.';
  if (message.includes('already exists')) return '같은 이름의 항목이 이미 있습니다.';
  return message
    .replace(/voice profile/gi, '글투')
    .replace(/voice sample/gi, '글 샘플')
    .replace(/project/gi, '작업')
    .replace(/scene/gi, '장면');
}

function status(message, error = false) {
  $('status').textContent = message;
  $('status').classList.toggle('error', error);
}

function sceneTextIsDirty() {
  return Boolean(state.scene) && $('text').value !== (state.scene.text || '');
}

function projectSceneReadIsCoherent(project, scene, projectId, sceneId) {
  const projectScene = project?.scenes?.[sceneId];
  return project?.id === projectId
    && scene?.id === sceneId
    && projectScene?.revision === scene.revision
    && projectScene?.text === scene.text;
}

function candidateTextIsDraft() {
  return !state.pending && $('candidate').value.length > 0;
}

function unsavedInputLabels() {
  const labels = [];
  if (sceneTextIsDirty()) labels.push('장면 원고');
  if (candidateTextIsDraft()) labels.push('제출하지 않은 수정안');
  if (state.metadataDirty) labels.push('작품 정보');
  return labels;
}

function updateSceneDraftStatus(announce = false) {
  const card = $('scene-card');
  const wasDirty = card.classList.contains('has-unsaved-draft');
  const dirty = sceneTextIsDirty();
  card.classList.toggle('has-unsaved-draft', dirty);
  if (state.sceneSaveConflict && !dirty) state.sceneSaveConflict = false;

  const saveState = $('save-state');
  let label = '저장됨';
  let stateClass = 'saved';
  if (state.sceneSaveInFlight) {
    label = '저장 중';
    stateClass = 'saving';
  } else if (state.candidatePrepareTerminal?.sourceConflictReadback) {
    label = '최신 원고 확인 필요';
    stateClass = 'conflict';
  } else if (state.approvalReadback) {
    label = '결정 확인 필요';
    stateClass = 'unknown';
  } else if (state.sceneSaveError === 'unknown') {
    label = '저장 결과 확인 필요';
    stateClass = 'unknown';
  } else if (state.sceneSaveConflict) {
    label = '저장 충돌';
    stateClass = 'conflict';
  } else if (state.sceneSaveError === 'failed') {
    label = '저장 실패';
    stateClass = 'error';
  } else if (dirty) {
    label = '저장되지 않음';
    stateClass = 'dirty';
  }
  saveState.textContent = label;
  saveState.className = 'save-state ' + stateClass;

  if (state.sceneSaveInFlight) {
    $('scene-help').textContent = '저장 요청 중입니다. 지금 입력하는 글은 요청한 원고와 별도로 남습니다.';
  } else if (state.candidatePrepareTerminal?.sourceConflictReadback) {
    $('scene-help').textContent = '저장 원고 기준이 달라 수정안을 준비하지 못했습니다. 최신 원고를 확인할 때까지 이 편집기는 잠겨 있고 수정안 입력은 남아 있습니다.';
  } else if (state.sceneSaveError === 'unknown') {
    $('scene-help').textContent = '저장 결과를 확인하지 못했습니다. 저장본을 다시 읽기 전에는 저장 상태를 확정할 수 없습니다.';
  } else if (state.sceneSaveConflict) {
    $('scene-help').textContent = '최신 저장본과 편집 중인 원고가 다릅니다. 아래 저장본을 확인한 뒤, 현재 원고 저장을 눌러 덮어쓸지 결정하세요.';
  } else if (state.sceneSaveError === 'failed') {
    $('scene-help').textContent = '원고를 저장하지 못했습니다. 입력한 글은 그대로 남아 있습니다. 연결을 확인한 뒤 다시 저장할 수 있습니다.';
  } else if (state.approvalReadback) {
    $('scene-help').textContent = '수정안 결정 결과를 다시 확인하는 동안 원고 편집을 잠갔습니다. 검토 카드의 저장된 결정 다시 확인을 눌러 주세요.';
  } else if (state.candidatePrepareReadback) {
    $('scene-help').textContent = '수정안 준비 결과를 다시 확인하는 동안 원고 편집과 이동을 잠갔습니다. 검토 영역에서 저장된 준비 상태를 다시 확인해 주세요.';
  } else if (dirty && state.pending) {
    $('scene-help').textContent = '저장되지 않은 원고 수정이 있습니다. 검토 중인 수정안을 먼저 결정한 뒤 원고를 저장할 수 있습니다.';
  } else if (dirty) {
    $('scene-help').textContent = '저장되지 않은 원고입니다. 작업 지시·검토·시각화·내보내기·수정안 심사는 마지막 저장본을 사용합니다.';
  } else {
    $('scene-help').textContent = '현재 저장본입니다. 수정한 뒤 저장하면 작업에 반영됩니다.';
  }

  if (announce && !state.sceneSaveInFlight && dirty !== wasDirty) {
    status(dirty
      ? '저장되지 않은 원고 수정이 있습니다. 현재 원고 저장을 누르면 반영됩니다.'
      : '편집 중인 원고가 저장된 내용과 같습니다.');
  }
  renderSceneConflict();
  return dirty;
}

function setCreationForm(formId, buttonId, open, focusId) {
  const form = $(formId);
  const button = $(buttonId);
  form.hidden = !open;
  button.setAttribute('aria-expanded', String(open));
  if (focusId) $(focusId).focus();
}

function pendingSourceMatches() {
  return Boolean(state.pending
    && !state.pending.stale
    && !state.approvalReadback?.latestObservation
    && state.project?.content_revision === state.pending.expectedSourceRevision
    && state.scene?.revision === state.pending.expectedSceneRevision
    && (state.scene?.text || '') === state.pending.sourceText);
}

function renderCandidateWorkspace() {
  const pending = Boolean(state.pending);
  const uncertain = Boolean(state.approvalReadback);
  const preparing = state.candidatePreparing;
  const prepareUnknown = Boolean(state.candidatePrepareReadback);
  const hasDraft = candidateTextIsDraft();
  const expanded = pending || uncertain || preparing || prepareUnknown || state.candidateOpen || hasDraft;
  const sourceMatches = pendingSourceMatches();
  const hasReadbackMismatch = Boolean(state.approvalReadback?.latestObservation);
  $('candidate-context').textContent = state.project && state.scene
    ? state.project.title + ' · ' + state.scene.title
    : '';
  $('candidate-content').hidden = !expanded;
  $('candidate-toggle').hidden = pending || uncertain || prepareUnknown || hasDraft;
  $('candidate-toggle').setAttribute('aria-expanded', String(expanded));
  $('candidate-toggle').textContent = state.candidateOpen ? '수정안 입력 닫기' : '수정안 붙여넣기';
  $('candidate-preparation-help').hidden = pending || uncertain || preparing || prepareUnknown;
  $('candidate-preparation-controls').hidden = pending || uncertain || preparing || prepareUnknown;
  $('candidate-prepare-progress').hidden = !preparing;
  $('candidate-prepare-outcome').hidden = !prepareUnknown;
  $('candidate-prepare-description').textContent = state.candidatePrepareReadback?.message
    || '준비 결과를 확인할 때까지 수정안 입력은 잠깁니다. 글은 선택·복사할 수 있으며, 확인된 뒤 다시 편집할 수 있습니다.';
  $('candidate-prepare-readback').disabled = state.busy || state.navigationBusy || state.sceneActionBusy;
  $('candidate').readOnly = Boolean(pending || uncertain || preparing || prepareUnknown);
  $('candidate-prepare-terminal').hidden = !state.candidatePrepareTerminal;
  if (state.candidatePrepareTerminal) {
    $('candidate-prepare-terminal').textContent = state.candidatePrepareTerminal.message;
  }
  $('candidate-pair').classList.toggle('has-source-preview', pending);
  $('candidate-source-view').hidden = !pending;
  if (pending) {
    $('candidate-source-context').textContent = sourceMatches
      ? '검토 기준 · 콘텐츠 revision ' + state.pending.expectedSourceRevision
        + ' · 장면 revision ' + state.pending.expectedSceneRevision
      : hasReadbackMismatch
        ? '요청 당시 원고 · 콘텐츠 revision ' + state.pending.expectedSourceRevision
          + ' · 장면 revision ' + state.pending.expectedSceneRevision
        : state.pending.sourceTextIsKnownBasis
          ? '수정안 준비 당시 원고 · 콘텐츠 revision ' + state.pending.expectedSourceRevision
            + ' · 장면 revision ' + state.pending.expectedSceneRevision
            + ' / 현재 저장본 · 콘텐츠 revision ' + state.project?.content_revision
            + ' · 장면 revision ' + state.scene?.revision
          : '현재 저장본 · 콘텐츠 revision ' + state.project?.content_revision
            + ' · 장면 revision ' + state.scene?.revision
            + ' / 수정안 출처 revision ' + state.pending.expectedSourceRevision;
    $('candidate-source-text').textContent = state.pending.sourceText;
    $('candidate-source-heading').textContent = sourceMatches
      ? '저장 원고 · 수정안의 출처'
      : hasReadbackMismatch
        ? '승인 요청 당시 출처 원고'
        : state.pending.sourceTextIsKnownBasis
          ? '수정안 준비 당시 원고'
          : '현재 저장 원고 · 수정안의 옛 기준을 불러올 수 없음';
    $('candidate-source-match').textContent = sourceMatches
      ? '출처 일치'
      : hasReadbackMismatch ? '요청 당시' : '출처 오래됨';
    $('candidate-source-match').className = 'save-state ' + (sourceMatches ? 'saved' : 'conflict');
    $('candidate-source-state').textContent = sourceMatches
      ? '이 원고가 수정안 작성의 기준입니다.'
      : hasReadbackMismatch
        ? '아래는 반영 요청을 보낼 때 화면에 표시했던 원고입니다. 최신 저장 상태는 위의 결과 확인 영역에서 다시 읽어 확인해 주세요.'
        : state.pending.sourceTextIsKnownBasis
          ? '아래는 수정안 검토를 준비할 때의 저장 원고입니다. 현재 저장 원고는 편집기에서 확인할 수 있습니다. 수정안은 아직 반영되지 않았으며 출처 revision이 달라 현재 원고에 적용할 수 없습니다.'
          : '아래는 현재 저장 원고입니다. 수정안이 만들어진 당시의 원문은 저장된 기록에서 확인할 수 없습니다. 수정안은 아직 반영되지 않았으며 출처 revision이 현재 원고와 맞지 않아 적용할 수 없습니다.';
    if (!uncertain) {
      $('approval-description').textContent = !sourceMatches
        ? '수정안의 출처가 된 원고가 바뀌어 반영할 수 없습니다. 현재 저장 원고와 수정안을 확인한 뒤, 기존 수정안을 반영하지 않거나 새 원고 기준으로 다시 준비해 주세요.'
        : sceneTextIsDirty()
          ? '편집기에 저장되지 않은 원고가 있습니다. 수정안은 아직 반영되지 않았습니다. 저장되지 않은 원고를 먼저 정리해 주세요.'
          : state.pending.restored
            ? '이 수정안은 아직 반영되지 않았습니다. 현재 원고와 수정안을 확인해 주세요.'
            : '현재 원고에는 아직 반영되지 않았습니다. 원고와 수정안을 확인한 뒤 반영 여부를 선택해 주세요.';
    }
  }
  $('candidate-voice-dependency').hidden = !hasDraft || Boolean(state.voiceProfile) || prepareUnknown;
  if (uncertain) {
    $('candidate-state').textContent = '결과 확인 필요 · 저장된 결정을 다시 확인해야 합니다.';
  } else if (preparing) {
    $('candidate-state').textContent = '수정안 검토를 준비하고 있습니다. 입력은 그대로 잠겨 있습니다.';
  } else if (prepareUnknown) {
    $('candidate-state').textContent = '준비 결과 확인 필요 · 저장된 후보와 검토 상태를 다시 읽어야 합니다.';
  } else if (state.candidatePrepareTerminal?.sourceConflictReadback) {
    $('candidate-state').textContent = '준비되지 않음 · 최신 저장 원고 확인이 필요합니다. 수정안 입력은 남아 있습니다.';
  } else if (state.candidatePrepareTerminal?.sourceConflict) {
    $('candidate-state').textContent = '준비되지 않음 · 최신 저장 원고를 열었습니다. 수정안 초안은 남아 있습니다.';
  } else if (state.candidatePrepareTerminal) {
    $('candidate-state').textContent = '저장된 준비 기록의 현재 상태를 확인했습니다.';
  } else if (pending && !sourceMatches) {
    $('candidate-state').textContent = (hasReadbackMismatch
      ? '결과 확인 필요 · 최신 저장 상태와 요청 당시 검토 대상이 다릅니다.'
      : '출처가 오래됨 · 수정안은 아직 저장된 대기 상태입니다. 현재 원고에 바로 반영할 수 없습니다.')
      + (state.approvalUnknowns.length ? ' 이전 결정 요청 결과도 미확인으로 남아 있습니다.' : '');
  } else if (pending && sceneTextIsDirty()) {
    $('candidate-state').textContent = '대기 중 · 현재 편집기에 저장되지 않은 원고가 있습니다.';
  } else if (pending) {
    $('candidate-state').textContent = '대기 중 · 장면 revision ' + state.pending.expectedSceneRevision + ' 기준, 아직 현재 원고에 반영되지 않았습니다.'
      + (state.approvalUnknowns.length ? ' 이전 결정 요청 결과는 별도로 미확인입니다.' : '');
  } else if (state.approvalUnknowns.length) {
    $('candidate-state').textContent = '이전 결정 요청 결과는 미확인입니다. 화면에는 최신 저장 원고와 작업이 표시되어 있습니다.';
  } else if (hasDraft) {
    $('candidate-state').textContent = '수정안 초안 · 저장된 원고는 바뀌지 않았습니다.';
  } else {
    $('candidate-state').textContent = '저장된 원고는 그대로 유지됩니다.';
  }
}

function revealPendingCandidate(focusHeading = false) {
  requestAnimationFrame(() => {
    $('candidate-card').scrollIntoView({block: 'start'});
    if (focusHeading) $('candidate-heading').focus({preventScroll: true});
  });
}

function renderSceneConflict() {
  let details = $('scene-conflict');
  if (!state.sceneSaveConflict || !state.scene) {
    details?.remove();
    return;
  }
  if (!details) {
    details = document.createElement('details');
    details.id = 'scene-conflict';
    details.className = 'small-note';
    const summary = document.createElement('summary');
    summary.textContent = '최신 저장 원고 비교';
    const explanation = document.createElement('p');
    explanation.textContent = '다른 저장본이 먼저 반영되었습니다. 현재 편집 내용은 그대로 두었습니다.';
    const savedText = document.createElement('pre');
    savedText.className = 'saved-scene-copy';
    const useSaved = document.createElement('button');
    useSaved.type = 'button';
    useSaved.className = 'button quiet';
    useSaved.textContent = '최신 저장 원고 사용';
    useSaved.addEventListener('click', () => {
      if (!window.confirm('현재 편집 중인 원고를 최신 저장 원고로 바꿉니다. 편집 중인 내용은 버려집니다. 계속할까요?')) return;
      $('text').value = state.scene.text;
      state.sceneSaveConflict = false;
      clearGuide();
      updateWordCount();
      renderButtons();
      status('최신 저장 원고를 열었습니다.');
    });
    details.append(summary, explanation, savedText, useSaved);
    $('scene-card').append(details);
  }
  details.querySelector('pre').textContent = state.scene.text;
}

function blockNavigation(action) {
  if (state.busy || state.navigationBusy || state.sceneActionBusy) {
    status('현재 작업이 끝난 뒤 다시 시도해 주세요.', true);
    return true;
  }
  if (state.candidatePrepareReadback) {
    status('수정안 준비 결과를 확인한 뒤 이동해 주세요. 입력은 그대로 보존되어 있습니다.', true);
    return true;
  }
  const labels = unsavedInputLabels();
  if (!labels.length) return false;
  const steps = [];
  if (sceneTextIsDirty()) steps.push('현재 원고 저장 버튼을 누르세요.');
  if (candidateTextIsDraft()) steps.push('수정안 검토를 준비하거나 수정안 입력창을 비우세요.');
  if (state.metadataDirty) steps.push('작품 정보를 저장하거나 변경을 취소하세요.');
  status(`저장되지 않은 입력(${labels.join('· ')})이 있습니다. ${steps.join(' ')} 그런 뒤 ${action}하세요.`, true);
  return true;
}

function requireSavedScene(action) {
  if (!state.scene) {
    status('먼저 장면을 선택해 주세요.', true);
    return false;
  }
  if (state.approvalReadback) {
    status('저장된 수정안 결정 상태를 확인한 뒤 다시 시도해 주세요.', true);
    return false;
  }
  if (state.candidatePrepareReadback) {
    status('수정안 준비 결과를 확인한 뒤 다시 시도해 주세요.', true);
    return false;
  }
  if (!sceneTextIsDirty()) return true;
  status(`저장되지 않은 원고 수정이 있습니다. ${action}은 마지막 저장본을 사용하므로, 현재 원고를 저장한 뒤 다시 시도해 주세요.`, true);
  return false;
}

function editorSnapshot() {
  return {
    projectId: state.project?.id || null,
    sceneId: state.scene?.id || null,
    text: $('text').value
  };
}

function editorSnapshotUnchanged(snapshot) {
  return (state.project?.id || null) === snapshot.projectId
    && (state.scene?.id || null) === snapshot.sceneId
    && $('text').value === snapshot.text
    && !sceneTextIsDirty()
    && !candidateTextIsDraft()
    && !state.metadataDirty;
}

function clearPendingReview() {
  state.pending = null;
  state.approvalReadback = null;
  $('approval-readback')?.remove();
  $('candidate').readOnly = false;
  $('approval-box').hidden = true;
  $('approval-heading').textContent = '수정안을 확인해 주세요.';
  $('approval-description').textContent = '현재 원고에는 아직 반영되지 않았습니다. 원고를 확인한 뒤 반영 여부를 선택해 주세요.';
  $('decision-actor').value = '';
  $('approval-subject').textContent = '';
}

function restorableReviews(project, sceneId) {
  const candidates = project.candidates || {};
  const scenes = project.scenes || {};
  return Object.values(project.changesets || {}).flatMap(changeset => {
    const candidate = candidates[changeset.candidate_id];
    if (changeset.status !== 'pending'
        || !candidate
        || candidate.status !== 'pending'
        || candidate.source_revision !== changeset.source_revision
        || changeset.operations?.length !== 1) return [];
    const operation = changeset.operations[0];
    const scene = scenes[operation.scene_id];
    if (operation.type !== 'replace_scene_text'
        || !scene
        || (sceneId && scene.id !== sceneId)
        || candidate.content !== operation.text) return [];
    return [{candidate, changeset, scene}];
  });
}

function showPendingReview(candidate, changeset, subject, expectedProjectRevision, restored, sourceText = state.scene?.text || '', sourceTextIsKnownBasis = false) {
  state.approvalReadback = null;
  $('approval-readback')?.remove();
  const operation = changeset.operations?.length === 1 ? changeset.operations[0] : null;
  const sourceIsCurrent = Boolean(operation
    && candidate.source_revision === state.project?.content_revision
    && changeset.source_revision === state.project?.content_revision
    && operation.scene_id === state.scene?.id
    && operation.expected_scene_revision === state.scene?.revision
    && projectSceneReadIsCoherent(state.project, state.scene, state.project?.id, state.scene?.id)
    && sourceText === (state.scene?.text || ''));
  state.pending = {
    candidateId: candidate.id,
    subject,
    changesetId: changeset.id,
    expectedProjectRevision,
    expectedSourceRevision: changeset.source_revision,
    expectedSceneRevision: operation?.expected_scene_revision,
    sourceText,
    sourceTextIsKnownBasis: sourceIsCurrent || sourceTextIsKnownBasis,
    stale: !sourceIsCurrent,
    restored
  };
  $('candidate').value = candidate.content;
  $('candidate').readOnly = true;
  $('approval-heading').textContent = state.pending.stale
    ? '이전 기준의 수정안입니다.'
    : restored ? '저장된 수정안을 다시 불러왔습니다.' : '수정안을 확인해 주세요.';
  $('approval-description').textContent = restored
    ? '이 수정안은 아직 반영되지 않았습니다. 현재 원고와 수정안을 확인해 주세요.'
    : '현재 원고에는 아직 반영되지 않았습니다. 원고를 확인한 뒤 반영 여부를 선택해 주세요.';
  $('decision-actor').value = '';
  $('approval-subject').textContent =
    subject.id + ' · revision ' + subject.revision + ' · ' + subject.digest;
  $('approval-box').hidden = false;
  renderButtons();
}

function renderApprovalReadbackAction() {
  const existing = $('approval-box').querySelector('#approval-readback');
  if (!state.approvalReadback || !state.pending) {
    existing?.remove();
    return;
  }
  if (existing) {
    existing.textContent = state.approvalReadback.latestObservation
      ? '최신 저장 상태 열기'
      : '저장된 결정 다시 확인';
    return;
  }
  const button = document.createElement('button');
  button.id = 'approval-readback';
  button.type = 'button';
  button.className = 'button secondary';
  button.textContent = state.approvalReadback.latestObservation
    ? '최신 저장 상태 열기'
    : '저장된 결정 다시 확인';
  button.addEventListener('click', () => state.approvalReadback?.latestObservation
    ? openLatestApprovalState()
    : rereadApprovalOutcome());
  $('approval-box').querySelector('.button-row').prepend(button);
}

function approvalObservation(project, scene, snapshot, subject = null) {
  const candidate = project.candidates?.[snapshot.candidateId];
  const changeset = project.changesets?.[snapshot.changesetId];
  const operation = changeset?.operations?.length === 1 ? changeset.operations[0] : null;
  return {
    projectId: project.id,
    projectRevision: project.revision,
    contentRevision: project.content_revision,
    sceneId: scene.id,
    sceneRevision: scene.revision,
    sourceTextMatches: (scene.text || '') === snapshot.sourceText,
    candidateStatus: candidate?.status || 'missing',
    candidateSourceRevision: candidate?.source_revision ?? null,
    candidateTextMatches: candidate?.content === snapshot.candidateText,
    candidateSubjectRead: subject ? 'read' : candidate?.status === 'pending' ? 'unavailable' : 'not_available',
    candidateSubject: subject ? {
      id: subject.id,
      revision: subject.revision,
      digest: subject.digest
    } : null,
    changesetStatus: changeset?.status || 'missing',
    changesetSourceRevision: changeset?.source_revision ?? null,
    operationSceneId: operation?.scene_id ?? null,
    operationSceneRevision: operation?.expected_scene_revision ?? null,
    operationTextMatches: operation?.text === snapshot.candidateText
  };
}

function approvalObservationText(snapshot, observed) {
  if (!observed) return '';
  const candidateStatus = observed.candidateStatus === 'missing' ? '찾지 못함' : observed.candidateStatus;
  const changesetStatus = observed.changesetStatus === 'missing' ? '찾지 못함' : observed.changesetStatus;
  const candidateTextState = observed.candidateStatus === 'missing'
    ? '없음'
    : observed.candidateTextMatches ? '같음' : '다름';
  return '현재 조회: 작업 콘텐츠 revision ' + observed.contentRevision
    + ' · 장면 revision ' + observed.sceneRevision
    + ' · 수정안 ' + candidateStatus
    + ' · 검토 ' + changesetStatus
    + ' · 원고 본문 ' + (observed.sourceTextMatches ? '요청 당시와 같음' : '요청 당시와 다름')
    + ' · 후보 본문 ' + candidateTextState
    + (observed.candidateSubject
      ? ' · 현재 subject revision ' + observed.candidateSubject.revision
        + ' · SHA-256 ' + observed.candidateSubject.digest
      : observed.candidateSubjectRead === 'unavailable' ? ' · 현재 subject를 읽지 못함' : '')
    + (observed.changesetSourceRevision !== null
      ? ' · 검토 출처 revision ' + observed.changesetSourceRevision : '')
    + (observed.operationSceneRevision !== null
      ? ' · 대상 장면 revision ' + observed.operationSceneRevision : '')
    + ' · 요청 당시 출처 revision ' + snapshot.sourceContentRevision
    + ' / 장면 revision ' + snapshot.sourceSceneRevision + '.';
}

function approvalIdentityText(snapshot) {
  return (snapshot.decisionKind === 'reject' ? '반영하지 않기 요청' : '반영 요청')
    + ' · ' + snapshot.candidateId + ' · 검토 ' + snapshot.changesetId
    + (snapshot.commandId ? ' · 요청 command ' + snapshot.commandId : ' · 승인 요청 전 대상 읽기 실패')
    + ' · 후보 subject revision ' + snapshot.subject.revision
    + ' · SHA-256 ' + snapshot.subject.digest
    + ' · 요청 당시 콘텐츠 revision ' + snapshot.sourceContentRevision
    + ' / 장면 revision ' + snapshot.sourceSceneRevision;
}

function renderApprovalOutcome() {
  const notice = $('approval-outcome-notice');
  const readback = state.approvalReadback;
  const priorUnknown = state.approvalUnknowns.at(-1);
  const entry = readback || priorUnknown;
  if (!entry) {
    notice.hidden = true;
    return;
  }

  notice.hidden = false;
  $('approval-outcome-heading').textContent = readback
    ? '결과 확인 필요'
    : '이전 결정 결과 미확인';
  $('approval-outcome-message').textContent = readback
    ? (readback.message || '결정 결과를 확인하지 못했습니다. 저장된 원고와 검토 대상을 다시 읽어 주세요.')
      + (state.approvalUnknowns.length ? ' 이전에 확인되지 않은 요청 ' + state.approvalUnknowns.length + '건도 아래에 보존되어 있습니다.' : '')
    : '이전 결정 요청의 결과는 아직 미확인입니다. 아래 최신 조회와 이전 요청 결과는 별도로 표시됩니다.';
  const observed = entry.latestObservation;
  $('approval-outcome-current').hidden = !observed;
  $('approval-outcome-current').textContent = approvalObservationText(entry, observed);
  $('approval-outcome-identity').textContent = approvalIdentityText(entry);
  const records = [...state.approvalUnknowns, ...(readback ? [readback] : [])];
  $('approval-outcome-history').replaceChildren(...records.map(record => {
    const item = document.createElement('li');
    item.textContent = approvalIdentityText(record)
      + (record.latestObservation ? ' · ' + approvalObservationText(record, record.latestObservation) : ' · 최신 상태를 아직 읽지 않음');
    return item;
  }));
  const action = $('approval-outcome-action');
  action.textContent = readback
    ? observed ? '최신 저장 상태 열기' : '저장된 결정 다시 확인'
    : '이전 요청 대상의 최신 상태 다시 읽기';
  action.disabled = state.busy || state.navigationBusy || state.sceneActionBusy;
}

async function observeApprovalReadback(snapshot, project, scene) {
  const candidate = project.candidates?.[snapshot.candidateId];
  let subject = null;
  if (candidate?.status === 'pending') {
    try {
      subject = await call('candidate.subject', {
        project_id: snapshot.projectId,
        candidate_id: snapshot.candidateId
      });
    } catch {
      // The latest project/scene remain authoritative; a transient subject read
      // failure must not be represented as an approval or a rejection.
    }
  }
  return approvalObservation(project, scene, snapshot, subject);
}

async function rememberMismatchedReadback(snapshot, project, scene, message, subject = null) {
  let observedSubject = subject;
  if (!observedSubject) {
    const candidate = project.candidates?.[snapshot.candidateId];
    if (candidate?.status === 'pending') {
      try {
        observedSubject = await call('candidate.subject', {
          project_id: snapshot.projectId,
          candidate_id: snapshot.candidateId
        });
      } catch {
        // Keep the owner status and the unavailable subject read distinct.
      }
    }
  }
  state.approvalReadback = {
    ...snapshot,
    message,
    latestObservation: approvalObservation(project, scene, snapshot, observedSubject)
  };
  $('approval-heading').textContent = '결정 상태를 다시 확인해 주세요.';
  $('approval-description').textContent = message;
  renderButtons();
  status('최신 저장 상태가 요청 당시 대상과 다릅니다. 이전 결과는 미확인으로 유지됩니다. 최신 저장 상태를 열어 확인해 주세요.', true);
}

async function readPendingReview(project, sceneId) {
  const matches = restorableReviews(project, sceneId);
  if (matches.length !== 1) return null;
  const {candidate, changeset, scene} = matches[0];
  const currentScene = await call('scene.get', {project_id: project.id, scene_id: scene.id});
  if (!projectSceneReadIsCoherent(project, currentScene, project.id, scene.id)) {
    throw new Error('최신 작업과 장면 조회가 서로 맞지 않습니다. 원고와 검토 상태는 그대로 유지합니다.');
  }
  const subject = await call('candidate.subject', {
    project_id: project.id,
    candidate_id: candidate.id
  });
  return {candidate, changeset, scene: currentScene, subject};
}

function setBusy(button, busy) {
  state.busy = busy;
  if (button) {
    button.disabled = busy;
    button.setAttribute('aria-busy', String(busy));
  }
  renderButtons();
}

function clearGuide() {
  state.guide = null;
  state.guideBasis = null;
  $('guide-result').hidden = true;
  $('guide-text').value = '';
}

function clearComparison() {
  state.comparison = null;
  state.comparisonBasis = null;
  $('comparison-result').hidden = true;
}

async function act(button, loadingMessage, task) {
  setBusy(button, true);
  status(loadingMessage);
  try {
    return await task();
  } catch (error) {
    status(friendlyError(error), true);
    return null;
  } finally {
    setBusy(button, false);
    renderButtons();
  }
}

function listButton(label, active, action, detail = '') {
  const node = document.createElement('button');
  node.type = 'button';
  node.className = 'item-button';
  node.disabled = state.busy || state.navigationBusy || Boolean(state.candidatePrepareReadback);
  const title = document.createElement('span');
  title.className = 'item-title';
  title.textContent = label;
  node.append(title);
  if (detail) {
    const note = document.createElement('span');
    note.className = 'item-detail';
    note.textContent = detail;
    node.append(note);
  }
  node.setAttribute('aria-current', active ? 'true' : 'false');
  node.addEventListener('click', action);
  return node;
}

async function loadProjects() {
  const projects = await call('project.list');
  $('projects').replaceChildren(...projects.map(project =>
    listButton(
      project.display_label || project.title,
      state.project?.id === project.id,
      () => selectProject(project.id),
      project.display_label ? project.title : (project.summary || '')
    )
  ));
  $('projects-empty').hidden = projects.length > 0;
  return projects;
}

async function selectProject(id) {
  if (state.candidatePrepareReadback) {
    status('수정안 준비 결과를 확인한 뒤 다른 작업으로 이동해 주세요.', true);
    return false;
  }
  if (state.approvalReadback) {
    status('저장된 수정안 결정 상태를 다시 확인한 뒤 다른 작업으로 이동해 주세요.', true);
    return false;
  }
  if (blockNavigation('다른 작업으로 이동')) return false;
  const snapshot = editorSnapshot();
  state.navigationBusy = true;
  renderButtons();
  try {
    const project = await call('project.get', {project_id: id});
    const voiceProfile = await call('voice.profile.get', {project_id: id});
    const restoredReview = await readPendingReview(project);
    if (!editorSnapshotUnchanged(snapshot)) {
      status('작업을 여는 동안 원고나 수정안이 바뀌었습니다. 입력을 저장하거나 비운 뒤 다시 시도해 주세요.', true);
      return false;
    }

    state.project = project;
    state.voiceProfile = voiceProfile;
    state.voiceInputsOpen = !voiceProfile;
    state.scene = restoredReview?.scene || null;
    state.libraryOpen = false;
    state.candidateOpen = false;
    state.candidatePrepareTerminal = null;
    state.metadataDirty = false;
    state.writingFocus = '';
    renderProjectMetadata(state.project);
    clearPendingReview();
    $('candidate').value = '';
    clearComparison();
    clearGuide();
    $('text').value = state.scene?.text || '';
    state.sceneSaveConflict = false;
    state.sceneSaveError = null;
    state.sceneSaveOutcome = null;
    if (restoredReview) {
      showPendingReview(
        restoredReview.candidate,
        restoredReview.changeset,
        restoredReview.subject,
        project.revision,
        true
      );
    }
    await loadScenes();
    renderProject();
    updateWordCount();
    if (restoredReview) revealPendingCandidate();
    status(restoredReview
      ? '저장된 수정안을 다시 불러왔습니다. 현재 원고와 비교해 보세요.'
      : state.project.title + ' 작업을 열었습니다.');
    return true;
  } catch (error) {
    status(friendlyError(error), true);
    return false;
  } finally {
    state.navigationBusy = false;
    renderButtons();
  }
}

async function loadScenes() {
  if (!state.project) return [];
  const scenes = await call('scene.list', {project_id: state.project.id});
  $('scenes').replaceChildren(...scenes.map(scene =>
    listButton(scene.title, state.scene?.id === scene.id, () => selectScene(scene.id))
  ));
  $('scene-count').textContent = scenes.length + '개';
  $('scenes-empty').hidden = scenes.length > 0;
  return scenes;
}

async function selectScene(id) {
  if (state.candidatePrepareReadback) {
    status('수정안 준비 결과를 확인한 뒤 다른 장면으로 이동해 주세요.', true);
    return false;
  }
  if (state.approvalReadback) {
    status('저장된 수정안 결정 상태를 다시 확인한 뒤 다른 장면으로 이동해 주세요.', true);
    return false;
  }
  if (blockNavigation('다른 장면으로 이동')) return false;
  if (!state.project) return false;
  const projectId = state.project.id;
  const snapshot = editorSnapshot();
  state.navigationBusy = true;
  renderButtons();
  try {
    const project = await call('project.get', {project_id: projectId});
    const voiceProfile = await call('voice.profile.get', {project_id: projectId});
    const scene = await call('scene.get', {project_id: projectId, scene_id: id});
    if (!projectSceneReadIsCoherent(project, scene, projectId, id)) {
      throw new Error('작업과 장면 조회가 서로 맞지 않습니다. 이전 화면을 그대로 유지합니다.');
    }
    const restoredReview = await readPendingReview(project, id);
    if (!editorSnapshotUnchanged(snapshot)) {
      status('장면을 여는 동안 원고나 수정안이 바뀌었습니다. 입력을 저장하거나 비운 뒤 다시 시도해 주세요.', true);
      return false;
    }

    state.project = project;
    state.voiceProfile = voiceProfile;
    state.scene = restoredReview?.scene || scene;
    state.libraryOpen = false;
    state.candidateOpen = false;
    state.candidatePrepareTerminal = null;
    state.metadataDirty = false;
    renderProjectMetadata(project);
    clearPendingReview();
    state.writingFocus = '';
    clearComparison();
    clearGuide();
    $('candidate').value = '';
    state.sceneSaveConflict = false;
    state.sceneSaveError = null;
    state.sceneSaveOutcome = null;
    $('text').value = state.scene.text;
    if (restoredReview) {
      showPendingReview(
        restoredReview.candidate,
        restoredReview.changeset,
        restoredReview.subject,
        project.revision,
        true
      );
    }
    await loadScenes();
    renderProject();
    updateWordCount();
    if (restoredReview) revealPendingCandidate();
    status(restoredReview
      ? '저장된 수정안을 다시 불러왔습니다. 아직 현재 원고에는 반영되지 않았습니다.'
      : state.scene.title + ' 장면을 열었습니다.');
    return true;
  } catch (error) {
    status(friendlyError(error), true);
    return false;
  } finally {
    state.navigationBusy = false;
    renderButtons();
  }
}

function renderProjectMetadata(project, force = false) {
  if (!project || (state.metadataDirty && !force)) return;
  $('metadata-title').value = project.title || '';
  $('metadata-display-label').value = project.display_label || '';
  $('metadata-summary').value = project.summary || '';
  state.metadataDirty = false;
  renderButtons();
}

function renderProject() {
  const hasProject = Boolean(state.project);
  $('empty-state').hidden = hasProject;
  $('project-view').hidden = !hasProject;
  $('scene-create-toggle').disabled = !hasProject || state.pending || state.approvalReadback;
  $('scene-create').hidden = !hasProject || $('scene-create-toggle').getAttribute('aria-expanded') !== 'true';
  $('library-content').classList.toggle('is-open', state.libraryOpen);
  $('library-toggle').setAttribute('aria-expanded', String(state.libraryOpen));
  $('library-toggle').textContent = hasProject
    ? '작업과 장면 · ' + (state.scene?.title || state.project.display_label || state.project.title)
    : '작업 만들기';
  if (!hasProject) return;

  $('project-heading').textContent = state.project.title;
  $('project-display-label-view').textContent = '현재 작업';
  $('project-summary-view').textContent = state.project.summary || '';
  $('project-summary-view').hidden = !state.project.summary;
  const sceneCount = Object.keys(state.project.scenes || {}).length;
  renderVoice();
  const hasScene = Boolean(state.scene);
  const hasSourceText = Boolean(state.scene?.text.trim());
  $('guide-focus-control').hidden = !hasScene || !hasSourceText || !state.voiceProfile;
  $('guide-copy-help').hidden = !state.voiceProfile || (hasScene && hasSourceText);
  $('guide-copy-help').textContent = hasScene
    ? '저장된 장면 원고가 있어야 작업 지시를 만들 수 있습니다.'
    : '장면을 선택하고 원고를 저장하면 작업 지시를 만들 수 있습니다.';
  $('guide-focus').value = state.writingFocus;
  $('scene-empty').hidden = hasScene;
  document.querySelector('#scene-empty h2').textContent = sceneCount
    ? '장면을 선택해 보세요.'
    : '장면을 하나 추가해 보세요.';
  document.querySelector('#scene-empty p').textContent = sceneCount
    ? '작업과 장면을 열어 원고를 이어 쓰거나, 새 장면을 만들 수 있습니다.'
    : '새 장면을 만든 뒤 여기에서 바로 쓸 수 있습니다.';
  $('empty-create-scene').textContent = sceneCount ? '새 장면 만들기' : '첫 장면 만들기';
  $('scene-card').hidden = !hasScene;
  $('candidate-card').hidden = !hasScene;
  if (hasScene) {
    $('scene-heading').textContent = state.scene.title;
  }
  renderButtons();
  loadProjects().catch(error => status(friendlyError(error), true));
}

function renderVoice() {
  const profile = state.voiceProfile;
  const ready = Boolean(profile);
  $('voice-state').textContent = ready ? '관찰값 있음' : '선택 기능 · 준비 전';
  $('voice-state').classList.toggle('ready', ready);
  $('voice-ready').hidden = !ready;
  $('voice-inputs').hidden = ready && !state.voiceInputsOpen;
  $('voice-intro').textContent = ready
    ? '관찰된 문장과 문단 흐름을 참고하는 값입니다. 원고 편집과 결정은 대신하지 않습니다.'
    : '직접 쓴 글이나 좋아하는 글을 붙여넣거나 TXT·MD 파일로 더하면, 수정 지시와 비교에 쓸 관찰값을 만듭니다. 원고 편집과 저장은 바로 할 수 있습니다.';
  if (!ready) return;
  $('voice-summary').textContent = profile.summary;
  $('voice-guidelines').replaceChildren(...profile.guidelines.map(guideline => {
    const item = document.createElement('li');
    item.textContent = guideline;
    return item;
  }));
  const measurements = profile.measurements;
  $('voice-measurements').textContent =
    profile.sample_count + '개 샘플 · ' + formatBytes(profile.corpus_bytes) +
    ' · 문장 평균 약 ' + Math.round(measurements.average_sentence_chars_x100 / 100) + '자';
  $('voice-rebuild-toggle').setAttribute('aria-expanded', String(state.voiceInputsOpen));
  $('voice-rebuild-toggle').textContent = state.voiceInputsOpen ? '샘플 입력 닫기' : '샘플 바꾸기';
}

function renderButtons() {
  const hasScene = Boolean(state.scene);
  const hasSourceText = Boolean(state.scene?.text.trim());
  const operationBusy = state.busy || state.navigationBusy || state.sceneActionBusy;
  const approvalReadback = Boolean(state.approvalReadback);
  const prepareReadback = Boolean(state.candidatePrepareReadback);
  const actionDisabled = operationBusy || approvalReadback || prepareReadback;
  const sourceConflictReadback = Boolean(state.candidatePrepareTerminal?.sourceConflictReadback);
  const sourceActionsDisabled = actionDisabled || sourceConflictReadback;
  $('refresh').textContent = sourceConflictReadback ? '최신 원고 확인' : '새로고침';
  const sourceDirty = updateSceneDraftStatus();
  $('text').readOnly = state.sceneActionBusy || approvalReadback || prepareReadback || sourceConflictReadback;
  $('text').setAttribute('aria-busy', String(state.sceneActionBusy));
  $('save').disabled = sourceActionsDisabled || !hasScene || !sourceDirty || Boolean(state.pending) || state.sceneSaveError === 'unknown';
  $('save').hidden = state.sceneSaveError === 'unknown';
  $('save').setAttribute('aria-busy', String(state.sceneSaveInFlight));
  $('save-readback').hidden = state.sceneSaveError !== 'unknown';
  $('save-readback').disabled = sourceActionsDisabled || !state.sceneSaveOutcome;
  $('guide-copy').disabled = sourceActionsDisabled || sourceDirty || state.metadataDirty || !state.voiceProfile || !hasScene || !hasSourceText;
  $('guide-focus').disabled = sourceActionsDisabled || sourceDirty || Boolean(state.pending) || !state.voiceProfile || !hasScene;
  $('metadata-save').disabled = sourceActionsDisabled || Boolean(state.pending) || !state.project || !state.metadataDirty;
  $('metadata-discard').disabled = sourceActionsDisabled || !state.project || !state.metadataDirty;
  $('prepare').disabled = sourceActionsDisabled || sourceDirty || Boolean(state.pending) || !hasScene || !$('candidate').value.trim();
  $('compare').disabled = sourceActionsDisabled || Boolean(state.pending) || !state.voiceProfile || !hasScene || !$('candidate').value.trim();
  $('approve').disabled = sourceActionsDisabled || sourceDirty || !state.pending || !pendingSourceMatches() || !$('decision-actor').value.trim();
  $('reject').disabled = sourceActionsDisabled || sourceDirty || !state.pending || !$('decision-actor').value.trim();
  $('voice-create').disabled = sourceActionsDisabled || Boolean(state.pending);
  $('review').disabled = sourceActionsDisabled || sourceDirty || !state.project;
  $('visual').disabled = sourceActionsDisabled || sourceDirty || !hasScene;
  $('export').disabled = sourceActionsDisabled || sourceDirty || !state.project;
  $('project-create-toggle').disabled = operationBusy || approvalReadback || prepareReadback || sourceConflictReadback;
  $('scene-create-toggle').disabled = operationBusy || approvalReadback || prepareReadback || sourceConflictReadback || !state.project || Boolean(state.pending);
  $('project-form').querySelector('button[type="submit"]').disabled = operationBusy || approvalReadback || prepareReadback || sourceConflictReadback;
  $('scene-form').querySelector('button[type="submit"]').disabled = operationBusy || approvalReadback || prepareReadback || sourceConflictReadback || Boolean(state.pending);
  $('refresh').disabled = operationBusy || approvalReadback || prepareReadback;
  renderApprovalReadbackAction();
  const recovery = $('approval-readback');
  if (recovery) recovery.disabled = operationBusy;
  $('candidate-prepare-readback').disabled = operationBusy;
  renderApprovalOutcome();
  $('projects').querySelectorAll('button').forEach(button => {
    button.disabled = operationBusy || approvalReadback || prepareReadback || sourceConflictReadback;
  });
  $('scenes').querySelectorAll('button').forEach(button => {
    button.disabled = operationBusy || approvalReadback || prepareReadback || sourceConflictReadback;
  });
  renderCandidateWorkspace();
}

function updateWordCount() {
  $('word-count').textContent = $('text').value.length.toLocaleString('ko-KR') + '자';
}

async function readSamples() {
  const samples = [];
  const pasted = $('sample-paste').value;
  if (pasted.trim()) samples.push({id: makeId('sample'), text: pasted});
  const files = [...$('sample-files').files];
  validateSampleFiles(files);
  if (samples.length + files.length > MAX_SAMPLES) {
    throw new Error('샘플은 최대 ' + MAX_SAMPLES + '개까지 준비할 수 있습니다.');
  }
  for (const file of files) {
    let text;
    try {
      text = new TextDecoder('utf-8', {fatal: true}).decode(await file.arrayBuffer());
    } catch {
      throw new Error('UTF-8로 읽을 수 없는 파일입니다: ' + file.name);
    }
    samples.push({id: makeId('sample'), text});
  }
  if (!samples.length) throw new Error('글 샘플을 한 개 이상 준비해 주세요.');
  let total = 0;
  for (const sample of samples) {
    const bytes = bytesOf(sample.text);
    if (bytes > MAX_SAMPLE_BYTES) throw new Error('한 글 샘플은 2MiB까지입니다.');
    total += bytes;
  }
  if (total > MAX_CORPUS_BYTES) throw new Error('전체 글 샘플은 20MiB까지입니다.');
  return samples;
}

function validateSampleFiles(files) {
  for (const file of files) {
    if (!/\.(?:txt|md)$/i.test(file.name)) {
      throw new Error('TXT 또는 MD 파일만 글 샘플로 가져올 수 있습니다.');
    }
  }
}

function handleSampleFilesChange() {
  try {
    validateSampleFiles([...$('sample-files').files]);
    updateSampleHint();
  } catch (error) {
    $('sample-files').value = '';
    updateSampleHint();
    status(friendlyError(error), true);
  }
}

function updateSampleHint() {
  const pasted = $('sample-paste').value;
  const files = [...$('sample-files').files];
  const count = (pasted.trim() ? 1 : 0) + files.length;
  const pastedBytes = pasted ? bytesOf(pasted) : 0;
  const fileBytes = files.reduce((total, file) => total + file.size, 0);
  const total = pastedBytes + fileBytes;
  $('sample-count').textContent = count
    ? count + '개 준비 · 약 ' + formatBytes(total)
    : '아직 준비한 샘플이 없습니다.';
  $('sample-count').classList.toggle('warning', count > MAX_SAMPLES || total > MAX_CORPUS_BYTES);
}

async function createVoiceProfile(event) {
  event.preventDefault();
  if (state.pending) {
    status('검토 중인 수정안을 먼저 결정한 뒤 글투를 바꿀 수 있습니다.', true);
    return;
  }
  await act($('voice-create'), '글 샘플을 관찰하고 있습니다…', async () => {
    const samples = await readSamples();
    state.project = await call('project.get', {project_id: state.project.id});
    await call('voice.profile.create', {
      project_id: state.project.id,
      profile_id: makeId('voice'),
      expected_project_revision: state.project.revision,
      samples,
      command_id: commandId()
    });
    $('sample-paste').value = '';
    $('sample-files').value = '';
    state.voiceInputsOpen = false;
    state.voiceProfile = await call('voice.profile.get', {project_id: state.project.id});
    clearGuide();
    clearComparison();
    renderProject();
      const hasScenes = Object.keys(state.project.scenes || {}).length > 0;
      status(hasScenes
        ? '글투 관찰을 저장했습니다. 장면을 열어 작업 지시를 만들 수 있습니다.'
        : '글투 관찰을 저장했습니다. 장면을 추가해 보세요.');
  });
}

async function copyText(text) {
  if (navigator.clipboard?.writeText) {
    await navigator.clipboard.writeText(text);
    return true;
  }
  $('guide-text').focus();
  $('guide-text').select();
  if (!document.execCommand('copy')) throw new Error('copy was not available');
  return true;
}

async function withSavedSceneAction(button, loadingMessage, task) {
  if (state.busy || state.navigationBusy || state.sceneActionBusy) {
    status('현재 작업이 끝난 뒤 다시 시도해 주세요.', true);
    return null;
  }
  if (state.approvalReadback) {
    status('저장된 수정안 결정 상태를 먼저 다시 확인해 주세요.', true);
    return null;
  }
  if (state.candidatePrepareReadback) {
    status('수정안 준비 결과를 다시 확인한 뒤 다른 작업을 시작해 주세요.', true);
    return null;
  }
  if (!requireSavedScene(loadingMessage)) return null;
  state.sceneActionBusy = true;
  renderButtons();
  try {
    return await act(button, loadingMessage, async () => {
      if (sceneTextIsDirty()) {
        throw new Error('저장되지 않은 원고 수정이 있습니다. 먼저 현재 원고를 저장해 주세요.');
      }
      return task();
    });
  } finally {
    state.sceneActionBusy = false;
    renderButtons();
  }
}

async function copyGuide(button) {
  await withSavedSceneAction(button, '작업 지시를 준비하고 있습니다…', async () => {
    if (state.metadataDirty) throw new Error('작품 정보를 저장하거나 취소한 뒤 작업 지시를 복사해 주세요.');
    const guide = await call('voice.guide', {
      project_id: state.project.id,
      scene_id: state.scene.id
    });
    const focusInstruction = WRITING_FOCUS_INSTRUCTIONS[state.writingFocus];
    const instruction = focusInstruction
      ? guide.instruction + '\n\n이번 수정에서 살펴볼 곳\n' + focusInstruction
      : guide.instruction;
    state.guide = {...guide, instruction};
    state.guideBasis = {
      project_id: guide.project_id,
      scene_id: guide.scene_id,
      source_project_revision: guide.source_project_revision,
      source_content_revision: guide.source_content_revision,
      source_scene_revision: guide.source_scene_revision,
      source_scene_digest: guide.source_scene_digest,
      writing_focus: state.writingFocus
    };
    $('guide-text').value = instruction;
    $('guide-result').hidden = false;
    try {
      await copyText(instruction);
      status('작업 지시를 복사했습니다.');
    } catch {
      status('작업 지시를 화면에 준비했습니다. 상자 안의 글을 직접 복사해 주세요.');
    }
  });
}

async function compareCandidate() {
  await act($('compare'), '수정안의 글투를 비교하고 있습니다…', async () => {
    const candidateText = $('candidate').value;
    const basis = {
      projectId: state.project.id,
      sceneId: state.scene.id,
      projectRevision: state.project.revision,
      sourceContentRevision: state.project.content_revision,
      sourceSceneRevision: state.scene.revision,
      sourceSceneDigest: await sha256Hex(state.scene.text || ''),
      writingFocus: state.writingFocus,
      voiceProfileFingerprint: JSON.stringify(state.voiceProfile),
      candidateText
    };
    const comparison = await call('voice.compare', {
      project_id: state.project.id,
      candidate_text: candidateText
    });
    if ($('candidate').value !== candidateText
        || state.project?.id !== basis.projectId
        || state.scene?.id !== basis.sceneId
        || state.writingFocus !== basis.writingFocus
        || JSON.stringify(state.voiceProfile) !== basis.voiceProfileFingerprint) {
      clearComparison();
      renderComparison();
      status('비교 중 수정안이나 기준이 바뀌어 이전 결과를 표시하지 않았습니다. 현재 수정안을 다시 비교해 주세요.', true);
      return;
    }
    state.comparison = comparison;
    state.comparisonBasis = basis;
    renderComparison();
    status('수정안의 글투를 비교했습니다. 팁을 확인해 보세요.');
  });
}

function renderComparison() {
  const result = state.comparison;
  if (!result) {
    $('comparison-result').hidden = true;
    return;
  }
  $('comparison-score').textContent = result.score + '점';
  $('comparison-label').textContent = result.label;
  $('comparison-summary').textContent = result.summary;
  $('comparison-tips').replaceChildren(...result.tips.map(tip => {
    const item = document.createElement('li');
    item.textContent = tip;
    return item;
  }));
  $('comparison-result').hidden = false;
}

function preparedCandidateMatches(candidate, snapshot) {
  return Boolean(candidate
    && candidate.id === snapshot.candidateId
    && candidate.kind === snapshot.kind
    && candidate.source_revision === snapshot.expectedContentRevision
    && candidate.content === snapshot.candidateText
    && (candidate.writing_type ?? null) === null
    && (candidate.stage ?? null) === null
    && candidate.receipt?.provider === snapshot.provider
    && candidate.receipt?.model === snapshot.model
    && (candidate.receipt?.request_id ?? null) === snapshot.requestId
    && (candidate.receipt?.input_hash ?? null) === snapshot.inputHash);
}

function preparedChangesetMatches(changeset, snapshot) {
  const operation = changeset?.operations?.length === 1 ? changeset.operations[0] : null;
  return Boolean(changeset
    && changeset.id === snapshot.changesetId
    && changeset.candidate_id === snapshot.candidateId
    && changeset.source_revision === snapshot.expectedContentRevision
    && operation?.type === 'replace_scene_text'
    && operation.scene_id === snapshot.sceneId
    && operation.expected_scene_revision === snapshot.expectedSceneRevision
    && operation.text === snapshot.candidateText);
}

function preparedPairLifecycle(candidate, changeset) {
  if (candidate?.status === 'pending' && changeset?.status === 'pending') return 'pending';
  if (candidate?.status === 'approved' && changeset?.status === 'approved') return 'terminal';
  if (candidate?.status === 'rejected'
      && ['pending', 'rejected'].includes(changeset?.status)) return 'terminal';
  if (candidate?.status === 'pending' && changeset?.status === 'rejected') return 'terminal';
  return null;
}

async function readPreparedCandidate(snapshot) {
  try {
    const [project, scene] = await Promise.all([
      call('project.get', {project_id: snapshot.projectId}),
      call('scene.get', {project_id: snapshot.projectId, scene_id: snapshot.sceneId})
    ]);
    if (!projectSceneReadIsCoherent(project, scene, snapshot.projectId, snapshot.sceneId)) return null;
    const commandRevision = project.applied_commands?.[snapshot.commandId];
    if (!Number.isInteger(commandRevision)
        || (snapshot.receiptRevision !== null && commandRevision !== snapshot.receiptRevision)) return null;
    const candidate = project.candidates?.[snapshot.candidateId];
    const changeset = project.changesets?.[snapshot.changesetId];
    if (!preparedCandidateMatches(candidate, snapshot)
        || !preparedChangesetMatches(changeset, snapshot)) return null;
    const lifecycle = preparedPairLifecycle(candidate, changeset);
    if (!lifecycle) return null;
    if (lifecycle === 'terminal') return {kind: lifecycle, project, scene, candidate, changeset};

    const subject = await call('candidate.subject', {
      project_id: snapshot.projectId,
      candidate_id: snapshot.candidateId
    });
    if (subject?.id !== 'candidate:' + snapshot.candidateId
        || subject.kind !== 'candidate'
        || subject.revision !== snapshot.expectedContentRevision + 1
        || !/^sha256:[0-9a-f]{64}$/.test(subject.digest || '')) return null;
    return {kind: lifecycle, project, scene, candidate, changeset, subject};
  } catch {
    return null;
  }
}

function keepCandidatePrepareUnknown(snapshot, message) {
  state.candidatePrepareReadback = {...snapshot, message};
  $('candidate').value = snapshot.candidateText;
  $('candidate').readOnly = true;
  renderButtons();
  status('수정안 준비 결과가 확인되지 않았습니다. 같은 요청을 자동으로 반복하지 않았습니다. 저장된 상태를 직접 다시 확인해 주세요.', true);
}

function candidateLifecycleLabel(status) {
  if (status === 'approved') return '승인됨';
  if (status === 'rejected') return '반영하지 않음';
  return '대기 중';
}

async function applyPreparedCandidateReadback(snapshot, recovered) {
  state.candidatePrepareReadback = null;
  state.candidatePreparing = false;
  state.project = recovered.project;
  state.scene = recovered.scene;
  state.candidateOpen = true;
  clearGuide();
  clearComparison();
  clearPendingReview();
  $('candidate').value = snapshot.candidateText;
  $('text').value = recovered.scene.text || '';
  state.sceneSaveConflict = false;
  state.sceneSaveError = null;
  state.sceneSaveOutcome = null;
  if (recovered.kind === 'pending') {
    state.candidatePrepareTerminal = null;
    showPendingReview(
      recovered.candidate,
      recovered.changeset,
      recovered.subject,
      recovered.project.revision,
      false,
      snapshot.sourceText,
      true
    );
    await loadScenes();
    renderProject();
    updateWordCount();
    const stale = !pendingSourceMatches();
    revealPendingCandidate(true);
    status(stale
      ? '준비 기록은 확인했습니다. 그 뒤 저장 원고가 바뀌어 수정안은 오래된 기준에 남아 있으며 현재 원고에 반영할 수 없습니다.'
      : '수정안 준비 기록을 확인했습니다. 현재 원고와 수정안을 함께 검토해 주세요.');
    return;
  }

  state.candidatePrepareTerminal = {
    message: '이 요청의 준비 기록을 확인했습니다. 저장된 상태는 후보 ' + candidateLifecycleLabel(recovered.candidate.status)
      + ' · 검토 ' + candidateLifecycleLabel(recovered.changeset.status)
      + '입니다. 최신 저장 원고를 열었고, 붙여넣은 수정안은 입력란에 보존했습니다. 이 결정은 다시 실행하지 않았습니다.'
  };
  $('candidate').readOnly = false;
  renderProjectMetadata(recovered.project);
  await loadScenes();
  renderProject();
  updateWordCount();
  status(state.candidatePrepareTerminal.message);
}

async function rereadCandidatePrepareOutcome() {
  const snapshot = state.candidatePrepareReadback;
  if (!snapshot || state.busy || state.navigationBusy || state.sceneActionBusy) return;
  state.sceneActionBusy = true;
  renderButtons();
  try {
    await act($('candidate-prepare-readback'), '저장된 후보 준비 상태를 다시 읽고 있습니다…', async () => {
      const recovered = await readPreparedCandidate(snapshot);
      if (!recovered) {
        state.candidatePrepareReadback = {
          ...snapshot,
          message: '현재 조회에서 요청 당시의 완전한 후보·검토 쌍과 저장 기록을 확인하지 못했습니다. 이것만으로 저장되지 않았다고 단정할 수 없습니다. 수정안 입력을 보존했으며 새 준비 요청은 보내지 않았습니다.'
        };
        renderButtons();
        status('준비 결과는 여전히 미확인입니다. 연결과 저장 상태를 확인한 뒤 이 버튼으로 다시 읽을 수 있습니다.', true);
        return;
      }
      await applyPreparedCandidateReadback(snapshot, recovered);
    });
  } finally {
    state.sceneActionBusy = false;
    renderButtons();
  }
}

function candidatePrepareConflictContextIsCurrent(snapshot) {
  return state.project?.id === snapshot.projectId
    && state.project?.content_revision === snapshot.sourceContentRevision
    && state.scene?.id === snapshot.sceneId
    && state.scene?.revision === snapshot.sourceSceneRevision
    && state.scene?.text === snapshot.sourceText
    && $('text').value === snapshot.sourceText
    && !sceneTextIsDirty();
}

async function latestCandidatePrepareConflictSource(snapshot) {
  try {
    const [project, scene] = await Promise.all([
      call('project.get', {project_id: snapshot.projectId}),
      call('scene.get', {project_id: snapshot.projectId, scene_id: snapshot.sceneId})
    ]);
    if (!projectSceneReadIsCoherent(project, scene, snapshot.projectId, snapshot.sceneId)) return null;
    return {project, scene};
  } catch {
    return null;
  }
}

async function openLatestCandidatePrepareConflictSource(snapshot) {
  const latest = await latestCandidatePrepareConflictSource(snapshot);
  if (!latest || !candidatePrepareConflictContextIsCurrent(snapshot)) return false;
  state.project = latest.project;
  state.scene = latest.scene;
  state.sceneSaveConflict = false;
  state.sceneSaveError = null;
  state.sceneSaveOutcome = null;
  clearGuide();
  clearComparison();
  $('text').value = latest.scene.text || '';
  state.candidatePrepareTerminal = {
    message: '저장 원고 기준이 달라 수정안을 준비하지 못했습니다. 최신 원고를 열었고 수정안은 남아 있습니다.',
    sourceConflict: true
  };
  renderProjectMetadata(latest.project);
  await loadScenes();
  renderProject();
  updateWordCount();
  return true;
}

async function rereadCandidatePrepareConflictSource() {
  const snapshot = state.candidatePrepareTerminal?.sourceConflictReadback;
  if (!snapshot || state.busy || state.navigationBusy || state.sceneActionBusy) return false;
  state.sceneActionBusy = true;
  renderButtons();
  try {
    await act($('refresh'), '최신 저장 원고를 확인하고 있습니다…', async () => {
      if (await openLatestCandidatePrepareConflictSource(snapshot)) {
        status(state.candidatePrepareTerminal.message, true);
        return true;
      }
      state.candidatePrepareTerminal = {
        ...state.candidatePrepareTerminal,
        sourceConflictReadback: snapshot,
        message: '저장 원고가 바뀌어 수정안을 준비하지 못했습니다. 최신 저장 원고는 아직 확인되지 않았습니다. 수정안은 남아 있으니 위의 최신 원고 확인을 다시 눌러 주세요.'
      };
      renderCandidateWorkspace();
      status(state.candidatePrepareTerminal.message, true);
      return false;
    });
  } finally {
    state.sceneActionBusy = false;
    renderButtons();
  }
  return true;
}

async function prepareCandidate() {
  await withSavedSceneAction($('prepare'), '수정안 검토를 준비하고 있습니다…', async () => {
    if (state.candidatePrepareReadback) throw new Error('저장된 준비 상태를 다시 확인한 뒤 새 준비를 시작할 수 있습니다.');
    if (!$('candidate').value.trim()) throw new Error('수정안을 붙여 넣어 주세요.');
    const candidateText = $('candidate').value;
    const projectId = state.project.id;
    const sceneId = state.scene.id;
    const writingFocus = state.writingFocus;
    const guideBasis = state.guideBasis ? {...state.guideBasis} : null;
    const comparisonBasis = state.comparisonBasis ? {...state.comparisonBasis} : null;
    state.candidatePrepareTerminal = null;
    state.candidatePreparing = true;
    $('candidate').readOnly = true;
    renderButtons();
    try {
      const project = await call('project.get', {project_id: projectId});
      const scene = await call('scene.get', {project_id: projectId, scene_id: sceneId});
      if (
        !projectSceneReadIsCoherent(project, scene, projectId, sceneId)
        || $('text').value !== scene.text
        || state.writingFocus !== writingFocus
      ) {
        throw new Error('작업이나 저장 원고가 바뀌었습니다. 최신 저장본을 확인한 뒤 다시 준비해 주세요.');
      }
      const sourceText = scene.text || '';
      const sourceDigest = await sha256Hex(sourceText);
      if (guideBasis && (
        guideBasis.project_id !== project.id
        || guideBasis.scene_id !== scene.id
        || guideBasis.source_project_revision !== project.revision
        || guideBasis.source_content_revision !== project.content_revision
        || guideBasis.source_scene_revision !== scene.revision
        || guideBasis.source_scene_digest !== sourceDigest
        || guideBasis.writing_focus !== writingFocus
      )) {
        clearGuide();
        clearComparison();
        throw new Error('저장 원고나 수정 초점이 바뀌어 이전 작업 지시를 사용할 수 없습니다. 현재 기준으로 다시 복사하거나 작업 지시 없이 한 번 더 준비해 주세요. 수정안은 그대로 남아 있습니다.');
      }

      let comparisonBecameStale = false;
      if (state.comparison) {
        const comparisonMatches = Boolean(comparisonBasis
          && comparisonBasis.projectId === project.id
          && comparisonBasis.sceneId === scene.id
          && comparisonBasis.projectRevision === project.revision
          && comparisonBasis.sourceContentRevision === project.content_revision
          && comparisonBasis.sourceSceneRevision === scene.revision
          && comparisonBasis.sourceSceneDigest === sourceDigest
          && comparisonBasis.writingFocus === writingFocus
          && comparisonBasis.voiceProfileFingerprint === JSON.stringify(project.voice_profile ?? null)
          && comparisonBasis.candidateText === candidateText);
        if (!comparisonMatches) {
          clearComparison();
          comparisonBecameStale = true;
        }
      }

      const snapshot = {
        projectId,
        sceneId,
        expectedContentRevision: project.content_revision,
        expectedSceneRevision: scene.revision,
        sourceText,
        candidateId: makeId('candidate'),
        changesetId: makeId('changeset'),
        commandId: commandId(),
        kind: 'style_aware_rewrite',
        candidateText,
        provider: 'external_editor',
        model: 'user_provided',
        requestId: null,
        inputHash: null,
        receiptRevision: null,
        message: '요청 결과를 확인할 때까지 수정안 입력은 잠깁니다. 글은 선택·복사할 수 있으며, 확인된 뒤 다시 편집할 수 있습니다.'
      };
      let receipt;
      try {
        receipt = await call('scene.prepare_candidate', {
          project_id: projectId,
          scene_id: sceneId,
          expected_content_revision: snapshot.expectedContentRevision,
          expected_scene_revision: snapshot.expectedSceneRevision,
          candidate_id: snapshot.candidateId,
          changeset_id: snapshot.changesetId,
          kind: snapshot.kind,
          content: candidateText,
          provider: snapshot.provider,
          model: snapshot.model,
          command_id: snapshot.commandId
        });
      } catch (error) {
        if (error?.code === 'conflict') {
          clearGuide();
          clearComparison();
          const conflictSnapshot = {
            projectId,
            sceneId,
            sourceText,
            sourceContentRevision: project.content_revision,
            sourceSceneRevision: scene.revision
          };
          if (await openLatestCandidatePrepareConflictSource(conflictSnapshot)) {
            status('저장 원고가 바뀌어 수정안을 준비하지 못했습니다. 최신 원고를 열었고 수정안은 남아 있습니다.', true);
          } else {
            state.candidatePrepareTerminal = {
              message: '저장 원고 기준이 달라 수정안을 준비하지 못했습니다. 최신 저장 원고는 아직 확인되지 않았습니다. 수정안은 남아 있으니 위의 최신 원고 확인을 눌러 주세요.',
              sourceConflict: true,
              sourceConflictReadback: conflictSnapshot
            };
            renderButtons();
            status(state.candidatePrepareTerminal.message, true);
          }
          return;
        }
        if (['invalid_input', 'not_found'].includes(error?.code)) {
          throw new Error(friendlyError(error));
        }
        keepCandidatePrepareUnknown(snapshot, '응답을 확인하지 못했습니다. 저장 여부를 다시 읽기 전에는 같은 요청을 반복하지 않습니다. 수정안 입력은 잠겨 있고 선택·복사할 수 있습니다.');
        return;
      }

      const receiptMatches = receipt?.project_id === projectId
        && Number.isInteger(receipt.revision)
        && receipt.event_count === 2
        && receipt.idempotent === false;
      snapshot.receiptRevision = receiptMatches ? receipt.revision : null;
      state.candidatePrepareReadback = snapshot;
      renderButtons();
      const recovered = await readPreparedCandidate(snapshot);
      if (!recovered) {
        keepCandidatePrepareUnknown(snapshot, '요청은 응답했지만 저장된 후보와 검토 쌍을 정확히 다시 읽지 못했습니다. 수정안 입력은 보존하며 같은 요청을 자동으로 반복하지 않습니다.');
        return;
      }
      state.project = project;
      state.scene = scene;
      await applyPreparedCandidateReadback(snapshot, recovered);
      status(comparisonBecameStale
        ? '오래된 글투 비교 결과는 숨겼습니다. 최신 저장 원고 기준으로 수정안을 준비했습니다.'
        : recovered.kind === 'pending'
          ? '수정안이 준비되었습니다. 아래에서 현재 원고와 함께 확인해 주세요.'
          : state.candidatePrepareTerminal.message);
      if (recovered.kind === 'pending') revealPendingCandidate(true);
    } finally {
      state.candidatePreparing = false;
      if (!state.pending && !state.candidatePrepareReadback) $('candidate').readOnly = false;
      renderButtons();
    }
  });
}

async function approveCandidate() {
  await withSavedSceneAction($('approve'), '수정안을 현재 원고에 반영하고 있습니다…', async () => {
    if (!state.pending) throw new Error('검토할 수정안이 없습니다.');
    const pending = state.pending;
    const projectId = state.project.id;
    const sceneId = state.scene.id;
    const sourceContentRevision = state.project.content_revision;
    const sourceSceneRevision = state.scene.revision;
    const sourceText = state.scene.text || '';
    const candidateText = $('candidate').value;
    const actorId = $('decision-actor').value.trim();
    if (!actorId) throw new Error('결정 기록에 사용할 검토자 ID를 입력해 주세요.');
    let decisionCommandId = null;
    const snapshot = {
      projectId,
      sourceContentRevision,
      sceneId,
      sourceSceneRevision,
      sourceText,
      candidateId: pending.candidateId,
      candidateText,
      changesetId: pending.changesetId,
      commandId: null,
      subject: {...pending.subject}
    };
    let currentSubject;
    try {
      currentSubject = await call('candidate.subject', {
        project_id: projectId,
        candidate_id: pending.candidateId
      });
    } catch {
      keepApprovalReadback(snapshot, '검토 대상의 최신 revision과 hash를 다시 읽지 못했습니다. 같은 검토 대상임을 확인하기 전에는 승인하지 않습니다.');
      return;
    }
    if (currentSubject.id !== pending.subject.id
        || currentSubject.revision !== pending.subject.revision
        || currentSubject.digest !== pending.subject.digest) {
      keepApprovalReadback(snapshot, '검토 대상 revision 또는 hash가 바뀌었습니다. 최신 저장 상태를 다시 읽기 전에는 승인하지 않습니다.');
      return;
    }
    snapshot.subject = {id: currentSubject.id, revision: currentSubject.revision, digest: currentSubject.digest};
    decisionCommandId = commandId();
    snapshot.commandId = decisionCommandId;
    try {
      await call('changeset.approve', {
        project_id: projectId,
        changeset_id: pending.changesetId,
        expected_project_revision: pending.expectedProjectRevision,
        actor_id: actorId,
        expected_artifact_revision: currentSubject.revision,
        expected_artifact_digest: currentSubject.digest,
        command_id: decisionCommandId
      });
    } catch {
      keepApprovalReadback(snapshot, '승인 요청 응답을 확인하지 못했습니다. 저장된 원고와 수정안 상태를 다시 읽고, 같은 검토 대상임을 확인하기 전에는 승인하지 않습니다.');
      return;
    }

    let project;
    let scene;
    try {
      [project, scene] = await Promise.all([
        call('project.get', {project_id: projectId}),
        call('scene.get', {project_id: projectId, scene_id: sceneId})
      ]);
    } catch {
      keepApprovalReadback(snapshot, '승인 요청은 응답했지만 최신 저장 원고를 다시 읽지 못했습니다. 결정 여부는 확인되지 않았으며, 다시 확인하기 전에는 승인하지 않습니다.');
      return;
    }
    const approvedCandidate = project.candidates?.[pending.candidateId];
    const approvedChangeset = project.changesets?.[pending.changesetId];
    const operation = approvedChangeset?.operations?.length === 1 ? approvedChangeset.operations[0] : null;
    if (!projectSceneReadIsCoherent(project, scene, projectId, sceneId)
        || approvedCandidate?.id !== pending.candidateId
        || approvedCandidate?.content !== candidateText
        || approvedCandidate?.source_revision !== sourceContentRevision
        || approvedChangeset?.id !== pending.changesetId
        || approvedChangeset?.candidate_id !== pending.candidateId
        || approvedChangeset?.source_revision !== sourceContentRevision
        || operation?.type !== 'replace_scene_text'
        || operation.scene_id !== sceneId
        || operation.expected_scene_revision !== sourceSceneRevision
        || operation.text !== candidateText) {
      keepApprovalReadback(snapshot, '다시 읽은 작업·장면·수정안이 요청 당시와 다릅니다. 저장된 결정 다시 확인으로 최신 상태를 확인해 주세요.');
      return;
    }
    if (approvedCandidate?.status !== 'approved' || approvedChangeset?.status !== 'approved') {
      keepApprovalReadback(snapshot, '다시 읽은 결정 상태가 아직 승인 완료와 일치하지 않습니다. 자동으로 다시 승인하지 않았습니다. 저장된 결정 다시 확인으로 상태를 다시 읽어 주세요.');
      return;
    }
    const appliedToCurrentScene = scene.text === candidateText;
    state.project = project;
    state.scene = scene;
    clearPendingReview();
    $('candidate').value = '';
    state.candidateOpen = false;
    clearComparison();
    clearGuide();
    state.sceneSaveConflict = false;
    $('text').value = state.scene.text;
    updateWordCount();
    renderProject();
    await loadScenes();
    status(appliedToCurrentScene
      ? '승인한 수정안이 현재 저장 원고에 반영된 것을 확인했습니다.'
      : '수정안 승인 기록은 저장되어 있습니다. 장면이 다시 바뀌어 최신 저장 원고를 열었습니다.');
  });
}

function keepApprovalReadback(snapshot, message) {
  state.approvalReadback = {...snapshot, message, latestObservation: null};
  $('approval-heading').textContent = '결정 상태를 다시 확인해 주세요.';
  $('approval-description').textContent = message;
  renderButtons();
  status('결정 결과가 확인되지 않았습니다. 승인이나 거절로 추측하지 않습니다. 저장된 상태를 다시 확인해 주세요.', true);
}

async function rereadApprovalOutcome() {
  const snapshot = state.approvalReadback;
  if (!snapshot || !state.pending) return;
  if (state.busy || state.navigationBusy || state.sceneActionBusy) {
    status('현재 작업이 끝난 뒤 다시 확인해 주세요.', true);
    return;
  }
  state.sceneActionBusy = true;
  renderButtons();
  try {
    await act($('approval-readback'), '저장된 결정 상태를 다시 확인하고 있습니다…', async () => {
      try {
        const [project, scene] = await Promise.all([
          call('project.get', {project_id: snapshot.projectId}),
          call('scene.get', {project_id: snapshot.projectId, scene_id: snapshot.sceneId})
        ]);
        const candidate = project.candidates?.[snapshot.candidateId];
        const changeset = project.changesets?.[snapshot.changesetId];
        const operation = changeset?.operations?.length === 1 ? changeset.operations[0] : null;
        const sameReview = projectSceneReadIsCoherent(
          project,
          scene,
          snapshot.projectId,
          snapshot.sceneId
        )
          && candidate?.id === snapshot.candidateId
          && candidate.content === snapshot.candidateText
          && candidate.source_revision === snapshot.sourceContentRevision
          && changeset?.id === snapshot.changesetId
          && changeset.candidate_id === snapshot.candidateId
          && changeset.source_revision === snapshot.sourceContentRevision
          && operation?.type === 'replace_scene_text'
          && operation.scene_id === snapshot.sceneId
          && operation.expected_scene_revision === snapshot.sourceSceneRevision
          && operation.text === snapshot.candidateText;
        if (!sameReview) {
          await rememberMismatchedReadback(
            snapshot,
            project,
            scene,
            '저장된 작업·장면·수정안이 요청 당시와 다릅니다. 이전 결정 결과는 미확인으로 유지하며 자동으로 다시 승인하지 않습니다.'
          );
          return;
        }

        if (candidate.status === 'approved' && changeset.status === 'approved') {
          if (snapshot.decisionKind === 'reject') {
            await rememberMismatchedReadback(
              snapshot,
              project,
              scene,
              '현재 저장된 수정안은 이미 반영된 상태입니다. 이 상태는 요청한 반영하지 않기와 다르므로 이전 요청 결과는 미확인으로 유지합니다.'
            );
            return;
          }
          state.project = project;
          state.scene = scene;
          clearPendingReview();
          $('candidate').value = '';
          state.candidateOpen = false;
          clearComparison();
          clearGuide();
          state.sceneSaveConflict = false;
          $('text').value = scene.text;
          updateWordCount();
          renderProject();
          await loadScenes();
          status(scene.text === snapshot.candidateText
            ? '승인한 수정안이 현재 저장 원고에 반영된 것을 확인했습니다.'
            : '수정안 승인 기록은 저장되어 있습니다. 장면이 다시 바뀌어 최신 저장 원고를 열었습니다.');
          return;
        }

        const recordedReject = Object.values(project.human_decisions || {}).some(decision =>
          decision.decision === 'REJECT'
          && decision.scope === 'project:' + snapshot.projectId + '/changeset:' + snapshot.changesetId
          && decision.subject?.id === snapshot.subject.id
          && decision.subject?.revision === snapshot.subject.revision
          && decision.subject?.digest === snapshot.subject.digest);
        if (changeset.status === 'rejected' && recordedReject) {
          state.project = project;
          state.scene = scene;
          clearPendingReview();
          $('candidate').value = '';
          state.candidateOpen = false;
          clearComparison();
          clearGuide();
          $('text').value = scene.text;
          updateWordCount();
          renderProject();
          await loadScenes();
          status(scene.text === snapshot.sourceText
            ? '이 수정안을 반영하지 않는 결정이 저장되어 있고 원고는 그대로입니다.'
            : '이 수정안을 반영하지 않는 결정이 저장되어 있습니다. 최신 원고는 별도로 다시 읽었습니다.');
          return;
        }

        const sourceUnchanged = candidate.status === 'pending'
          && changeset.status === 'pending'
          && project.content_revision === snapshot.sourceContentRevision
          && candidate.source_revision === snapshot.sourceContentRevision
          && scene.revision === snapshot.sourceSceneRevision
          && scene.text === snapshot.sourceText;
        if (sourceUnchanged) {
          const subject = await call('candidate.subject', {
            project_id: snapshot.projectId,
            candidate_id: snapshot.candidateId
          });
          if (subject.id === snapshot.subject.id
              && subject.revision === snapshot.subject.revision
              && subject.digest === snapshot.subject.digest) {
            const reviewer = $('decision-actor').value;
            state.project = project;
            state.scene = scene;
            showPendingReview(candidate, changeset, subject, project.revision, true, snapshot.sourceText, true);
            $('decision-actor').value = reviewer;
            renderProject();
            updateWordCount();
            status('같은 원고와 수정안 hash가 여전히 대기 중입니다. 직접 확인한 뒤 다시 결정할 수 있습니다.');
            return;
          }
          await rememberMismatchedReadback(
            snapshot,
            project,
            scene,
            '후보의 저장 subject가 요청 당시와 다릅니다. 이전 결정 결과는 미확인으로 유지하며 이 상태를 승인하지 않습니다.',
            subject
          );
          return;
        }
        await rememberMismatchedReadback(
          snapshot,
          project,
          scene,
          '저장된 원고 또는 수정안이 요청 당시 기준과 다릅니다. 이전 결정 결과는 미확인으로 유지하며 자동으로 승인하지 않습니다.'
        );
      } catch {
        status('저장된 결정 상태를 다시 읽지 못했습니다. 연결을 확인한 뒤 다시 확인해 주세요. 승인 요청은 반복되지 않았습니다.', true);
      }
    });
  } finally {
    state.sceneActionBusy = false;
    renderButtons();
  }
}

async function openLatestApprovalState() {
  const activeReadback = state.approvalReadback;
  const latestUnknown = state.approvalUnknowns.at(-1);
  const snapshot = activeReadback || latestUnknown;
  if (!snapshot) return;
  if (state.busy || state.navigationBusy || state.sceneActionBusy) {
    status('현재 작업이 끝난 뒤 최신 저장 상태를 열어 주세요.', true);
    return;
  }
  const before = editorSnapshot();
  state.sceneActionBusy = true;
  renderButtons();
  try {
    await act($('approval-outcome-action'), '최신 저장 원고와 수정안 상태를 읽고 있습니다…', async () => {
      const [project, scene] = await Promise.all([
        call('project.get', {project_id: snapshot.projectId}),
        call('scene.get', {project_id: snapshot.projectId, scene_id: snapshot.sceneId})
      ]);
      if (!projectSceneReadIsCoherent(project, scene, snapshot.projectId, snapshot.sceneId)) {
        throw new Error('최신 작업과 장면 조회가 서로 맞지 않습니다. 다시 읽기 전에는 편집 상태를 바꾸지 않았습니다.');
      }
      const latestReview = await readPendingReview(project, snapshot.sceneId);
      const currentScene = latestReview?.scene || scene;
      if (!projectSceneReadIsCoherent(project, currentScene, snapshot.projectId, snapshot.sceneId)) {
        throw new Error('최신 원고를 다시 읽는 동안 장면이 바뀌었습니다. 다시 확인해 주세요.');
      }
      if (!editorSnapshotUnchanged(before)) {
        status('최신 저장 상태를 읽는 동안 편집 입력이 바뀌었습니다. 입력을 보존하고 상태 전환을 멈췄습니다.', true);
        return;
      }

      let voiceProfile = null;
      try {
        voiceProfile = await call('voice.profile.get', {project_id: snapshot.projectId});
      } catch {
        // Voice is optional for opening the authoritative source state.
      }
      const observation = await observeApprovalReadback(snapshot, project, currentScene);
      if (state.approvalReadback) {
        state.approvalUnknowns.push({...state.approvalReadback, latestObservation: observation});
        state.approvalReadback = null;
      } else if (state.approvalUnknowns.length) {
        const last = state.approvalUnknowns.length - 1;
        state.approvalUnknowns[last] = {...state.approvalUnknowns[last], latestObservation: observation};
      }

      state.project = project;
      state.voiceProfile = voiceProfile;
      state.scene = currentScene;
      state.libraryOpen = false;
      state.candidateOpen = Boolean(latestReview);
      state.metadataDirty = false;
      state.writingFocus = '';
      renderProjectMetadata(project);
      clearPendingReview();
      $('candidate').value = '';
      clearComparison();
      clearGuide();
      state.sceneSaveConflict = false;
      state.sceneSaveError = null;
      state.sceneSaveOutcome = null;
      $('text').value = currentScene.text || '';
      if (latestReview) {
        showPendingReview(
          latestReview.candidate,
          latestReview.changeset,
          latestReview.subject,
          project.revision,
          true,
          currentScene.text || ''
        );
      }
      await loadScenes();
      renderProject();
      updateWordCount();
      if (latestReview) revealPendingCandidate();
      renderApprovalOutcome();
      status('최신 저장 상태를 열었습니다. 이전 결정 결과는 미확인으로 남아 있으며, 화면의 검토 대상은 저장된 내용에서 다시 읽었습니다.');
    });
  } finally {
    state.sceneActionBusy = false;
    renderButtons();
  }
}

async function rejectCandidate() {
  await withSavedSceneAction($('reject'), '수정안을 반영하지 않는 것으로 기록하고 있습니다…', async () => {
    if (!state.pending) throw new Error('검토할 수정안이 없습니다.');
    const pending = state.pending;
    const projectId = state.project.id;
    const sceneId = state.scene.id;
    const actorId = $('decision-actor').value.trim();
    if (!actorId) throw new Error('결정 기록에 사용할 검토자 ID를 입력해 주세요.');
    const snapshot = {
      decisionKind: 'reject',
      projectId,
      sourceContentRevision: pending.expectedSourceRevision,
      sceneId,
      sourceSceneRevision: pending.expectedSceneRevision,
      sourceText: pending.sourceText,
      candidateId: pending.candidateId,
      candidateText: $('candidate').value,
      changesetId: pending.changesetId,
      commandId: null,
      subject: {...pending.subject}
    };
    const currentSubject = await call('candidate.subject', {
      project_id: projectId,
      candidate_id: pending.candidateId
    });
    if (currentSubject.id !== pending.subject.id
        || currentSubject.revision !== pending.subject.revision
        || currentSubject.digest !== pending.subject.digest) {
      throw new Error('검토 대상이 바뀌었습니다. 최신 수정안을 다시 불러와 주세요.');
    }
    snapshot.subject = {id: currentSubject.id, revision: currentSubject.revision, digest: currentSubject.digest};
    const rejectCommandId = commandId();
    snapshot.commandId = rejectCommandId;
    let receipt;
    try {
      receipt = await call('changeset.reject', {
        project_id: projectId,
        changeset_id: pending.changesetId,
        actor_id: actorId,
        expected_artifact_revision: currentSubject.revision,
        expected_artifact_digest: currentSubject.digest,
        command_id: rejectCommandId
      });
    } catch {
      keepApprovalReadback(snapshot, '반영하지 않기 요청의 응답을 확인하지 못했습니다. 저장된 결정과 최신 원고를 다시 읽기 전에는 같은 요청을 반복하지 않습니다.');
      return;
    }
    const decision = receipt?.decision;
    const receiptMatches = receipt?.project_id === projectId
      && Number.isInteger(receipt?.revision)
      && receipt.revision >= state.project.revision
      && receipt.idempotent === false
      && decision?.decision === 'REJECT'
      && decision.actor_id === actorId
      && decision.scope === 'project:' + projectId + '/changeset:' + pending.changesetId
      && decision.subject?.id === currentSubject.id
      && decision.subject?.revision === currentSubject.revision
      && decision.subject?.digest === currentSubject.digest;
    if (!receiptMatches) {
      keepApprovalReadback(snapshot, '저장 응답이 요청한 수정안과 반영하지 않기 결정에 일치하지 않습니다. 최신 저장 상태를 다시 읽기 전에는 결정된 것으로 표시하지 않습니다.');
      return;
    }

    let project;
    let scene;
    try {
      [project, scene] = await Promise.all([
        call('project.get', {project_id: projectId}),
        call('scene.get', {project_id: projectId, scene_id: sceneId})
      ]);
    } catch {
      keepApprovalReadback(snapshot, '반영하지 않기 결정 응답을 받았지만 최신 원고를 다시 읽지 못했습니다. 결정 대상과 저장 상태를 다시 확인해 주세요.');
      return;
    }
    const rejectedCandidate = project.candidates?.[pending.candidateId];
    const rejectedChangeset = project.changesets?.[pending.changesetId];
    const operation = rejectedChangeset?.operations?.length === 1 ? rejectedChangeset.operations[0] : null;
    const recordedDecision = project.human_decisions?.[decision.decision_id];
    const readbackMatches = projectSceneReadIsCoherent(project, scene, projectId, sceneId)
      && project.revision >= receipt.revision
      && rejectedCandidate?.id === pending.candidateId
      && rejectedCandidate.status === 'pending'
      && rejectedCandidate.content === snapshot.candidateText
      && rejectedCandidate.source_revision === snapshot.sourceContentRevision
      && rejectedChangeset?.id === pending.changesetId
      && rejectedChangeset.status === 'rejected'
      && rejectedChangeset.candidate_id === pending.candidateId
      && rejectedChangeset.source_revision === snapshot.sourceContentRevision
      && operation?.type === 'replace_scene_text'
      && operation.scene_id === sceneId
      && operation.expected_scene_revision === snapshot.sourceSceneRevision
      && operation.text === snapshot.candidateText
      && recordedDecision?.decision_id === decision.decision_id
      && recordedDecision?.actor_id === actorId
      && recordedDecision?.decision === 'REJECT'
      && recordedDecision?.scope === decision.scope
      && recordedDecision?.subject?.id === currentSubject.id
      && recordedDecision?.subject?.revision === currentSubject.revision
      && recordedDecision?.subject?.digest === currentSubject.digest;
    if (!readbackMatches) {
      await rememberMismatchedReadback(
        snapshot,
        project,
        scene,
        '현재 저장된 결정이 반영하지 않기 요청 당시 대상과 일치하지 않습니다. 결과는 미확인으로 유지하며 최신 상태를 직접 확인해 주세요.',
        currentSubject
      );
      return;
    }

    const sourceStayedTheSame = scene.text === snapshot.sourceText;
    state.project = project;
    state.scene = scene;
    clearPendingReview();
    $('candidate').value = '';
    state.candidateOpen = false;
    clearComparison();
    clearGuide();
    $('text').value = scene.text || '';
    updateWordCount();
    renderProject();
    await loadScenes();
    status(sourceStayedTheSame
      ? '반영하지 않기 결정이 저장되었고 현재 원고는 그대로입니다.'
      : '반영하지 않기 결정이 저장되었습니다. 최신 원고에는 이후 저장된 변경도 반영되어 있습니다.');
  });
}

function renderReview(report) {
  const result = $('review-result');
  result.replaceChildren();
  const metrics = document.createElement('div');
  metrics.className = 'metric-grid';
  [
    ['글자 수', String(report.metrics.character_count.toLocaleString('ko-KR')) + '자'],
    ['문장 수', String(report.metrics.sentence_count.toLocaleString('ko-KR')) + '개'],
    ['문단 수', String(report.metrics.paragraph_count.toLocaleString('ko-KR')) + '개'],
    ['문장 평균', String(Math.round(report.metrics.average_sentence_length)) + '자']
  ].forEach(([label, value]) => {
    const item = document.createElement('div');
    item.className = 'metric';
    item.innerHTML = '<strong></strong><span></span>';
    item.querySelector('strong').textContent = value;
    item.querySelector('span').textContent = label;
    metrics.append(item);
  });
  result.append(metrics);
  const heading = document.createElement('h3');
  heading.textContent = report.findings.length ? '살펴볼 점' : '읽기 흐름이 안정적입니다';
  result.append(heading);
  if (report.findings.length) {
    const list = document.createElement('ul');
    list.className = 'finding-list';
    report.findings.forEach(finding => {
      const item = document.createElement('li');
      item.className = 'finding ' + String(finding.severity || '').toLowerCase();
      item.textContent = finding.message;
      list.append(item);
    });
    result.append(list);
  } else {
    const note = document.createElement('p');
    note.className = 'small-note';
    note.textContent = '현재 작업에서 눈에 띄는 경고가 없습니다.';
    result.append(note);
  }
  result.hidden = false;
}

async function runReview() {
  await withSavedSceneAction($('review'), '원고를 읽고 있습니다…', async () => {
    const report = await call('review.run', {project_id: state.project.id});
    renderReview(report);
    $('advanced-tools').open = true;
    status('읽기 흐름 검토를 마쳤습니다.');
  });
}

async function runVisualPlan() {
  await withSavedSceneAction($('visual'), '장면을 시각적으로 정리하고 있습니다…', async () => {
    const planId = makeId('visual');
    await call('visual.plan', {
      project_id: state.project.id,
      scene_id: state.scene.id,
      plan_id: planId,
      command_id: commandId()
    });
    const plans = await call('visual.list', {project_id: state.project.id});
    const plan = plans.find(item => item.id === planId);
    const result = $('review-result');
    result.replaceChildren();
    const heading = document.createElement('h3');
    heading.textContent = '장면 시각화 준비';
    result.append(heading);
    const note = document.createElement('p');
    note.className = 'small-note';
    note.textContent = '장면의 흐름을 바탕으로 외부 이미지 도구에 전달할 수 있는 쇼트 목록을 준비했습니다.';
    result.append(note);
    const list = document.createElement('ol');
    list.className = 'shot-list';
    plan.shots.forEach(shot => {
      const item = document.createElement('li');
      item.innerHTML = '<strong></strong><span></span>';
      item.querySelector('strong').textContent = shot.shot_type;
      item.querySelector('span').textContent = shot.description;
      list.append(item);
    });
    result.append(list);
    result.hidden = false;
    $('advanced-tools').open = true;
    status('장면 시각화 준비를 마쳤습니다.');
  });
}

async function exportProject() {
  await withSavedSceneAction($('export'), '원고 파일을 준비하고 있습니다…', async () => {
    const result = await call('project.export_markdown', {project_id: state.project.id});
    const blob = new Blob([result.markdown], {type: 'text/markdown;charset=utf-8'});
    const link = document.createElement('a');
    link.href = URL.createObjectURL(blob);
    link.download = state.project.title.replace(/[^\p{L}\p{N}]+/gu, '-').replace(/^-|-$/g, '') || '원고';
    link.download += '.md';
    link.click();
    URL.revokeObjectURL(link.href);
    status('원고 파일을 내려받았습니다.');
  });
}

async function reconcileFailedSceneSave(snapshot) {
  try {
    const [project, scene] = await Promise.all([
      call('project.get', {project_id: snapshot.projectId}),
      call('scene.get', {project_id: snapshot.projectId, scene_id: snapshot.sceneId})
    ]);
    if (!projectSceneReadIsCoherent(project, scene, snapshot.projectId, snapshot.sceneId)) return 'unknown';

    state.project = project;
    state.scene = scene;
    clearGuide();
    if (scene.text === snapshot.submittedText) {
      state.sceneSaveConflict = false;
      renderProjectMetadata(project);
      renderProject();
      await loadProjects();
      return 'confirmed';
    }
    if (scene.revision !== snapshot.expectedSceneRevision || scene.text !== snapshot.savedText) {
      state.sceneSaveConflict = true;
      renderProjectMetadata(project);
      renderProject();
      await loadProjects();
      return 'conflict';
    }
    return 'not-applied';
  } catch {
    return 'unknown';
  }
}

async function rereadSceneSaveOutcome() {
  const snapshot = state.sceneSaveOutcome;
  if (!snapshot || state.busy || state.navigationBusy || state.sceneActionBusy) return;
  await act($('save-readback'), '저장된 원고 상태를 다시 읽고 있습니다…', async () => {
    const result = await reconcileFailedSceneSave(snapshot);
    if (result === 'confirmed') {
      state.sceneSaveError = null;
      state.sceneSaveOutcome = null;
      status($('text').value === snapshot.submittedText
        ? '저장 상태를 다시 확인했습니다. 현재 원고가 저장되어 있습니다.'
        : '요청 당시 원고는 저장되어 있습니다. 이후 입력은 아직 저장되지 않았습니다.');
    } else if (result === 'conflict') {
      state.sceneSaveError = 'conflict';
      state.sceneSaveOutcome = null;
      status('다른 저장본이 먼저 반영되었습니다. 최신 저장 원고와 편집 중인 글을 비교해 주세요.', true);
    } else if (result === 'not-applied') {
      state.sceneSaveError = 'failed';
      state.sceneSaveOutcome = null;
      status('저장된 원고에서 요청한 내용을 찾지 못했습니다. 편집 내용은 그대로 남아 있습니다.', true);
    } else {
      state.sceneSaveError = 'unknown';
      state.sceneSaveOutcome = snapshot;
      status('저장 상태를 다시 확인하지 못했습니다. 입력은 그대로 남아 있습니다. 연결을 확인한 뒤 다시 확인해 주세요.', true);
    }
    await loadScenes();
    renderProject();
    updateWordCount();
  });
}

async function saveSceneText() {
  if (state.busy || state.navigationBusy || state.sceneActionBusy) {
    status('현재 작업이 끝난 뒤 다시 저장해 주세요.', true);
    return;
  }
  if (!state.scene || !sceneTextIsDirty()) return;
  if (state.sceneSaveError === 'unknown') {
    status('저장 결과를 다시 읽어 확인한 뒤 새 저장을 요청해 주세요.', true);
    return;
  }
  if (state.pending) {
    status('검토 중인 수정안을 먼저 결정한 뒤 원고를 저장할 수 있습니다.', true);
    return;
  }
  if (state.sceneSaveConflict && !window.confirm('현재 원고를 저장하면 아래에 표시된 최신 저장 원고를 교체합니다. 계속할까요?')) return;

  const snapshot = {
    projectId: state.project.id,
    sceneId: state.scene.id,
    expectedSceneRevision: state.scene.revision,
    savedText: state.scene.text || '',
    submittedText: $('text').value
  };
  state.sceneSaveError = null;
  state.sceneSaveOutcome = snapshot;
  await act($('save'), '현재 원고를 저장하고 있습니다…', async () => {
    state.sceneSaveInFlight = true;
    updateSceneDraftStatus();
    renderButtons();
    try {
      if (state.project?.id !== snapshot.projectId
          || state.scene?.id !== snapshot.sceneId
          || state.scene.revision !== snapshot.expectedSceneRevision) {
        throw new Error('다른 변경이 먼저 저장되었습니다.');
      }
      await call('scene.set_text', {
        project_id: snapshot.projectId,
        scene_id: snapshot.sceneId,
        text: snapshot.submittedText,
        expected_scene_revision: snapshot.expectedSceneRevision,
        command_id: commandId()
      });
      const recovery = await reconcileFailedSceneSave(snapshot);
      if (recovery === 'confirmed') {
        state.sceneSaveError = null;
        state.sceneSaveOutcome = null;
        status($('text').value === snapshot.submittedText
          ? '현재 원고를 저장했습니다.'
          : '저장 요청 당시의 원고는 저장했습니다. 이후 입력은 아직 저장되지 않았습니다.');
      } else if (recovery === 'conflict') {
        state.sceneSaveError = 'conflict';
        state.sceneSaveOutcome = null;
        status('다른 저장본이 먼저 반영되었습니다. 현재 편집 내용은 그대로 두었습니다. 최신 저장 원고를 비교한 뒤 저장 여부를 결정하세요.', true);
      } else if (recovery === 'not-applied') {
        state.sceneSaveError = 'failed';
        state.sceneSaveOutcome = null;
        status('저장 응답은 받았지만 현재 저장 원고에서 요청한 내용을 확인하지 못했습니다. 편집 내용은 그대로 남아 있습니다.', true);
      } else {
        state.sceneSaveError = 'unknown';
        status('저장 응답을 받았지만 결과를 확인하지 못했습니다. 입력한 내용은 그대로 남아 있습니다. 연결을 확인한 뒤 저장본을 다시 확인해 주세요.', true);
      }
    } catch (error) {
      const recovery = await reconcileFailedSceneSave(snapshot);
      if (recovery === 'confirmed') {
        state.sceneSaveError = null;
        state.sceneSaveOutcome = null;
        status($('text').value === snapshot.submittedText
          ? '저장 응답을 다시 확인했습니다. 현재 원고가 저장되어 있습니다.'
          : '저장 요청 당시의 원고는 저장되어 있습니다. 이후 입력은 아직 저장되지 않았습니다.');
      } else if (recovery === 'conflict') {
        state.sceneSaveError = 'conflict';
        state.sceneSaveOutcome = null;
        status('다른 저장본이 먼저 반영되었습니다. 현재 편집 내용은 그대로 두었습니다. 최신 저장 원고를 비교한 뒤 저장 여부를 결정하세요.', true);
      } else if (recovery === 'not-applied') {
        state.sceneSaveError = 'failed';
        state.sceneSaveOutcome = null;
        status(`원고를 저장하지 못했습니다. 입력한 내용은 그대로 남아 있습니다. ${friendlyError(error)}`, true);
      } else {
        state.sceneSaveError = 'unknown';
        status('저장 결과를 확인하지 못했습니다. 입력한 내용은 편집기에 그대로 남아 있습니다. 연결을 확인한 뒤 다시 저장해 주세요.', true);
      }
    } finally {
      state.sceneSaveInFlight = false;
      updateSceneDraftStatus();
      renderButtons();
    }
  });
}

$('project-form').addEventListener('submit', async event => {
  event.preventDefault();
  if (blockNavigation('새 작업 생성을 계속')) return;
  let createdProjectId = null;
  await act(event.submitter, '새 작업을 만들고 있습니다…', async () => {
    const title = $('project-title').value.trim();
    if (!title) throw new Error('작품 제목을 입력해 주세요.');
    const projectId = makeId('project');
    await call('project.create', {
      project_id: projectId,
      title,
      display_label: $('project-display-label').value.trim() || undefined,
      summary: $('project-summary').value.trim() || undefined,
      command_id: commandId()
    });
    createdProjectId = projectId;
    event.target.reset();
    setCreationForm('project-form', 'project-create-toggle', false);
    await loadProjects();
  });
  if (createdProjectId) await selectProject(createdProjectId);
});

$('metadata-form').addEventListener('submit', async event => {
  event.preventDefault();
  if (sceneTextIsDirty() || candidateTextIsDraft() || state.pending) {
    if (state.pending && !sceneTextIsDirty() && !candidateTextIsDraft()) {
      status('검토 중인 수정안을 먼저 결정한 뒤 작품 정보를 저장할 수 있습니다.', true);
    } else {
      blockNavigation('작품 정보 저장을 계속');
    }
    return;
  }
  await act($('metadata-save'), '작품 정보를 저장하고 있습니다…', async () => {
    if (!state.project) throw new Error('먼저 작업을 선택해 주세요.');
    const projectId = state.project.id;
    const values = {
      title: $('metadata-title').value.trim(),
      display_label: $('metadata-display-label').value.trim(),
      summary: $('metadata-summary').value.trim()
    };
    const updatedProject = await call('project.update_metadata', {
      project_id: projectId,
      ...values,
      expected_project_revision: state.project.revision,
      command_id: commandId()
    });
    state.project = updatedProject;
    if (state.scene) state.scene = updatedProject.scenes[state.scene.id];
    const formUnchanged = $('metadata-title').value.trim() === values.title
      && $('metadata-display-label').value.trim() === values.display_label
      && $('metadata-summary').value.trim() === values.summary;
    state.metadataDirty = !formUnchanged;
    if (formUnchanged) renderProjectMetadata(updatedProject, true);
    renderProject();
    await loadProjects();
    status(formUnchanged
      ? '작품 정보를 저장했습니다.'
      : '요청한 작품 정보는 저장했습니다. 이후 입력은 아직 저장되지 않았습니다.');
  });
});

$('metadata-form').addEventListener('input', () => {
  state.metadataDirty = true;
  clearGuide();
  clearComparison();
  renderButtons();
});
$('metadata-discard').addEventListener('click', () => {
  renderProjectMetadata(state.project, true);
  status('저장하지 않은 작품 정보 변경을 취소했습니다.');
});

$('scene-form').addEventListener('submit', async event => {
  event.preventDefault();
  if (state.pending) {
    status('검토 중인 수정안을 먼저 결정한 뒤 장면을 추가할 수 있습니다.', true);
    return;
  }
  if (blockNavigation('장면 추가를 계속')) return;
  let createdSceneId = null;
  await act(event.submitter, '장면을 추가하고 있습니다…', async () => {
    if (!state.project) throw new Error('먼저 작업을 선택해 주세요.');
    const title = $('scene-title').value.trim();
    if (!title) throw new Error('장면 제목을 입력해 주세요.');
    const sceneId = makeId('scene', '_');
    createdSceneId = sceneId;
    await call('scene.add', {
      project_id: state.project.id,
      scene_id: sceneId,
      title,
      text: '',
      order: Object.keys(state.project.scenes).length,
      command_id: commandId()
    });
    event.target.reset();
    setCreationForm('scene-create', 'scene-create-toggle', false);
    await loadScenes();
  });
  if (createdSceneId) await selectScene(createdSceneId);
});

$('voice-form').addEventListener('submit', createVoiceProfile);
$('sample-paste').addEventListener('input', updateSampleHint);
$('sample-files').addEventListener('change', handleSampleFilesChange);
$('voice-rebuild-toggle').addEventListener('click', () => {
  state.voiceInputsOpen = !state.voiceInputsOpen;
  renderVoice();
});
$('project-create-toggle').addEventListener('click', () => {
  const open = $('project-form').hidden;
  setCreationForm('project-form', 'project-create-toggle', open, open ? 'project-title' : null);
});
$('empty-create-project').addEventListener('click', () => {
  if (window.matchMedia('(max-width: 820px)').matches) {
    state.libraryOpen = true;
    $('library-content').classList.add('is-open');
    $('library-toggle').setAttribute('aria-expanded', 'true');
  }
  setCreationForm('project-form', 'project-create-toggle', true, 'project-title');
});
$('scene-create-toggle').addEventListener('click', () => {
  const open = $('scene-create').hidden;
  setCreationForm('scene-create', 'scene-create-toggle', open, open ? 'scene-title' : null);
});
$('empty-create-scene').addEventListener('click', () => {
  if (window.matchMedia('(max-width: 820px)').matches) {
    state.libraryOpen = true;
    renderProject();
  }
  setCreationForm('scene-create', 'scene-create-toggle', true, 'scene-title');
});
$('library-toggle').addEventListener('click', () => {
  state.libraryOpen = !state.libraryOpen;
  $('library-content').classList.toggle('is-open', state.libraryOpen);
  $('library-toggle').setAttribute('aria-expanded', String(state.libraryOpen));
});
$('candidate-toggle').addEventListener('click', () => {
  state.candidateOpen = !state.candidateOpen;
  renderCandidateWorkspace();
  if (state.candidateOpen) $('candidate').focus();
});
$('guide-copy').addEventListener('click', () => copyGuide($('guide-copy')));
$('guide-copy-again').addEventListener('click', () => copyGuide($('guide-copy-again')));
$('guide-focus').addEventListener('change', () => {
  if (state.pending) return;
  state.writingFocus = $('guide-focus').value;
  clearGuide();
  clearComparison();
  renderButtons();
  status(state.writingFocus
    ? '수정 초점을 바꿨습니다. 새 작업 지시를 복사해 주세요.'
    : '수정 초점을 해제했습니다.');
});
$('candidate').addEventListener('input', () => {
  state.candidateOpen = true;
  if (!state.candidatePrepareTerminal?.sourceConflictReadback) {
    state.candidatePrepareTerminal = null;
  }
  clearComparison();
  clearPendingReview();
  renderButtons();
});
$('decision-actor').addEventListener('input', renderButtons);
$('text').addEventListener('input', () => {
  if (state.sceneSaveError === 'failed') state.sceneSaveError = null;
  updateWordCount();
  if (sceneTextIsDirty()) {
    clearGuide();
    clearComparison();
  }
  updateSceneDraftStatus(true);
  renderButtons();
});
$('save').addEventListener('click', saveSceneText);
$('save-readback').addEventListener('click', rereadSceneSaveOutcome);
$('compare').addEventListener('click', compareCandidate);
$('prepare').addEventListener('click', prepareCandidate);
$('candidate-prepare-readback').addEventListener('click', rereadCandidatePrepareOutcome);
$('approve').addEventListener('click', approveCandidate);
$('reject').addEventListener('click', rejectCandidate);
$('approval-outcome-action').addEventListener('click', () => {
  if (state.approvalReadback?.latestObservation) {
    openLatestApprovalState();
  } else if (state.approvalReadback) {
    rereadApprovalOutcome();
  } else {
    openLatestApprovalState();
  }
});
$('review').addEventListener('click', runReview);
$('visual').addEventListener('click', runVisualPlan);
$('export').addEventListener('click', exportProject);
$('refresh').addEventListener('click', async () => {
  if (state.candidatePrepareTerminal?.sourceConflictReadback) {
    await rereadCandidatePrepareConflictSource();
    return;
  }
  if (state.candidatePrepareReadback) {
    status('수정안 준비 결과를 이 영역에서 다시 확인해 주세요. 확인 전에는 작업을 바꾸지 않습니다.', true);
    return;
  }
  if (state.approvalReadback) {
    if (state.approvalReadback.latestObservation) await openLatestApprovalState();
    else await rereadApprovalOutcome();
    return;
  }
  if (blockNavigation('새로고침')) return;
  let nextProjectId = null;
  let refreshSucceeded = false;
  await act($('refresh'), '작업 목록을 새로 불러오고 있습니다…', async () => {
    const currentId = state.project?.id;
    const projects = await loadProjects();
    nextProjectId = currentId && projects.some(project => project.id === currentId)
      ? currentId
      : projects[0]?.id || null;
    refreshSucceeded = true;
  });
  if (!refreshSucceeded) return;
  if (nextProjectId) {
    await selectProject(nextProjectId);
  } else {
    state.project = null;
    state.scene = null;
    state.voiceProfile = null;
    state.pending = null;
    state.sceneSaveConflict = false;
    state.sceneSaveError = null;
    state.sceneSaveOutcome = null;
    state.candidateOpen = false;
    $('text').value = '';
    $('candidate').value = '';
    clearGuide();
    renderProject();
    updateWordCount();
  }
});

window.addEventListener('beforeunload', event => {
  if (!unsavedInputLabels().length && !state.busy && !state.navigationBusy) return;
  event.preventDefault();
  event.returnValue = '';
});

async function boot() {
  try {
    const projects = await loadProjects();
    if (projects[0]) await selectProject(projects[0].id);
    renderProject();
  } catch (error) {
    status(friendlyError(error), true);
  }
}

renderButtons();
boot();

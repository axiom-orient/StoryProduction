// Scene HTML Composer — SceneGraph → 브라우저에서 바로 재생 가능한 애니메이션 HTML.
//
// 불변식:
// - 출력은 단일 self-contained HTML 파일이며 외부 네트워크를 사용하지 않는다.
// - 각 샷은 독립적인 레이어 스택(environment → characters → dialogue)이다.
// - 카메라 스펙이 있으면 이동·줌을 표현한다.
// - 에셋이 없으면 밝고 구분 가능한 플레이스홀더로 채운다.
// - 모든 색상은 배경(#1a1a2e)과 충분한 대비를 가져야 한다.

import sceneGraphSchema from '../../../contracts/schemas/scene-graph.schema.json' with { type: 'json' };
import { SchemaRegistry, SchemaValidationError } from '../../contracts/src/schema-validator.js';
import { canonicalizeJson } from '../../core/src/canonical.js';
import {
  TRACKB_SEMANTIC_MANIFEST_ANCHOR,
  TRACKB_SEMANTIC_MANIFEST_ENCODING,
  hashTrackBRawBytes,
  validateTrackBSemanticManifestObject
} from '../../contracts/src/trackb-contract.js';

const SCENE_GRAPH_REGISTRY = new SchemaRegistry();
SCENE_GRAPH_REGISTRY.add(sceneGraphSchema);

const SHOT_PALETTES = [
  { sky: ['#667eea', '#764ba2'], ground: '#2d3561', accent: '#f093fb' },
  { sky: ['#f093fb', '#f5576c'], ground: '#453a94', accent: '#4facfe' },
  { sky: ['#43e97b', '#38f9d7'], ground: '#1a2a6c', accent: '#fa709a' },
  { sky: ['#fa709a', '#fee140'], ground: '#30cfd0', accent: '#667eea' },
];

function esc(text) {
  return String(text ?? '').replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
}

export function composeSceneHtml({ sceneGraph, assets = {}, options = {} }) {
  assertSceneGraph(sceneGraph);
  assertLocalAssets(assets);
  const width = options.width ?? 1280;
  const height = options.height ?? 720;
  if (![width, height].every(value => Number.isSafeInteger(value) && value >= 1 && value <= 16384)) {
    throw new Error("Scene output dimensions must be integers in 1..16384");
  }

  const charMap = new Map((sceneGraph.characters ?? []).map((c) => [c.id, c]));
  const envMap = new Map((sceneGraph.environments ?? []).map((e) => [e.id, e]));
  const propMap = new Map((sceneGraph.props ?? []).map((p) => [p.id, p]));
  const transitionMap = new Map((sceneGraph.timeline.transitions ?? []).map((transition) => [transition.fromShot, transition]));

  const shotsHtml = sceneGraph.shots.map((shot, i) =>
    renderShotHtml(shot, i, { envMap, charMap, propMap, assets, fps: sceneGraph.timeline.fps, transition: transitionMap.get(shot.id) })
  ).join('\n');
  const semanticManifestEmbedding = renderSemanticManifestEmbedding(sceneGraph.semanticManifest);

  return `<!DOCTYPE html>
<html lang="ko">
<head><meta charset="UTF-8"><title>${esc(sceneGraph.sceneGraphId || 'SceneMotionCompiler')}</title>
<style>
*{margin:0;padding:0;box-sizing:border-box}
body{background:#0d1117;display:flex;flex-direction:column;align-items:center;justify-content:center;min-height:100vh;font-family:'Segoe UI',sans-serif;color:#e6edf3}
h3{margin-bottom:12px;color:#58a6ff;font-weight:600}
#stage{position:relative;width:${width}px;height:${height}px;border-radius:12px;overflow:hidden;box-shadow:0 20px 60px rgba(0,0,0,.5)}
.shot{position:absolute;inset:0;opacity:0;transition:opacity .8s ease}
.shot.active{opacity:1}
#stage[data-transition="cut"] .shot{transition:none}
#stage[data-transition="cross-dissolve"] .shot{transition:opacity var(--transition-duration,.8s) ease}
#stage[data-transition="wipe"] .shot.active{animation:wipe-in var(--transition-duration,.8s) ease both}
#stage[data-transition="fade-to-black"] .shot.active,#stage[data-transition="fade-from-black"] .shot.active{animation:fade-through-black var(--transition-duration,.8s) ease both}
@keyframes wipe-in{from{clip-path:inset(0 100% 0 0)}to{clip-path:inset(0 0 0 0)}}
@keyframes fade-through-black{0%{opacity:0}45%{opacity:0}100%{opacity:1}}
.environment{position:absolute;inset:0;display:flex;align-items:flex-end;justify-content:center;padding-bottom:40px}
.environment-image{position:absolute;inset:0;width:100%;height:100%;object-fit:cover}
.env-label{font-size:${Math.round(height / 14)}px;font-weight:bold;color:rgba(255,255,255,.85);text-shadow:0 2px 12px rgba(0,0,0,.5);letter-spacing:2px}
.character{position:absolute;bottom:18%;display:flex;flex-direction:column;align-items:center;gap:6px;opacity:0;transform:translateY(24px);transition:opacity .7s ease,transform .7s cubic-bezier(.175,.885,.32,1.275)}
.shot.active .character{opacity:1;transform:translateY(0);transition-delay:var(--character-delay,0s)}
.sprite{width:140px;height:200px;border-radius:16px;display:flex;align-items:center;justify-content:center;font-size:13px;text-align:center;overflow:hidden;box-shadow:0 8px 24px rgba(0,0,0,.3);border:2px solid rgba(255,255,255,.15)}
.char-name{font-size:14px;font-weight:bold;color:rgba(255,255,255,.92);text-shadow:0 1px 4px rgba(0,0,0,.6)}
.prop{position:absolute;bottom:16%;display:flex;flex-direction:column;align-items:center;gap:4px;opacity:0;transform:translateY(16px) scale(.96);transition:opacity .6s ease,transform .6s ease}
.shot.active .prop{opacity:1;transform:translateY(0) scale(1);transition-delay:var(--prop-delay,0s)}
.prop-visual{width:92px;height:92px;border-radius:12px;display:flex;align-items:center;justify-content:center;overflow:hidden;box-shadow:0 6px 18px rgba(0,0,0,.28);border:2px solid rgba(255,255,255,.2)}
.prop-name{font-size:12px;color:rgba(255,255,255,.9);text-shadow:0 1px 4px rgba(0,0,0,.65)}
.dialogue-overlay{position:absolute;bottom:6%;left:8%;right:8%;padding:18px 28px;background:rgba(0,0,0,.82);backdrop-filter:blur(8px);border-radius:16px;border-left:5px solid #58a6ff;opacity:0;transition:opacity .5s ease .5s}
.shot.active .dialogue-overlay{opacity:1}
.speaker{color:#79c0ff;font-size:15px;font-weight:bold;margin-bottom:6px;display:block}
.line{color:#fff;font-size:20px;line-height:1.6}
.scene-label{position:absolute;top:16px;left:16px;color:rgba(255,255,255,.75);font-size:12px;background:rgba(0,0,0,.4);padding:4px 10px;border-radius:8px;backdrop-filter:blur(4px)}
.progress-bar{position:absolute;bottom:0;left:0;height:4px;background:#58a6ff;border-radius:0 2px 0 0;transition:width linear}
.environment{transform-origin:center}
.shot.active .environment.camera-push-in{animation:camera-push var(--shot-duration) linear both}
.shot.active .environment.camera-pull-out{animation:camera-push var(--shot-duration) linear reverse both}
.shot.active .environment.camera-pan-left{animation:camera-pan var(--shot-duration) linear both}
.shot.active .environment.camera-pan-right{animation:camera-pan var(--shot-duration) linear reverse both}
@keyframes camera-push{from{transform:scale(1)}to{transform:scale(1.15)}}
@keyframes camera-pan{from{transform:translateX(30px) scale(1.1)}to{transform:translateX(-30px) scale(1.1)}}
.environment.camera-push-in{transform:scale(1.15)}
.environment.camera-pan-left{transform:translateX(-40px)}
.environment.camera-pan-right{transform:translateX(40px)}
.environment.camera-close-up{transform:scale(1.3)}
</style></head>
<body>
<h3>🎬 ${esc(sceneGraph.sceneGraphId || 'SceneMotionCompiler')}</h3>
<div id="stage">
${shotsHtml}
</div>
${semanticManifestEmbedding}
<p style="margin-top:12px;color:#8b949e;font-size:12px">샷을 클릭하면 해당 장면으로 건너뜁니다</p>
<script>
(function(){
var shots=document.querySelectorAll('.shot');
var stage=document.getElementById('stage');
// Absolute frame clock: scheduling latency does not accumulate per shot.
var fps=${sceneGraph.timeline.fps}, totalFrames=${sceneGraph.timeline.totalDurationFrames};
var boundaries=${JSON.stringify(sceneGraph.shots.reduce((a,shot) => [...a,a[a.length-1]+shot.durationFrames], [0]))};
var current=-1, epoch=performance.now();
function show(i){
  if(current===i)return;
  var previous=current>=0?shots[current]:null;
  if(previous){previous.classList.remove('active');}
  stage.dataset.transition=previous?.dataset.nextTransitionType||'cut';
  stage.style.setProperty('--transition-duration',Number(previous?.dataset.nextTransitionDurationMs||0)+'ms');
  current=i;
  shots[i].classList.add('active');
}
function tick(now){
  var frame=Math.floor(Math.max(0,now-epoch)*fps/1000)%totalFrames;
  var i=0;
  while(i+1<shots.length&&frame>=boundaries[i+1])i++;
  show(i);
  shots[i].querySelector('.progress-bar').style.width=((frame-boundaries[i])/(boundaries[i+1]-boundaries[i])*100)+'%';
  requestAnimationFrame(tick);
}
// Preserve the public DOM selection contract used by the locked renderer.
shots.forEach(function(el,i){
  el.style.cursor='pointer';
  el.addEventListener('click',function(){
    epoch=performance.now()-boundaries[i]*1000/fps;
    show(i);
  });
});
show(0);
requestAnimationFrame(tick);
})();
</script></body></html>`;
}

function renderSemanticManifestEmbedding(semanticManifest) {
  const payloadBytes = Buffer.from(canonicalizeJson(semanticManifest), 'utf8');
  const payload = payloadBytes.toString('base64url');
  const payloadHash = hashTrackBRawBytes(payloadBytes);
  const semanticManifestHash = semanticManifest.semanticManifestHash;
  return `<div id="${TRACKB_SEMANTIC_MANIFEST_ANCHOR}" hidden aria-hidden="true" encoding="${TRACKB_SEMANTIC_MANIFEST_ENCODING}" semanticManifestHash="${semanticManifestHash}" payloadHash="${payloadHash}">${payload}</div>`;
}

function assertSceneGraph(sceneGraph) {
  try {
    SCENE_GRAPH_REGISTRY.assert(sceneGraphSchema.$id, sceneGraph);
  } catch (error) {
    const snakeCaseField = findSnakeCaseField(sceneGraph);
    if (snakeCaseField) {
      throw new Error(`SceneGraph must use canonical camelCase; snake_case field "${snakeCaseField}" is not supported`);
    }
    if (error instanceof SchemaValidationError && !sceneGraph?.shots?.length) {
      throw new Error('sceneGraph with at least one shot is required');
    }
    throw error;
  }

  const snakeCaseField = findSnakeCaseField(sceneGraph);
  if (snakeCaseField) {
    throw new Error(`SceneGraph must use canonical camelCase; snake_case field "${snakeCaseField}" is not supported`);
  }

  const frames = sceneGraph.shots.reduce((sum, shot) => sum + shot.durationFrames, 0);
  if (!Number.isSafeInteger(frames) || frames !== sceneGraph.timeline.totalDurationFrames) {
    throw new Error('SceneGraph totalDurationFrames must equal the exact sum of shot durations');
  }
  const semanticManifest = validateTrackBSemanticManifestObject(sceneGraph.semanticManifest);
  if (!semanticManifest.valid) throw new Error(`Track B ${semanticManifest.error}`);
  assertReferenceIntegrity(sceneGraph);
}

function findSnakeCaseField(value, path = '$') {
  if (Array.isArray(value)) {
    for (let index = 0; index < value.length; index += 1) {
      const field = findSnakeCaseField(value[index], `${path}[${index}]`);
      if (field) return field;
    }
    return undefined;
  }
  if (!value || typeof value !== 'object') return undefined;

  for (const [key, child] of Object.entries(value)) {
    const fieldPath = `${path}.${key}`;
    if (key.includes('_')) return fieldPath;
    const nestedField = findSnakeCaseField(child, fieldPath);
    if (nestedField) return nestedField;
  }
  return undefined;
}

function assertReferenceIntegrity(sceneGraph) {
  const environmentIds = new Set(sceneGraph.environments.map(({ id }) => id));
  const characterIds = new Set(sceneGraph.characters.map(({ id }) => id));
  const propIds = new Set((sceneGraph.props ?? []).map(({ id }) => id));
  const shotIds = new Set(sceneGraph.shots.map(({ id }) => id));
  const transitionKeys = new Set();
  const transitionSources = new Set();
  const shotIndexes = new Map(sceneGraph.shots.map((shot, index) => [shot.id, index]));

  for (const transition of sceneGraph.timeline.transitions ?? []) {
    if (!shotIds.has(transition.fromShot) || !shotIds.has(transition.toShot)) {
      throw new Error(`SceneGraph transition references unknown shot: ${transition.fromShot} → ${transition.toShot}`);
    }
    if (transition.fromShot === transition.toShot) {
      throw new Error(`SceneGraph transition cannot target the same shot: ${transition.fromShot}`);
    }
    if (shotIndexes.get(transition.toShot) !== shotIndexes.get(transition.fromShot) + 1) {
      throw new Error(`SceneGraph transition must target the next sequential shot: ${transition.fromShot} → ${transition.toShot}`);
    }
    if (transitionSources.has(transition.fromShot)) {
      throw new Error(`SceneGraph transition has multiple outgoing transitions: ${transition.fromShot}`);
    }
    const key = `${transition.fromShot}|${transition.toShot}`;
    if (transitionKeys.has(key)) throw new Error(`SceneGraph transition is duplicated: ${key}`);
    transitionKeys.add(key);
    transitionSources.add(transition.fromShot);
  }

  for (const shot of sceneGraph.shots) {
    if (!environmentIds.has(shot.environmentId)) {
      throw new Error(`SceneGraph reference invalid: shot "${shot.id}" references unknown environment "${shot.environmentId}"`);
    }
    for (const characterId of shot.characterIds ?? []) {
      if (!characterIds.has(characterId)) {
        throw new Error(`SceneGraph reference invalid: shot "${shot.id}" references unknown character "${characterId}"`);
      }
    }
    for (const dialogue of shot.dialogue ?? []) {
      if (!characterIds.has(dialogue.characterId)) {
        throw new Error(`SceneGraph reference invalid: shot "${shot.id}" dialogue references unknown character "${dialogue.characterId}"`);
      }
    }
    for (const propId of shot.propIds ?? []) {
      if (!propIds.has(propId)) {
        throw new Error(`SceneGraph reference invalid: shot "${shot.id}" references unknown prop "${propId}"`);
      }
    }
  }
}

function assertLocalAssets(assets) {
  if (!assets || typeof assets !== 'object' || Array.isArray(assets)) throw new Error('Scene assets must be an object map');
  for (const [assetId, value] of Object.entries(assets)) {
    if (typeof value !== 'string' || value.length === 0) throw new Error(`Scene asset "${assetId}" must be a non-empty URL or path`);
    const isBase64Image = /^data:image\/(?:png|jpeg|webp);base64,[A-Za-z0-9+/=]+$/i.test(value);
    if (!isBase64Image && /^(?:[a-z][a-z0-9+.-]*:|\/\/)/i.test(value)) {
      throw new Error(`Scene asset "${assetId}" must be local or a base64 image`);
    }
  }
}

function renderShotHtml(shot, index, { envMap, charMap, propMap, assets, fps, transition }) {
  const palette = SHOT_PALETTES[index % SHOT_PALETTES.length];
  const env = envMap.get(shot.environmentId);
  const envName = env ? `${env.name}${shot.timeOfDay ? ' · ' + shot.timeOfDay : ''}` : shot.environmentId;
  const durSec = shot.durationFrames / fps;
  const environmentAsset = assets[shot.environmentId];
  const environmentVisual = environmentAsset
    ? `<img class="environment-image" src="${esc(environmentAsset)}" alt="${esc(envName)}">`
    : `<div style="position:absolute;inset:0;background:linear-gradient(180deg,${palette.sky[0]},${palette.sky[1]})"></div>`;

  const charactersHtml = (shot.characterIds ?? []).map((cid, ci) => {
    const character = charMap.get(cid);
    const name = character?.name ?? cid;
    const appearance = character?.appearance ?? '';
    const leftPct = 18 + ci * 28;
    const hue = (ci * 137 + index * 73) % 360;
    const hasAsset = !!assets[cid];
    const inner = hasAsset
      ? `<img src="${esc(assets[cid])}" style="width:100%;height:100%;object-fit:contain" alt="${esc(name)}">`
      : `<div style="font-size:42px;margin-bottom:4px">👤</div><div style="color:rgba(255,255,255,.85)">${esc(appearance.slice(0, 50))}</div>`;
    return `<div class="character" style="left:${leftPct}%;--character-delay:${(ci * 0.2).toFixed(1)}s">
<div class="sprite" style="background:linear-gradient(145deg,hsl(${hue},45%,40%),hsl(${hue},35%,22%))">${inner}</div>
<span class="char-name">${esc(name)}</span></div>`;
  }).join('\n');

  const propsHtml = (shot.propIds ?? []).map((pid, pi) => {
    const prop = propMap.get(pid);
    const name = prop?.name ?? pid;
    const leftPct = 12 + pi * 24;
    const hue = (pi * 97 + index * 41 + 30) % 360;
    const hasAsset = !!assets[pid];
    const inner = hasAsset
      ? `<img src="${esc(assets[pid])}" style="width:100%;height:100%;object-fit:contain" alt="${esc(name)}">`
      : `<div style="font-size:30px">◈</div>`;
    return `<div class="prop" style="left:${leftPct}%;--prop-delay:${(pi * 0.15).toFixed(2)}s">
<div class="prop-visual" style="background:linear-gradient(145deg,hsl(${hue},45%,40%),hsl(${hue},35%,22%))">${inner}</div>
<span class="prop-name">${esc(name)}</span></div>`;
  }).join('\n');

  const dialogueHtml = (shot.dialogue ?? []).length > 0
    ? `<div class="dialogue-overlay">${shot.dialogue.map(dialogue =>
      `<div><span class="speaker">💬 ${esc(charMap.get(dialogue.characterId)?.name ?? dialogue.characterId)}</span><p class="line">${esc(dialogue.line)}</p></div>`).join('')}</div>`
    : '';

  const cameraClass = cameraClassName(shot.camera);
  return `<div class="shot" data-shot-id="${esc(shot.id)}"${transition ? ` data-next-transition-type="${esc(transition.type)}" data-next-transition-duration-ms="${transition.durationFrames * 1000 / fps}"` : ''}>
<div class="environment${cameraClass ? ` ${cameraClass}` : ''}" data-duration="${durSec}" style="--shot-duration:${durSec}s">
${environmentVisual}
<div style="position:absolute;bottom:0;left:0;right:0;height:30%;background:${palette.ground};opacity:.6"></div>
<span class="env-label">📍 ${esc(envName)}</span></div>
${charactersHtml}
${propsHtml}
${dialogueHtml}
<div class="progress-bar"></div>
<div class="scene-label">${esc(shot.id)} · ${esc(envName)} · ${durSec}s</div>
</div>`;
}

function cameraClassName(camera) {
  if (!camera) return '';
  if (camera.cameraMovement === 'PULL_OUT') return 'camera-pull-out';
  if (camera.cameraMovement === 'PUSH_IN') return 'camera-push-in';
  if (camera.cameraMovement === 'PAN_LEFT') return 'camera-pan-left';
  if (camera.cameraMovement === 'PAN_RIGHT') return 'camera-pan-right';
  if (camera.shotSize === 'CLOSE_UP') return 'camera-close-up';
  return '';
}

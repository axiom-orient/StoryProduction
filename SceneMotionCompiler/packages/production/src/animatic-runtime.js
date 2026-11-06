// Pure frame evaluation. Integers in the producer contract; interpolation is
// confined to drawing. No wall clock, file access or mutable playback authority.
export function totalFrames(plan) {
  return plan.panels.reduce((sum, panel) => sum + panel.durationFrames, 0);
}

export function interpolateTrack(track, frame) {
  let left = track[0];
  let right = left;
  for (const key of track) {
    if (key.frame <= frame) left = key;
    if (key.frame > frame) { right = key; break; }
    right = left;
  }
  let t = right.frame === left.frame ? 0 : (frame - left.frame) / (right.frame - left.frame);
  if (left.easing === 'hold') t = 0;
  else if (left.easing === 'easeInOut') t = t * t * (3 - 2 * t);
  const result = {};
  for (const key of Object.keys(left)) {
    if (key !== 'frame' && key !== 'easing') result[key] = left[key] + (right[key] - left[key]) * t;
  }
  return result;
}

export function frameState(plan, frame, reducedMotion = false) {
  if (!Number.isSafeInteger(frame) || frame < 0 || frame >= totalFrames(plan)) throw new Error('FRAME_OUT_OF_RANGE');
  let local = frame;
  let panel;
  for (const candidate of plan.panels) {
    if (local < candidate.durationFrames) { panel = candidate; break; }
    local -= candidate.durationFrames;
  }
  return {
    panelId: panel.panelId, sceneId: panel.sceneId, localFrame: local,
    camera: interpolateTrack(panel.camera, reducedMotion ? 0 : local),
    layers: panel.layers.map((layer) => {
      let pose = layer.poses[0];
      for (const candidate of layer.poses) if (candidate.frame <= local) pose = candidate;
      return { layerId: layer.layerId, assetId: pose.assetId, depthPerMille: layer.depthPerMille,
        ...interpolateTrack(layer.motion, reducedMotion ? 0 : local) };
    })
  };
}

// Exact source slices and absolute frame times are shared by drawing and VTT export.
export function captionTimeline(packet) {
  const timeline = [], ids = new Set();
  let offset = 0;
  packet.plan.panels.forEach((panel, index) => {
    const source = packet.captions[index];
    if (source.panelId !== panel.panelId) throw new Error('CAPTION_PANEL_ORDER');
    const lines = new Map(source.lines.map(line => [line.semanticId, line]));
    if (lines.size !== source.lines.length) throw new Error('CAPTION_SOURCE_DUPLICATE');
    if (!packet.plan.captionStyle) {
      if (panel.captionCues) throw new Error('CAPTION_STYLE_REQUIRED');
      if (source.lines.length) timeline.push({ cueId: `panel-${index + 1}`, panelId: panel.panelId,
        startFrame: offset, endFrame: offset + panel.durationFrames,
        text: source.lines.map(line => line.text).join('\n') });
    } else {
      if (!Array.isArray(panel.captionCues)) throw new Error('CAPTION_CUES_REQUIRED');
      const order = [], ends = new Map();
      let previousEnd = 0;
      for (const cue of panel.captionCues) {
        const line = lines.get(cue.semanticId), a = cue.startUtf16, b = cue.endUtf16;
        if (!line || ids.has(cue.cueId) || ids.size >= 4096
          || ![a, b, cue.startFrame, cue.endFrame].every(Number.isSafeInteger)
          || a < (ends.get(cue.semanticId) ?? 0) || b <= a || b > line.text.length
          || line.text.slice(ends.get(cue.semanticId) ?? 0, a).trim()
          || !line.text.slice(a, b).trim() || /[\uD800-\uDFFF]/u.test(line.text.slice(a, b))
          || cue.startFrame < previousEnd || cue.endFrame <= cue.startFrame
          || cue.endFrame > panel.durationFrames
          || cue.endFrame - cue.startFrame < packet.plan.captionStyle.fadeFrames * 2) {
          throw new Error('CAPTION_CUE_SOURCE_OR_TIME');
        }
        const boundaries = new Set([0, line.text.length,
          ...Array.from(new Intl.Segmenter('ko', { granularity: 'grapheme' }).segment(line.text), part => part.index)]);
        if (!boundaries.has(a) || !boundaries.has(b)) throw new Error('CAPTION_RANGE_SPLITS_GRAPHEME');
        ids.add(cue.cueId); ends.set(cue.semanticId, b); previousEnd = cue.endFrame;
        if (order.at(-1) !== cue.semanticId) order.push(cue.semanticId);
        timeline.push({ ...cue, panelId: panel.panelId, startFrame: offset + cue.startFrame,
          endFrame: offset + cue.endFrame, text: line.text.slice(a, b) });
      }
      if (JSON.stringify(order) !== JSON.stringify(panel.captions)
        || panel.captions.some(id => !lines.has(id) || lines.get(id).text.slice(ends.get(id) ?? 0).trim())) {
        throw new Error('CAPTION_CUE_COVERAGE_ORDER');
      }
    }
    offset += panel.durationFrames;
  });
  return timeline;
}

export function captionAtFrame(cues, frame) {
  return cues.find(cue => frame >= cue.startFrame && frame < cue.endFrame) ?? null;
}

function captionLines(ctx, text, width) {
  const lines = [];
  const segmenter = new Intl.Segmenter('ko', { granularity: 'grapheme' });
  let line = '';
  for (const token of text.split(/(\s+)/u)) {
    if (!token) continue;
    if (ctx.measureText(line + token).width <= width) { line += token; continue; }
    if (line.trim()) { lines.push(line.trim()); line = ''; }
    if (!token.trim()) continue;
    for (const { segment } of segmenter.segment(token)) {
      if (line && ctx.measureText(line + segment).width > width) { lines.push(line); line = ''; }
      line += segment;
    }
  }
  if (line.trim()) lines.push(line.trim());
  return lines;
}

// One drawing program used by editable HTML and CDP export. Native transport
// controls belong to the host. This artifact exposes seekFrame/play/pause only.
export async function installComposition(packet, imageSources, reducedMotion) {
  const plan = packet.plan;
  const canvas = document.querySelector('canvas');
  const ctx = canvas.getContext('2d', { alpha: false });
  const images = {};
  let loadedFonts;
  try { loadedFonts = await document.fonts.load('16px AnimaticLocked'); }
  catch (error) { throw new Error(`LOCKED_FONT_LOAD_FAILED:${error.message}`); }
  if (loadedFonts.length === 0) throw new Error('LOCKED_FONT_NOT_LOADED');
  await document.fonts.ready;
  for (const asset of plan.assets) {
    const img = new Image();
    img.src = imageSources[asset.assetId];
    try { await img.decode(); }
    catch (error) { throw new Error(`IMAGE_DECODE_FAILED:${asset.assetId}:${error.message}`); }
    if (img.naturalWidth !== asset.width || img.naturalHeight !== asset.height) throw new Error('DECODED_IMAGE_DIMENSIONS');
    images[asset.assetId] = img;
  }
  const cues = captionTimeline(packet);
  const style = plan.captionStyle;
  const layouts = new Map();
  if (style) {
    if (style.maxLines !== 2 || style.bandHeight < Math.ceil(style.fontSize * 1.3) * 2 + 12
      || style.bandHeight > plan.canvas.height - 64) throw new Error('CAPTION_BAND_TOO_SMALL');
    ctx.font = `${style.fontSize}px AnimaticLocked`;
    for (const cue of cues) {
      const lines = captionLines(ctx, cue.text, plan.canvas.width - 32);
      if (lines.length > style.maxLines) throw new Error(`CAPTION_CUE_OVERFLOW:${cue.cueId}`);
      layouts.set(cue.cueId, lines);
    }
  }
  let activeFrame = 0;
  let animation = null;
  function seekFrame(frame) {
    const state = frameState(plan, frame, reducedMotion);
    const { width, height } = plan.canvas;
    ctx.globalAlpha = 1;
    ctx.setTransform(1, 0, 0, 1, 0, 0);
    ctx.fillStyle = plan.canvas.background;
    ctx.fillRect(0, 0, width, height);
    const pictureHeight = height - (style?.bandHeight ?? 0);
    ctx.save();
    ctx.beginPath(); ctx.rect(0, 0, width, pictureHeight); ctx.clip();
    for (const layer of state.layers) {
      const img = images[layer.assetId];
      ctx.save();
      ctx.translate(width / 2, pictureHeight / 2);
      ctx.scale(state.camera.scalePerMille / 1000, state.camera.scalePerMille / 1000);
      ctx.translate(-width / 2 - state.camera.x * layer.depthPerMille / 1000,
        -pictureHeight / 2 - state.camera.y * layer.depthPerMille / 1000);
      ctx.translate(layer.x, layer.y);
      ctx.rotate(layer.rotationMilliDeg * Math.PI / 180000);
      ctx.scale(layer.scalePerMille / 1000, layer.scalePerMille / 1000);
      ctx.globalAlpha = layer.opacityPerMille / 1000;
      ctx.drawImage(img, -img.width / 2, -img.height / 2);
      ctx.restore();
    }
    ctx.restore();
    const active = captionAtFrame(cues, frame);
    const text = active?.text ?? '';
    canvas.setAttribute('aria-label', text || state.panelId);
    document.querySelector('p').textContent = text;
    if (style && active) {
      const lines = layouts.get(active.cueId);
      const lineHeight = Math.ceil(style.fontSize * 1.3);
      ctx.font = `${style.fontSize}px AnimaticLocked`;
      ctx.textAlign = 'center'; ctx.textBaseline = 'top';
      ctx.fillStyle = '#ffffff'; ctx.strokeStyle = 'rgba(0,0,0,0.8)'; ctx.lineWidth = 3;
      ctx.globalAlpha = reducedMotion || style.fadeFrames === 0 ? 1 : Math.min(1,
        (frame - active.startFrame + 1) / (style.fadeFrames + 1),
        (active.endFrame - frame) / (style.fadeFrames + 1));
      const top = pictureHeight + (style.bandHeight - lines.length * lineHeight) / 2;
      lines.forEach((line, i) => {
        ctx.strokeText(line, width / 2, top + lineHeight * i);
        ctx.fillText(line, width / 2, top + lineHeight * i);
      });
      ctx.globalAlpha = 1;
    } else if (!style && text) {
      const padding = Math.round(width / 24);
      const fontSize = Math.max(12, Math.round(height / 28));
      const lineHeight = Math.ceil(fontSize * 1.4);
      ctx.font = `${fontSize}px AnimaticLocked`;
      const lines = [];
      for (const paragraph of text.split('\n')) {
        let line = '';
        for (const ch of paragraph) {
          if (line && ctx.measureText(line + ch).width > width - padding * 2) { lines.push(line); line = ''; }
          line += ch;
        }
        lines.push(line);
      }
      const boxHeight = lines.length * lineHeight + fontSize;
      if (boxHeight > height / 2) throw new Error('CAPTION_DOES_NOT_FIT: shorten selection or enlarge canvas');
      ctx.fillStyle = 'rgba(0,0,0,0.82)';
      ctx.fillRect(0, height - boxHeight, width, boxHeight);
      ctx.fillStyle = '#ffffff';
      ctx.textAlign = 'left';
      ctx.textBaseline = 'top';
      lines.forEach((line, i) => ctx.fillText(line, padding, height - boxHeight + fontSize / 2 + lineHeight * i));
    }
    activeFrame = frame;
    return state;
  }
  function pause() { if (animation !== null) cancelAnimationFrame(animation); animation = null; }
  function play() {
    pause();
    const start = performance.now();
    const first = activeFrame;
    const tick = (now) => {
      const frame = Math.min(totalFrames(plan) - 1, first + Math.floor((now - start) * plan.fps / 1000));
      seekFrame(frame);
      animation = frame === totalFrames(plan) - 1 ? null : requestAnimationFrame(tick);
    };
    animation = requestAnimationFrame(tick);
  }
  globalThis.animatic = Object.freeze({ seekFrame: (frame) => { pause(); return seekFrame(frame); },
    play, pause, totalFrames: totalFrames(plan), fps: plan.fps,
    captions: Object.freeze(cues.map(cue => Object.freeze(cue))) });
  seekFrame(0);
  return true;
}


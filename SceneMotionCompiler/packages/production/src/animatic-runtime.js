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

// One drawing program used by editable HTML and CDP export. Native transport
// controls belong to the host. This artifact exposes seekFrame/play/pause only.
export async function installComposition(packet, imageSources, reducedMotion) {
  const plan = packet.plan;
  const canvas = document.querySelector('canvas');
  const ctx = canvas.getContext('2d', { alpha: false });
  const images = {};
  const loadedFonts = await document.fonts.load('16px AnimaticLocked');
  if (loadedFonts.length === 0) throw new Error('LOCKED_FONT_NOT_LOADED');
  await document.fonts.ready;
  for (const asset of plan.assets) {
    const img = new Image();
    img.src = imageSources[asset.assetId];
    await img.decode();
    if (img.naturalWidth !== asset.width || img.naturalHeight !== asset.height) throw new Error('DECODED_IMAGE_DIMENSIONS');
    images[asset.assetId] = img;
  }
  const captions = new Map(packet.captions.map((c) => [c.panelId, c]));
  let activeFrame = 0;
  let animation = null;
  function seekFrame(frame) {
    const state = frameState(plan, frame, reducedMotion);
    const { width, height } = plan.canvas;
    ctx.globalAlpha = 1;
    ctx.setTransform(1, 0, 0, 1, 0, 0);
    ctx.fillStyle = plan.canvas.background;
    ctx.fillRect(0, 0, width, height);
    for (const layer of state.layers) {
      const img = images[layer.assetId];
      ctx.save();
      ctx.translate(width / 2, height / 2);
      ctx.scale(state.camera.scalePerMille / 1000, state.camera.scalePerMille / 1000);
      ctx.translate(-width / 2 - state.camera.x * layer.depthPerMille / 1000,
        -height / 2 - state.camera.y * layer.depthPerMille / 1000);
      ctx.translate(layer.x, layer.y);
      ctx.rotate(layer.rotationMilliDeg * Math.PI / 180000);
      ctx.scale(layer.scalePerMille / 1000, layer.scalePerMille / 1000);
      ctx.globalAlpha = layer.opacityPerMille / 1000;
      ctx.drawImage(img, -img.width / 2, -img.height / 2);
      ctx.restore();
    }
    const caption = captions.get(state.panelId);
    const text = caption.lines.map((line) => line.text).join('\n');
    canvas.setAttribute('aria-label', text || state.panelId);
    document.querySelector('p').textContent = text;
    if (text) {
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
    play, pause, totalFrames: totalFrames(plan), fps: plan.fps });
  seekFrame(0);
  return true;
}


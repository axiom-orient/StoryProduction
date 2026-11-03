/* NarrativeCourier scene graph viewer — zero-dependency SVG renderer.
 *
 * Consumes the deterministic JSON graph projection delivered by
 * `OutputFormat::Json` ({schema_version:1, nodes:[{id,refs,content}], edges:[{from,to,kind}]}).
 * Layout follows the producer's ordered sequence in one deterministic pass. No CDN, no framework — CSP `script-src 'self'` safe.
 */
(function (global) {
  "use strict";

  const NODE_W = 190;
  const NODE_H = 64;
  const GAP_X = 90;
  const PAD = 28;
  let viewerSerial = 0;
  const admittedModels = new WeakSet();

  function closedObject(value, keys, label) {
    if (!value || typeof value !== "object" || Array.isArray(value)
        || Object.keys(value).length !== keys.length || keys.some(key => !Object.prototype.hasOwnProperty.call(value, key))) {
      throw new Error(label + " has missing or unknown fields");
    }
  }

  function parseGraph(json) {
    const data = typeof json === "string" ? JSON.parse(json) : json;
    if (!data || data.schema_version !== 1) throw new Error("unsupported graph schema_version");
    closedObject(data, ["schema_version", "nodes", "edges"], "graph");
    if (!Array.isArray(data.nodes) || !data.nodes.length || !Array.isArray(data.edges)) {
      throw new Error("graph requires nonempty nodes and an edges array");
    }
    const ids = new Set();
    const nodes = data.nodes.map((node, index) => {
      closedObject(node, ["id", "refs", "content"], "node[" + index + "]");
      if (typeof node.id !== "string" || !node.id.trim() || ids.has(node.id)) throw new Error("invalid or duplicate node id");
      ids.add(node.id);
      if (!Array.isArray(node.refs) || !node.refs.length || new Set(node.refs).size !== node.refs.length
          || node.refs.some(ref => typeof ref !== "string" || !/^(SRC|REL|GAP|CON|SCN)-[A-Za-z0-9_-]+$/.test(ref))) {
        throw new Error("node[" + index + "] requires unique canonical refs");
      }
      if (node.content !== null && typeof node.content !== "string") throw new Error("invalid node content");
      return Object.freeze({ id: node.id, refs: Object.freeze(node.refs.slice()), content: node.content });
    });
    const edges = data.edges.map((edge, index) => {
      closedObject(edge, ["from", "to", "kind"], "edge[" + index + "]");
      if (!ids.has(edge.from) || !ids.has(edge.to)) throw new Error("edge[" + index + "] references unknown node");
      if (edge.kind !== "sequence") throw new Error("unsupported graph edge kind");
      return Object.freeze({ from: edge.from, to: edge.to, kind: edge.kind });
    });
    if (edges.length !== nodes.length - 1 || edges.some((edge, index) => edge.from !== nodes[index].id || edge.to !== nodes[index + 1].id)) {
      throw new Error("graph edges must preserve the producer's ordered node sequence");
    }
    const model = Object.freeze({ nodes: Object.freeze(nodes), edges: Object.freeze(edges) });
    admittedModels.add(model);
    return model;
  }

  function computeLayout(model) {
    if (!admittedModels.has(model)) throw new Error("graph layout requires a parsed producer model");
    const positions = new Map(model.nodes.map((node, index) => [node.id, {
      x: PAD + index * (NODE_W + GAP_X), y: PAD, depth: index,
    }]));
    return { positions, width: PAD * 2 + (model.nodes.length - 1) * (NODE_W + GAP_X) + NODE_W,
      height: PAD * 2 + NODE_H };
  }

  function svgEl(name, attrs) {
    const el = document.createElementNS("http://www.w3.org/2000/svg", name);
    for (const key of Object.keys(attrs || {})) el.setAttribute(key, attrs[key]);
    return el;
  }

  function render(container, model, options) {
    const opts = options || {};
    container.replaceChildren();
    container.classList.add("graph-viewer");

    const layout = computeLayout(model);
    const markerId = "graph-arrow-" + (++viewerSerial);
    const viewport = { width: Math.max(NODE_W + PAD * 2, container.clientWidth || 640), height: 520 };

    const svg = svgEl("svg", {
      class: "graph-svg",
      viewBox: "0 0 " + viewport.width + " " + viewport.height,
      role: "group",
      "aria-label": "Scene graph with " + model.nodes.length + " units",
    });
    const defs = svgEl("defs");
    const marker = svgEl("marker", {
      id: markerId,
      viewBox: "0 0 10 10",
      refX: "9",
      refY: "5",
      markerWidth: "7",
      markerHeight: "7",
      orient: "auto-start-reverse",
    });
    marker.appendChild(svgEl("path", { d: "M 0 0 L 10 5 L 0 10 z", fill: "#7f97bd" }));
    defs.appendChild(marker);
    svg.appendChild(defs);

    const root = svgEl("g", { class: "graph-root" });

    for (const edge of model.edges) {
      const from = layout.positions.get(edge.from);
      const to = layout.positions.get(edge.to);
      if (!from || !to) throw new Error("graph layout is missing an admitted node");
      const x1 = from.x + NODE_W;
      const y1 = from.y + NODE_H / 2;
      const x2 = to.x - 6;
      const y2 = to.y + NODE_H / 2;
      const mid = (x1 + x2) / 2;
      root.appendChild(
        svgEl("path", {
          class: "graph-edge",
          d: "M " + x1 + " " + y1 + " C " + mid + " " + y1 + ", " + mid + " " + y2 + ", " + x2 + " " + y2,
          "marker-end": "url(#" + markerId + ")",
        })
      );
    }

    let selectedGroup = null;
    const groups = [];
    model.nodes.forEach((node, index) => {
      const pos = layout.positions.get(node.id);
      const group = svgEl("g", {
        class: "graph-node",
        transform: "translate(" + pos.x + "," + pos.y + ")",
        tabindex: "0",
        role: "button",
        "aria-label": node.id + " · " + node.refs.join(", "),
        "aria-pressed": "false",
      });
      group.appendChild(
        svgEl("rect", {
          width: NODE_W,
          height: NODE_H,
          rx: "10",
          class: "graph-node-box",
          "data-order": String(index),
        })
      );
      const label = svgEl("text", { x: 14, y: 26, class: "graph-node-id" });
      label.textContent = node.refs[0];
      group.appendChild(label);
      const sub = svgEl("text", { x: 14, y: 46, class: "graph-node-sub" });
      const heading = node.content === null ? "Content omitted" : node.content.split("\n", 1)[0];
      sub.textContent = heading.length > 26 ? heading.slice(0, 25) + "…" : heading;
      const title = svgEl("title"); title.textContent = node.id + " · " + heading; group.appendChild(title);
      group.appendChild(sub);

      const activate = () => {
        if (selectedGroup) { selectedGroup.removeAttribute("data-selected"); selectedGroup.setAttribute("aria-pressed", "false"); }
        selectedGroup = group;
        group.setAttribute("data-selected", "true"); group.setAttribute("aria-pressed", "true");
        if (typeof opts.onSelect === "function") opts.onSelect(node);
      };
      group.addEventListener("click", activate);
      group.addEventListener("focus", () => ensureVisible(pos));
      group.addEventListener("keydown", (event) => {
        if (event.key === "Enter" || event.key === " ") {
          event.preventDefault();
          activate();
        } else if (["ArrowRight", "ArrowDown", "ArrowLeft", "ArrowUp", "Home", "End"].includes(event.key)) {
          event.preventDefault();
          const next = event.key === "Home" ? 0 : event.key === "End" ? model.nodes.length - 1
            : Math.max(0, Math.min(model.nodes.length - 1, index + (["ArrowRight", "ArrowDown"].includes(event.key) ? 1 : -1)));
          groups[next].focus();
        }
      });
      groups.push(group);root.appendChild(group);
    });

    svg.appendChild(root);

    const state = { k: 1, x: 0, y: 0 };
    const applyTransform = () => {
      root.setAttribute("transform", "translate(" + state.x + "," + state.y + ") scale(" + state.k + ")");
    };

    function ensureVisible(pos) {
      const left = pos.x * state.k + state.x, top = pos.y * state.k + state.y;
      const right = left + NODE_W * state.k, bottom = top + NODE_H * state.k;
      if (left < PAD) state.x += PAD - left;
      else if (right > viewport.width - PAD) state.x -= right - viewport.width + PAD;
      if (top < PAD) state.y += PAD - top;
      else if (bottom > viewport.height - PAD) state.y -= bottom - viewport.height + PAD;
      applyTransform();
    }
    const pointAt = event => {
      const matrix = svg.getScreenCTM();
      if (!matrix) throw new Error("graph coordinate transform is unavailable");
      const point = svg.createSVGPoint();point.x = event.clientX;point.y = event.clientY;
      return point.matrixTransform(matrix.inverse());
    };

    svg.addEventListener("wheel", (event) => {
      event.preventDefault();
      if (!event.deltaY) return;
      const point = pointAt(event), cx = point.x, cy = point.y;
      const factor = event.deltaY < 0 ? 1.12 : 1 / 1.12;
      const next = Math.min(4, Math.max(0.3, state.k * factor));
      const ratio = next / state.k;
      state.x = cx - (cx - state.x) * ratio;
      state.y = cy - (cy - state.y) * ratio;
      state.k = next;
      applyTransform();
    }, { passive: false });

    let dragging = null;
    svg.addEventListener("pointerdown", event => {
      if (!event.isPrimary || event.button !== 0) return;
      // Node interaction is selection; background interaction is pan.
      if (event.target.closest(".graph-node")) return;
      const point = pointAt(event);
      dragging = { id: event.pointerId, x: point.x, y: point.y, ox: state.x, oy: state.y };
      svg.setPointerCapture(event.pointerId);
    });
    svg.addEventListener("pointermove", event => {
      if (!dragging || event.pointerId !== dragging.id) return;
      const point = pointAt(event);
      const dx = point.x - dragging.x, dy = point.y - dragging.y;
      state.x = dragging.ox + dx;state.y = dragging.oy + dy;applyTransform();
    });
    const finishPan = event => {
      if (!dragging || event.pointerId !== dragging.id) return;
      dragging = null;
      if (svg.hasPointerCapture(event.pointerId)) svg.releasePointerCapture(event.pointerId);
    };
    for (const name of ["pointerup", "pointercancel", "lostpointercapture"]) svg.addEventListener(name, finishPan);

    const reset = document.createElement("button");
    reset.type = "button";
    reset.className = "inline-action graph-reset";
    reset.textContent = "Reset view";
    reset.addEventListener("click", () => {
      state.k = 1;
      state.x = 0;
      state.y = 0;
      applyTransform();
    });

    const toolbar = document.createElement("div");
    toolbar.className = "graph-toolbar";
    const summary = document.createElement("span");
    summary.className = "muted-copy";
    summary.textContent = model.nodes.length + " units · " + model.edges.length + " sequence links · drag background to pan";
    toolbar.append(summary, reset);

    container.append(toolbar, svg);
    applyTransform();
    return { layout };
  }

  const api = { parseGraph, computeLayout, render, NODE_W, NODE_H };
  global.NarrativeCourierGraphViewer = api;
  if (typeof module !== "undefined" && module.exports) module.exports = api;
})(typeof window !== "undefined" ? window : globalThis);

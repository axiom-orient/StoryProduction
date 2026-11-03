/* Mechanical assertions for the scene graph viewer (run by verify-all.sh). */
const assert = require("assert");
const viewer = require("./graph-viewer.js");

const fixture = {
  schema_version: 1,
  nodes: [
    { id: "u1", refs: ["SCN-001"], content: "first" },
    { id: "u2", refs: ["SCN-002", "SCN-003"], content: "second" },
    { id: "u3", refs: ["SCN-004"], content: "third" },
  ],
  edges: [
    { from: "u1", to: "u2", kind: "sequence" },
    { from: "u2", to: "u3", kind: "sequence" },
  ],
};

const model = viewer.parseGraph(fixture);
assert.strictEqual(model.nodes.length, 3);
assert.strictEqual(model.edges.length, 2);
assert.deepStrictEqual(model.nodes[1].refs, ["SCN-002", "SCN-003"]);

const a = viewer.computeLayout(model);
const b = viewer.computeLayout(model);
assert.deepStrictEqual(a, b, "layout must be deterministic");

assert.strictEqual(a.positions.get("u1").depth, 0);
assert.strictEqual(a.positions.get("u2").depth, 1);
assert.strictEqual(a.positions.get("u3").depth, 2);
assert.ok(a.positions.get("u3").x > a.positions.get("u2").x);
assert.ok(a.width > 0 && a.height > 0);

let threw = null;
try {
  viewer.parseGraph({ schema_version: 2, nodes: [], edges: [] });
} catch (error) {
  threw = error;
}
assert.ok(threw && /schema_version/.test(threw.message), "bad schema must be rejected");

threw = null;
try {
  viewer.parseGraph({
    schema_version: 1,
    nodes: [{ id: "a", refs: ["SCN-001"], content: "a" }],
    edges: [{ from: "a", to: "ghost", kind: "sequence" }],
  });
} catch (error) {
  threw = error;
}
assert.ok(threw && /unknown node/.test(threw.message), "dangling edge must be rejected");

const single = viewer.computeLayout(viewer.parseGraph({ schema_version: 1, nodes: [{ id: "solo", refs: ["SCN-001"], content: "solo" }], edges: [] }));
assert.strictEqual(single.positions.get("solo").depth, 0);

console.log("PASS graph-viewer layout determinism and schema guards");

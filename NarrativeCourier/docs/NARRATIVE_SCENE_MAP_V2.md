# NarrativeSceneMap v2

`NarrativeSceneMap v2` is the hash-bound semantic extension of the v1 scene
map.  The v1 scene fields remain unchanged (`scene_id`, `order`, heading and
setting metadata, `characters`, `action`, `dialogue`, and
`asset_requirements`).  The v2 top level additionally requires
`semantic_manifest` and changes `schema_version` to `2`.

## Shape and bounds

- `map_id` is `nsm2_<64 lowercase hexadecimal characters>`.
- `scenes` contains 1–10,000 scenes.  Scene order is contiguous and scene IDs
  are unique `SCN-...` values of at most 64 characters.  The same 64-character
  bound applies to entry and locator `scene_id` values.
- `semantic_manifest.schema_version` is `1`; its `entries` array is bounded
  to 100,000 items.
- Each entry has exactly these fields: `semantic_id`, `entry_hash`, `scene_id`,
  `kind`, `source_text`, and `source_locator`.
- `semantic_id` uses the safe namespace form
  `semantic.<ASCII letter/digit/period/underscore/hyphen>` (up to 128
  characters after the period).  `kind` and `source_locator.field` are one of
  `action`, `dialogue`, `emotion`, `lightingTransition`, or `cameraMovement`.
- `source_text` is 1–16,384 characters.  A locator has only
  `scene_id`, `field`, `occurrence_index`, and the optional `character` and
  `dialogue_index` fields; unknown fields are rejected.  Dialogue locators
  require both optional dialogue fields, and `dialogue_index` must equal
  `occurrence_index` exactly.

STS derives IDs deterministically as
`semantic.<kind>.<scene_id>.<occurrence_index>` with a four-digit,
zero-padded occurrence index.  Repeated source text is therefore retained as
separate occurrences rather than deduplicated.

All objects are closed (`additionalProperties: false`).  The schema also
places finite bounds on scene strings and arrays so an otherwise valid map
cannot grow without limit.

## Semantic invariants

`entry.scene_id` equals `entry.source_locator.scene_id`, and
`entry.kind` equals `entry.source_locator.field`.  Semantic IDs and locators
are unique.  For the v1 arrays, the manifest is an exact bijection:

- every `scene.action[i]` has exactly one `action` entry with
  `occurrence_index=i` and byte-for-byte/equivalent decoded `source_text`;
- every `scene.dialogue[i]` has exactly one `dialogue` entry with
  `dialogue_index = occurrence_index = i`, matching `character`, and exact
  `source_text`.

The two dialogue indices are intentional: identical dialogue text still has
two distinct, verifiable source occurrences.  v1 has no `emotion`,
`lightingTransition`, or `cameraMovement` arrays; those kinds are represented
by manifest entries with a scene-owned locator, while the exact array
bijection applies to the action and dialogue fields that v1 defines.

The three optional directive kinds are emitted only from Scene Blueprint lines
whose labels are exactly `- Emotion:`, `- Lighting transition:`, or
`- Camera movement:` (English labels, case-insensitive).  Their source text is
the non-empty text after the colon.  Similar labels, free-form prose, and
screenplay content do not infer optional meanings.

## Hash domains

Every hash below is SHA-256 over canonical RFC 8785 JSON (JCS), encoded as
UTF-8, with lowercase hexadecimal output.  Object keys are ordered by UTF-16
code units and the contract's integer-only JSON number domain is used.

1. `entry_hash` is `sha256:<digest>` of that entry after removing only
   `entry_hash`.
2. `semantic_manifest_hash` is `sha256:<digest>` of the manifest after
   removing only `semantic_manifest_hash` (entries, including their hashes,
   remain in the domain).
3. `map_id` is `nsm2_<digest>` of the complete map after replacing only
   `map_id` with the empty string.

`source_text` is authoritative as decoded.  The verifier does not Unicode
normalize, trim, collapse whitespace, or rewrite line endings.  Therefore a
newline, a trailing space, and their hashes are distinct source content.

The positive fixture is
`contracts/examples/narrative-scene-map-v2.example.json`.  Run its focused
stdlib-only verifier from the NarrativeCourier root:

```bash
python3 tools/verify_narrative_scene_map_v2.py
```

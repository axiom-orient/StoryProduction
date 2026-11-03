"""Validate visual-layout evidence and bind it to resolved attempt artifacts."""
from __future__ import annotations

from pathlib import Path
import re

from .render_contract_validation import (
    LABEL_GUIDE_POLICIES,
    RenderContractError,
    _crop_safe_inset,
    _integer,
    _normalized_bounds,
    _normalized_point,
    _number,
    _reject_unknown,
    _required,
)
from .util import load_json

_SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
LABEL_GUIDE_STATES = {"PRESENT", "ABSENT", "NOT_APPLICABLE"}
EVIDENCE_VERDICTS = {"PASS", "FAIL", "NA"}


def _hash(value, where):
    if not isinstance(value, str) or not _SHA256_RE.fullmatch(value):
        raise RenderContractError(f"HASH:{where}")
    return value


def _evidence_bounds(value, where):
    return _normalized_bounds(value, where)


def _evidence_point(value, bounds, where):
    return _normalized_point(value, bounds, where)


def _evidence_inset(value, bounds, where):
    return _crop_safe_inset(value, bounds, where)


def _layout_slot_result_pass(item):
    return (
        item["focal_crop_result"]["verdict"] == "PASS"
        and item["label_guide_result"]["verdict"] == "PASS"
        and item["tolerance_verdict"] == "PASS"
    )


def validate_layout_evidence(
    evidence,
    contract,
    attempt_dir=None,
    where="visual_review.layout_evidence",
    require_pass=False,
):
    """Validate slot-level wireframe evidence and optionally bind it to an attempt.

    Hashes are structural evidence when called without ``attempt_dir``.  The CLI and
    approval path pass the attempt directory so the hashes are also checked against
    the resolved source references and the returned PRIMARY artifact.
    """

    if not isinstance(evidence, dict):
        raise RenderContractError(f"TYPE:{where}")
    _reject_unknown(evidence, {"source_sha256", "output_sha256", "slots"}, where)
    source_sha256 = _hash(_required(evidence, "source_sha256", where), f"{where}.source_sha256")
    output_sha256 = _hash(_required(evidence, "output_sha256", where), f"{where}.output_sha256")
    slots = _required(evidence, "slots", where)
    if not isinstance(slots, list) or not slots:
        raise RenderContractError(f"EMPTY:{where}.slots")

    layout = contract.get("wireframe_layout") if isinstance(contract, dict) else None
    if not isinstance(layout, dict):
        raise RenderContractError(f"LAYOUT_EVIDENCE_WITHOUT_LAYOUT:{where}")
    expected_slots = {slot["slot_id"]: slot for slot in layout["slots"]}
    observed_slots = {}
    policy = layout.get("label_guide_policy", "PRESERVE_IF_PRESENT")
    if policy not in LABEL_GUIDE_POLICIES:
        raise RenderContractError(f"LABEL_GUIDE_POLICY:{where}")

    for index, item in enumerate(slots, 1):
        item_where = f"{where}.slots[{index}]"
        _reject_unknown(
            item,
            {"slot_id", "observed_bounds", "observed_rotation_degrees", "observed_z_order", "focal_crop_result", "label_guide_result", "tolerance_verdict"},
            item_where,
        )
        slot_id = _required(item, "slot_id", item_where)
        if slot_id not in expected_slots or slot_id in observed_slots:
            raise RenderContractError(f"LAYOUT_SLOT_SET:{item_where}:{slot_id}")
        observed_slots[slot_id] = item

        observed_bounds = _evidence_bounds(
            _required(item, "observed_bounds", item_where),
            f"{item_where}.observed_bounds",
        )
        observed_rotation = _number(
            _required(item, "observed_rotation_degrees", item_where),
            f"{item_where}.observed_rotation_degrees",
        )
        observed_z_order = _integer(
            _required(item, "observed_z_order", item_where),
            f"{item_where}.observed_z_order",
        )
        focal_crop = _required(item, "focal_crop_result", item_where)
        _reject_unknown(
            focal_crop,
            {"observed_focal_point", "observed_crop_safe_inset", "verdict"},
            f"{item_where}.focal_crop_result",
        )
        observed_focal = _evidence_point(
            _required(focal_crop, "observed_focal_point", f"{item_where}.focal_crop_result"),
            observed_bounds,
            f"{item_where}.focal_crop_result.observed_focal_point",
        )
        observed_inset = _evidence_inset(
            _required(focal_crop, "observed_crop_safe_inset", f"{item_where}.focal_crop_result"),
            observed_bounds,
            f"{item_where}.focal_crop_result.observed_crop_safe_inset",
        )
        if focal_crop["verdict"] not in EVIDENCE_VERDICTS:
            raise RenderContractError(f"VERDICT:{item_where}.focal_crop_result")

        label_guide = _required(item, "label_guide_result", item_where)
        _reject_unknown(
            label_guide,
            {"source_labels", "output_labels", "source_guides", "output_guides", "verdict"},
            f"{item_where}.label_guide_result",
        )
        for key in ("source_labels", "output_labels", "source_guides", "output_guides"):
            state = _required(label_guide, key, f"{item_where}.label_guide_result")
            if state not in LABEL_GUIDE_STATES:
                raise RenderContractError(f"LABEL_GUIDE_STATE:{item_where}.{key}")
        if label_guide["verdict"] not in EVIDENCE_VERDICTS:
            raise RenderContractError(f"VERDICT:{item_where}.label_guide_result")
        tolerance_verdict = _required(item, "tolerance_verdict", item_where)
        if tolerance_verdict not in EVIDENCE_VERDICTS:
            raise RenderContractError(f"VERDICT:{item_where}.tolerance_verdict")

        expected = expected_slots[slot_id]
        tolerance = expected["layout_tolerance"]
        position_ok = (
            abs(observed_bounds["x"] - expected["bounds"]["x"]) <= tolerance["position"]
            and abs(observed_bounds["y"] - expected["bounds"]["y"]) <= tolerance["position"]
        )
        scale_ok = (
            abs(observed_bounds["width"] - expected["bounds"]["width"]) <= tolerance["scale"]
            and abs(observed_bounds["height"] - expected["bounds"]["height"]) <= tolerance["scale"]
        )
        focal_ok = (
            abs(observed_focal["x"] - expected["focal_point"]["x"]) <= tolerance["position"]
            and abs(observed_focal["y"] - expected["focal_point"]["y"]) <= tolerance["position"]
        )
        crop_ok = all(
            observed_inset[key] + tolerance["scale"] >= expected["crop_safe_inset"][key]
            for key in ("top", "right", "bottom", "left")
        )
        rotation_ok = abs(observed_rotation) <= tolerance["rotation"]
        z_order_ok = observed_z_order == expected["z_order"]
        if tolerance_verdict == "PASS" and not rotation_ok:
            raise RenderContractError(f"LAYOUT_ROTATION_FALSE_PASS:{slot_id}")
        if tolerance_verdict == "PASS" and not z_order_ok:
            raise RenderContractError(f"LAYOUT_Z_ORDER_FALSE_PASS:{slot_id}")
        if tolerance_verdict == "PASS" and not (position_ok and scale_ok and focal_ok):
            raise RenderContractError(f"LAYOUT_TOLERANCE_FALSE_PASS:{slot_id}")
        if focal_crop["verdict"] == "PASS" and not (focal_ok and crop_ok):
            raise RenderContractError(f"LAYOUT_FOCAL_CROP_FALSE_PASS:{slot_id}")

        label_states_ok = {
            "REQUIRED": label_guide["output_labels"] == "PRESENT" and label_guide["output_guides"] == "PRESENT",
            "PRESERVE_IF_PRESENT": (
                label_guide["source_labels"] == label_guide["output_labels"]
                and label_guide["source_guides"] == label_guide["output_guides"]
            ),
            "FORBIDDEN": label_guide["output_labels"] == "ABSENT" and label_guide["output_guides"] == "ABSENT",
        }[policy]
        if label_guide["verdict"] == "PASS" and not label_states_ok:
            raise RenderContractError(f"LABEL_GUIDE_FALSE_PASS:{slot_id}")

    if set(observed_slots) != set(expected_slots):
        raise RenderContractError(f"LAYOUT_SLOT_SET:{where}")

    if attempt_dir is not None:
        attempt_dir = Path(attempt_dir)
        attempt = load_json(attempt_dir / "attempt.json")
        source_hashes = {
            ref.get("sha256")
            for ref in attempt.get("resolved_references", [])
            if isinstance(ref, dict)
        }
        if source_sha256 not in source_hashes:
            raise RenderContractError("LAYOUT_SOURCE_HASH_UNBOUND")
        result = load_json(attempt_dir / "result.json")
        output_hashes = {
            artifact.get("sha256")
            for artifact in result.get("artifacts", [])
            if isinstance(artifact, dict)
        }
        if output_sha256 not in output_hashes:
            raise RenderContractError("LAYOUT_OUTPUT_HASH_UNBOUND")

    if require_pass and not all(_layout_slot_result_pass(item) for item in slots):
        raise RenderContractError("VISUAL_LAYOUT_FIDELITY_FALSE_PASS")
    return evidence

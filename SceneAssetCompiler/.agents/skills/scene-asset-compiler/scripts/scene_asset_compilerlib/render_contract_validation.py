"""Canonical semantic validation for RenderContract values and wireframe layout slots."""
from __future__ import annotations


class RenderContractError(ValueError):
    """Raised when a render contract or its layout cannot be trusted."""


CAPTURE_PROFILES = {"NONE", "PHOTOREAL"}
GENRE_PROFILES = {
    "PROJECT_STYLE",
    "PHOTOREAL",
    "JAPANESE_MANGA",
    "FANTASY",
    "PICTURE_BOOK",
}
SEMANTIC_ROLES = {"SCENE", "BACKGROUND", "OBJECT", "CHARACTER"}
LABEL_GUIDE_POLICIES = {"REQUIRED", "PRESERVE_IF_PRESENT", "FORBIDDEN"}
_SLOT_ID_CHARS = set("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789._-")
_SLOT_ID_FIRST_CHARS = set("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789")
STYLE_PRESERVE_AXES = frozenset(
    {
        "line_weight",
        "ink",
        "value_grouping",
        "screentone",
        "palette",
        "edge_language",
        "shading",
        "texture",
        "material_response",
        "lighting_language",
        "camera_language",
        "detail_density",
        "brushwork",
    }
)


def _reject_unknown(value, allowed, where):
    if not isinstance(value, dict):
        raise RenderContractError(f"TYPE:{where}")
    extra = sorted(set(value) - set(allowed))
    if extra:
        raise RenderContractError(f"UNKNOWN_FIELDS:{where}:{extra}")


def _required(value, key, where):
    if key not in value:
        raise RenderContractError(f"MISSING:{where}.{key}")
    return value[key]


def _number(value, where, minimum=None, maximum=None, exclusive_minimum=False):
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise RenderContractError(f"TYPE:{where}")
    if minimum is not None and ((value <= minimum) if exclusive_minimum else (value < minimum)):
        raise RenderContractError(f"RANGE:{where}")
    if maximum is not None and value > maximum:
        raise RenderContractError(f"RANGE:{where}")
    return value


def _integer(value, where):
    if isinstance(value, bool) or not isinstance(value, int):
        raise RenderContractError(f"TYPE:{where}")
    return value


def _normalized_bounds(value, where):
    _reject_unknown(value, {"x", "y", "width", "height"}, where)
    result = {
        key: _number(_required(value, key, where), f"{where}.{key}", 0, 1)
        for key in ("x", "y", "width", "height")
    }
    if result["width"] <= 0 or result["height"] <= 0:
        raise RenderContractError(f"RANGE:{where}.width_height")
    if result["x"] + result["width"] > 1 or result["y"] + result["height"] > 1:
        raise RenderContractError(f"BOUNDS_OUT_OF_RANGE:{where}")
    return result


def _normalized_point(value, bounds, where):
    _reject_unknown(value, {"x", "y"}, where)
    result = {
        key: _number(_required(value, key, where), f"{where}.{key}", 0, 1)
        for key in ("x", "y")
    }
    if not (
        bounds["x"] <= result["x"] <= bounds["x"] + bounds["width"]
        and bounds["y"] <= result["y"] <= bounds["y"] + bounds["height"]
    ):
        raise RenderContractError(f"FOCAL_OUTSIDE_SLOT:{where}")
    return result


def _crop_safe_inset(value, bounds, where):
    _reject_unknown(value, {"top", "right", "bottom", "left"}, where)
    result = {
        key: _number(_required(value, key, where), f"{where}.{key}", 0, 1)
        for key in ("top", "right", "bottom", "left")
    }
    if result["top"] + result["bottom"] > 1 or result["left"] + result["right"] > 1:
        raise RenderContractError(f"CROP_SAFE_INSET_EXCEEDS_SLOT:{where}")
    # Keep the parameter in the validator signature so the relationship is explicit:
    # crop-safe values are normalized to the slot, not to the physical canvas.
    del bounds
    return result


def _layout_tolerance(value, where):
    _reject_unknown(value, {"position", "scale", "rotation"}, where)
    return {
        key: _number(_required(value, key, where), f"{where}.{key}", 0)
        for key in ("position", "scale", "rotation")
    }


def validate_layout_slot(slot, where="wireframe_layout.slots"):
    _reject_unknown(
        slot,
        {
            "slot_id",
            "bounds",
            "intended_aspect_ratio",
            "content_intent",
            "focal_point",
            "crop_safe_inset",
            "z_order",
            "layout_tolerance",
        },
        where,
    )
    slot_id = _required(slot, "slot_id", where)
    if (
        not isinstance(slot_id, str)
        or not slot_id
        or slot_id[0] not in _SLOT_ID_FIRST_CHARS
        or any(c not in _SLOT_ID_CHARS for c in slot_id)
    ):
        raise RenderContractError(f"SLOT_ID:{where}")
    bounds = _normalized_bounds(_required(slot, "bounds", where), f"{where}.{slot_id}.bounds")
    aspect_ratio = _number(
        _required(slot, "intended_aspect_ratio", where),
        f"{where}.{slot_id}.intended_aspect_ratio",
        0,
        exclusive_minimum=True,
    )
    content_intent = _required(slot, "content_intent", where)
    if not isinstance(content_intent, str) or not content_intent.strip():
        raise RenderContractError(f"TYPE:{where}.{slot_id}.content_intent")
    focal_point = _normalized_point(
        _required(slot, "focal_point", where),
        bounds,
        f"{where}.{slot_id}.focal_point",
    )
    crop_safe_inset = _crop_safe_inset(
        _required(slot, "crop_safe_inset", where),
        bounds,
        f"{where}.{slot_id}.crop_safe_inset",
    )
    z_order = _integer(_required(slot, "z_order", where), f"{where}.{slot_id}.z_order")
    layout_tolerance = _layout_tolerance(
        _required(slot, "layout_tolerance", where),
        f"{where}.{slot_id}.layout_tolerance",
    )
    return {
        "slot_id": slot_id,
        "bounds": bounds,
        "intended_aspect_ratio": aspect_ratio,
        "content_intent": content_intent,
        "focal_point": focal_point,
        "crop_safe_inset": crop_safe_inset,
        "z_order": z_order,
        "layout_tolerance": layout_tolerance,
    }


def _validate_wireframe_layout(value, where):
    _reject_unknown(value, {"canvas", "slots", "label_guide_policy"}, where)
    canvas = _required(value, "canvas", where)
    _reject_unknown(canvas, {"width", "height"}, f"{where}.canvas")
    width = _integer(_required(canvas, "width", f"{where}.canvas"), f"{where}.canvas.width")
    height = _integer(_required(canvas, "height", f"{where}.canvas"), f"{where}.canvas.height")
    if width < 1 or height < 1:
        raise RenderContractError(f"RANGE:{where}.canvas")
    slots = _required(value, "slots", where)
    if not isinstance(slots, list) or not slots:
        raise RenderContractError(f"EMPTY:{where}.slots")
    normalized = []
    seen = set()
    for slot in slots:
        item = validate_layout_slot(slot, f"{where}.slots")
        if item["slot_id"] in seen:
            raise RenderContractError(f"DUPLICATE_SLOT:{item['slot_id']}")
        seen.add(item["slot_id"])
        normalized.append(item)
    policy = value.get("label_guide_policy", "PRESERVE_IF_PRESENT")
    if policy not in LABEL_GUIDE_POLICIES:
        raise RenderContractError(f"LABEL_GUIDE_POLICY:{where}")
    return {
        "canvas": {"width": width, "height": height},
        "slots": normalized,
        "label_guide_policy": policy,
    }


def validate_render_contract(contract, where="render_contract"):
    _reject_unknown(
        contract,
        {"schema_version", "capture_profile", "genre_profile", "semantic_role", "wireframe_layout"},
        where,
    )
    if _required(contract, "schema_version", where) != 1:
        raise RenderContractError(f"VERSION:{where}")
    capture = _required(contract, "capture_profile", where)
    genre = _required(contract, "genre_profile", where)
    role = _required(contract, "semantic_role", where)
    if capture not in CAPTURE_PROFILES:
        raise RenderContractError(f"CAPTURE_PROFILE:{where}")
    if genre not in GENRE_PROFILES:
        raise RenderContractError(f"GENRE_PROFILE:{where}")
    if role not in SEMANTIC_ROLES:
        raise RenderContractError(f"SEMANTIC_ROLE:{where}")
    if genre == "PHOTOREAL" and capture != "PHOTOREAL":
        raise RenderContractError(
            f"RENDER_CONTRACT_CONFLICT:{where}:PHOTOREAL_GENRE_REQUIRES_PHOTOREAL_CAPTURE"
        )
    layout = contract.get("wireframe_layout")
    if layout is not None and role not in {"SCENE", "BACKGROUND"}:
        raise RenderContractError(
            f"RENDER_CONTRACT_CONFLICT:{where}:WIREFRAME_REQUIRES_SCENE_OR_BACKGROUND_ROLE"
        )
    if layout is not None:
        _validate_wireframe_layout(layout, f"{where}.wireframe_layout")
    return contract


def validate_style_reference_separation(references, where="references"):
    """Keep STYLE references on explicit style axes and out of source content."""

    if references is None:
        return
    if not isinstance(references, list):
        raise RenderContractError(f"TYPE:{where}")
    for index, reference in enumerate(references, 1):
        if not isinstance(reference, dict):
            continue
        if reference.get("role") != "STYLE":
            continue
        preserve_values = reference.get("preserve", [])
        ignore_values = reference.get("ignore", [])
        if not isinstance(preserve_values, list):
            raise RenderContractError(f"TYPE:{where}[{index}].preserve")
        if not isinstance(ignore_values, list):
            raise RenderContractError(f"TYPE:{where}[{index}].ignore")
        if any(not isinstance(item, str) for item in preserve_values):
            raise RenderContractError(f"TYPE:{where}[{index}].preserve")
        if any(not isinstance(item, str) for item in ignore_values):
            raise RenderContractError(f"TYPE:{where}[{index}].ignore")
        unsafe = sorted(set(preserve_values) - STYLE_PRESERVE_AXES)
        if unsafe:
            raise RenderContractError(
                f"STYLE_REFERENCE_UNSAFE_PRESERVE:{where}[{index}]:{unsafe}"
            )
        preserve = set(preserve_values)
        ignore = set(ignore_values)
        overlap = sorted(preserve & ignore)
        if overlap:
            raise RenderContractError(f"STYLE_REFERENCE_OVERLAP:{where}[{index}]:{overlap}")

"""Deterministic prompt projection for an already validated render contract."""
from __future__ import annotations

from .render_contract_validation import (
    RenderContractError,
    validate_render_contract,
    validate_style_reference_separation,
)


def _format_number(value):
    if isinstance(value, int):
        return str(value)
    return format(value, ".12g")


def _csv(values):
    return ", ".join(str(value).strip() for value in values if str(value).strip()) or "none"


def prompt_lines(contract, references=None, transparent_background=False, slot_id=None):
    """Return stable, ordered prompt lines for a validated render contract."""

    validate_render_contract(contract)
    validate_style_reference_separation(references)
    capture = contract["capture_profile"]
    genre = contract["genre_profile"]
    role = contract["semantic_role"]
    lines = [
        f"Render contract: capture_profile={capture}; genre_profile={genre}; semantic_role={role}.",
        "Style-reference boundary: style references control style only; preserve/ignore declarations stay separate; no source subject, layout, or content leakage.",
    ]
    if references:
        for index, reference in enumerate(references, 1):
            if reference.get("role") == "STYLE":
                lines.append(
                    f"Style reference {index}: preserve={_csv(reference.get('preserve', []))}; "
                    f"ignore={_csv(reference.get('ignore', []))}; use only its declared style attributes."
                )

    if capture == "PHOTOREAL":
        lines.append(
            "Photoreal capture: coherent camera and lens behavior; controlled exposure; physically plausible light transport; material-specific response; correct anatomy; natural imperfection and restrained asymmetry."
        )
    if genre == "PHOTOREAL":
        lines.append(
            "Photoreal genre: prioritize camera/exposure truth, light transport, material response, anatomy, and natural imperfection over glossy or generic CGI polish."
        )
    elif genre == "JAPANESE_MANGA":
        lines.append(
            "Japanese manga genre: deliberate line weight, confident ink, readable value grouping, controlled screentone, and clear panel readability; no named-artist imitation."
        )
    elif genre == "FANTASY":
        lines.append(
            "Fantasy genre: obey the declared world rules; preserve material continuity and weather continuity; avoid generic game-CGI drift."
        )
    elif genre == "PICTURE_BOOK":
        lines.append(
            "Picture-book genre: honor the requested age band; simplify shapes intentionally; control palette and edge language; protect page-safe negative space; no text unless requested."
        )

    if role == "BACKGROUND":
        lines.append(
            "Background role: maintain coherent perspective, depth, and motivated light; protect negative space; no accidental foreground hero."
        )
    elif role == "OBJECT":
        lines.append(
            "Object role: show the complete object extent with an explicit view, scale, and material; no hand, support, or text."
        )
        if transparent_background:
            lines.append(
                "Transparent object output: preserve genuine alpha and do not bake a contact shadow into the object."
            )

    layout = contract.get("wireframe_layout")
    if layout:
        slots = layout["slots"]
        if slot_id is not None:
            slots = [slot for slot in slots if slot["slot_id"] == slot_id]
            if not slots:
                raise RenderContractError(f"UNKNOWN_SLOT:{slot_id}")
        else:
            slots = sorted(slots, key=lambda slot: slot["slot_id"])
        lines.append(
            "Wireframe layout: lock stable slot geometry; preserve every slot's bounds, intended aspect ratio, focal point, crop-safe inset, z-order, and layout tolerance."
        )
        lines.append(
            f"Wireframe labels/guides policy: {layout.get('label_guide_policy', 'PRESERVE_IF_PRESENT')}; apply this exact policy to labels and guides while preserving slot geometry."
        )
        for slot in slots:
            bounds = slot["bounds"]
            focal = slot["focal_point"]
            inset = slot["crop_safe_inset"]
            tolerance = slot["layout_tolerance"]
            lines.append(
                f"Layout slot {slot['slot_id']}: content={slot['content_intent']}; "
                f"bounds=(x={_format_number(bounds['x'])}, y={_format_number(bounds['y'])}, "
                f"width={_format_number(bounds['width'])}, height={_format_number(bounds['height'])}); "
                f"aspect={_format_number(slot['intended_aspect_ratio'])}; "
                f"focal=({_format_number(focal['x'])}, {_format_number(focal['y'])}); "
                f"crop_safe_inset=(top={_format_number(inset['top'])}, right={_format_number(inset['right'])}, "
                f"bottom={_format_number(inset['bottom'])}, left={_format_number(inset['left'])}); "
                f"z_order={slot['z_order']}; "
                f"layout_tolerance=(position={_format_number(tolerance['position'])}, "
                f"scale={_format_number(tolerance['scale'])}, rotation={_format_number(tolerance['rotation'])})."
            )
        lines.append(
            "Placeholder-photo staging: generate each slot placeholder independently before PSD packaging; do not collapse slots into one image before the PSD stage."
        )
    return lines

from __future__ import annotations
import importlib.util
from pathlib import Path

from .psd import parse_psd

def validate_generated_psd(source, profile, required_layers, expected_canvas=None):
    """Validate a GPT/$imagegen produced PSD against SceneAssetCompiler PSDGenerationProfile."""
    from .psd_profile import resolved_profile

    resolved = resolved_profile(profile)
    info = parse_psd(Path(source).read_bytes())
    findings = []

    if info['version'] != 1:
        findings.append(f"PSD_VERSION_EXPECTED_1:{info['version']}")
    if info['color_mode'] != 3:
        findings.append(f"PSD_COLOR_MODE_NOT_RGB:{info['color_mode']}")
    if info['depth'] != 8:
        findings.append(f"PSD_BIT_DEPTH_NOT_8:{info['depth']}")

    icc_profile = info['image_resources'].get('icc_profile')
    if not icc_profile:
        findings.append("PSD_ICC_PROFILE_MISSING")
    elif not icc_profile.get('valid_header'):
        findings.append("PSD_ICC_PROFILE_INVALID")
    elif not icc_profile.get('is_srgb'):
        findings.append("PSD_ICC_PROFILE_NOT_SRGB")
    if info['image_resources'].get('icc_untagged'):
        findings.append("PSD_ICC_INTENTIONALLY_UNTAGGED")

    if expected_canvas and (info['width'], info['height']) != tuple(expected_canvas):
        findings.append(
            f"PSD_CANVAS_MISMATCH:{info['width']}x{info['height']}:"
            f"{expected_canvas[0]}x{expected_canvas[1]}"
        )

    layers = info['layers']
    layer_names = [layer['name'] for layer in layers]
    for layer_id in required_layers:
        if layer_id not in layer_names:
            findings.append(f"PSD_REQUIRED_LAYER_MISSING:{layer_id}")
    if len(layer_names) != len(set(layer_names)):
        findings.append("PSD_DUPLICATE_LAYER_NAMES")

    for layer in layers:
        if layer['name'] in required_layers:
            if not layer.get('is_pixel_leaf'):
                findings.append(f"PSD_REQUIRED_LAYER_NOT_PIXEL_LEAF:{layer['name']}")
            if layer['has_user_pixel_mask']:
                findings.append(f"PSD_USER_PIXEL_MASK_FORBIDDEN:{layer['name']}")
            if layer.get('has_vector_mask'):
                findings.append(f"PSD_VECTOR_MASK_FORBIDDEN:{layer['name']}")
            if layer.get('has_smart_object'):
                findings.append(f"PSD_SMART_OBJECT_FORBIDDEN:{layer['name']}")
            left, top, right, bottom = layer['bounds']
            if right <= left or bottom <= top:
                findings.append(f"PSD_LAYER_EMPTY_BOUNDS:{layer['name']}")
            if left < 0 or top < 0 or right > info['width'] or bottom > info['height']:
                findings.append(f"PSD_LAYER_BOUNDS_OUTSIDE_CANVAS:{layer['name']}")

    saved_paths = {
        path['name']: path for path in info['image_resources']['saved_paths']
    }
    for path_layer in resolved['path_layers']:
        layer_id = path_layer['layer_id']
        expected_path = f"clip.{layer_id}"
        source_layer = next((layer for layer in layers if layer['name'] == layer_id), None)
        if source_layer is not None and -1 not in source_layer.get('channel_ids', []):
            findings.append(f"PSD_TRANSPARENCY_CHANNEL_MISSING:{layer_id}")
        if expected_path not in saved_paths:
            findings.append(f"PSD_SAVED_PATH_MISSING:{expected_path}")
        elif not saved_paths[expected_path].get('valid_path_structure'):
            findings.append(f"PSD_PATH_STRUCTURE:{expected_path}")

    clipping_path = info['image_resources'].get('clipping_path')
    if resolved['document_role'] == "SCENE_MASTER":
        if clipping_path:
            findings.append(
                f"PSD_SCENE_CLIPPING_PATH_FORBIDDEN:{clipping_path.get('path_name')}"
            )
    else:
        primary = resolved['primary_cutout_layer_id']
        primary_profile = next(
            item for item in resolved['path_layers'] if item['layer_id'] == primary
        )
        if primary_profile['edge_class'] in ("HARD", "ORGANIC"):
            expected_path = f"clip.{primary}"
            clipping_policy = resolved.get('document_clipping_path')
            if clipping_policy == "PRIMARY_SAVED_PATH":
                if not clipping_path or clipping_path.get('path_name') != expected_path:
                    findings.append(f"PSD_CLIPPING_PATH_REQUIRED:{expected_path}")
            elif clipping_policy == "NONE" and clipping_path:
                findings.append(
                    f"PSD_CLIPPING_PATH_FORBIDDEN:{clipping_path.get('path_name')}"
                )
        elif clipping_path:
            findings.append("PSD_SOFT_ALPHA_CLIPPING_PATH_FORBIDDEN")

    if clipping_path and clipping_path.get('path_name') not in saved_paths:
        findings.append(
            f"PSD_CLIPPING_PATH_TARGET_MISSING:{clipping_path.get('path_name')}"
        )

    render_back = {
        'status': 'NOT_RUN_TOOLCHAIN',
        'reason': 'psd-tools is not installed',
    }
    if importlib.util.find_spec('psd_tools') is not None:
        render_back = {
            'status': 'NOT_RUN',
            'reason': 'optional psd-tools render-back was not requested by this structural validator',
        }

    return {
        'status': 'PASS' if not findings else 'FAIL',
        'findings': findings,
        'profile': resolved,
        'psd': info,
        'render_back': render_back,
        'alpha_evidence': {
            'status': 'STRUCTURE_ONLY',
            'reason': 'channel declaration is checked; decoded layer alpha pixels require render-back or an external receipt',
        },
    }

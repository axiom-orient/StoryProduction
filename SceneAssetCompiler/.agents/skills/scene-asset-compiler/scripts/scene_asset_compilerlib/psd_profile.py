from __future__ import annotations

class PsdProfileError(ValueError): pass

VALID_ROLES={"SINGLE_ASSET_MASTER","SCENE_MASTER"}
VALID_EDGES={"HARD","ORGANIC","SOFT_ALPHA"}

DEFAULT_TOLERANCE={"HARD":0.75,"ORGANIC":1.5,"SOFT_ALPHA":3.0}

DEFAULT_PROFILE={
    "schema_version":1,
    "document":{
        "format":"PSD",
        "mode":"RGB",
        "bit_depth":8,
        "color_profile":"sRGB IEC61966-2.1",
        "preserve_exact_canvas":True,
        "smart_objects":"FORBID",
        "adjustment_layers":"FORBID_UNLESS_REQUESTED",
    },
    "layers":{
        "native_pixel_layers":True,
        "exact_machine_ids":True,
        "normal_transparency_channel":True,
        "pixel_user_layer_masks":"FORBID",
        "vector_masks":"FORBID_UNLESS_REQUESTED",
        "trim":"FORBID",
    },
    "paths":{
        "saved_path_required_for_declared_path_layers":True,
        "path_name_template":"clip.{layer_id}",
        "work_path_must_be_saved":True,
        "clipping_flatness":"DEFAULT_UNSET",
    },
}

def validate_profile(p):
    if p.get("schema_version")!=1: raise PsdProfileError("PSD_PROFILE_VERSION")
    role=p.get("document_role")
    if role not in VALID_ROLES: raise PsdProfileError(f"PSD_PROFILE_ROLE:{role}")
    layers=p.get("path_layers")
    if not isinstance(layers,list): raise PsdProfileError("PSD_PROFILE_PATH_LAYERS")
    seen=set()
    for x in layers:
        if not isinstance(x,dict): raise PsdProfileError("PSD_PROFILE_PATH_LAYER_TYPE")
        lid=x.get("layer_id"); edge=x.get("edge_class")
        if not isinstance(lid,str) or not lid: raise PsdProfileError("PSD_PROFILE_PATH_LAYER_ID")
        if lid in seen: raise PsdProfileError(f"PSD_PROFILE_PATH_LAYER_DUPLICATE:{lid}")
        seen.add(lid)
        if edge not in VALID_EDGES: raise PsdProfileError(f"PSD_PROFILE_EDGE:{lid}:{edge}")
        tol=x.get("work_path_tolerance_px")
        if tol is not None and not (0.5 <= float(tol) <= 10.0):
            raise PsdProfileError(f"PSD_PROFILE_TOLERANCE:{lid}")
    primary=p.get("primary_cutout_layer_id")
    if role=="SINGLE_ASSET_MASTER":
        if not primary: raise PsdProfileError("PSD_PROFILE_PRIMARY_LAYER_REQUIRED")
        if primary not in seen: raise PsdProfileError("PSD_PROFILE_PRIMARY_NOT_PATH_LAYER")
    if role=="SCENE_MASTER" and p.get("document_clipping_path") not in (None,"NONE"):
        raise PsdProfileError("PSD_PROFILE_SCENE_CLIPPING_PATH_FORBIDDEN")
    if primary:
        pl=next((x for x in layers if x["layer_id"]==primary),None)
        if pl and pl["edge_class"]=="SOFT_ALPHA" and p.get("document_clipping_path") not in (None,"NONE"):
            raise PsdProfileError("PSD_PROFILE_SOFT_CLIPPING_PATH_FORBIDDEN")
    return p

def resolved_profile(p):
    validate_profile(p)
    out={**DEFAULT_PROFILE, **{k:v for k,v in p.items() if k not in DEFAULT_PROFILE}}
    out["path_layers"]=[]
    for x in p["path_layers"]:
        y=dict(x)
        y.setdefault("work_path_tolerance_px",DEFAULT_TOLERANCE[y["edge_class"]])
        out["path_layers"].append(y)
    role=p["document_role"]
    if role=="SCENE_MASTER":
        out["work_path_policy"]="TEMPORARY_ONLY_SAVE_THEN_CLEAR"
        out["document_clipping_path"]="NONE"
    else:
        out["work_path_policy"]="TEMPORARY_ONLY_SAVE_THEN_CLEAR"
        primary=next(x for x in out["path_layers"] if x["layer_id"]==p["primary_cutout_layer_id"])
        out["document_clipping_path"]="NONE" if primary["edge_class"]=="SOFT_ALPHA" else (p.get("document_clipping_path") or "PRIMARY_SAVED_PATH")
    return out

def prompt_lines(p, required_layer_ids):
    r=resolved_profile(p)
    role=r["document_role"]
    lines=[
      "PSD delivery requirements:",
      "- Return a real editable Adobe Photoshop PSD, not a flattened image renamed .psd.",
      "- Capability status: a real editable PSD is a real capability; this invocation may return PREFERRED_OUTPUT_MISSING, which is invocation-specific evidence and never a global unsupported declaration.",
      "- Packaging retry discipline: PREFERRED_OUTPUT_MISSING is a format/package failure, not a visual failure. Permit at most one PSD-only retry using the approved locked pixels and layout; do not redraw, restyle, relight, recompose, or assemble PSD locally. If PSD is missing a second time, stop and retain previews/evidence without claiming PSD unsupported.",
      "- Document: RGB, 8 bits/channel, embedded sRGB IEC61966-2.1 profile, exact requested canvas; do not crop or resize.",
      "- Use native pixel layers with ordinary transparency channels. Do not use Photoshop pixel/user layer masks, and do not substitute a layer vector mask for the formal Paths-panel Saved Path.",
      "- Do not convert semantic layers into Smart Objects. Do not add adjustment layers unless explicitly requested.",
      "- Keep every required semantic leaf as an independently editable pixel layer with its exact machine ID.",
      "- Do not trim individual layers after registration.",
    ]
    if required_layer_ids:
        lines.append("- Required leaf layer IDs: " + "; ".join(required_layer_ids) + ".")
    if r["path_layers"]:
        lines.append("- Only the following declared cutout/path layers require formal Paths-panel boundaries:")
        for x in r["path_layers"]:
            lines.append(f"  - {x['layer_id']}: edge={x['edge_class']}; Work Path tolerance target about {x['work_path_tolerance_px']} px; save as `clip.{x['layer_id']}`.")
        lines += [
          "- For each declared cutout/path layer, create the final clean cutout with ordinary RGBA transparency, create a Work Path from that boundary, clean/simplify it, then SAVE/RENAME it as the exact Named Saved Path above.",
          "- The Named Saved Path is canonical; do not leave the only boundary as an unsaved Work Path.",
          "- Preserve smooth Bezier curves and use the fewest clean anchor points that remain visually faithful.",
        ]
    else:
        lines.append("- No formal Saved Path is required for layers that are not declared cutout/path layers.")
    if role=="SCENE_MASTER":
        lines += [
          "- Multi-layer scene PSD: do NOT set a document clipping path.",
          "- After each declared boundary has been promoted to its Named Saved Path, the temporary Work Path does not need to remain.",
        ]
    else:
        primary=r["primary_cutout_layer_id"]
        pinfo=next(x for x in r["path_layers"] if x["layer_id"]==primary)
        if pinfo["edge_class"] in ("HARD","ORGANIC"):
            lines += [
              "- Single-asset PSD: after saving/renaming the Work Path, the formal Named Saved Path is the authority; no surviving Work Path copy is required.",
              "- Set that primary Named Saved Path as the document clipping path.",
              "- Clipping-path Flatness: leave unset/default. Do not invent a numeric value unless a downstream print contract explicitly requires one.",
            ]
        else:
            lines += [
              "- Soft-alpha asset: preserve the real soft RGBA alpha for hair/fur/translucency.",
              "- The Saved Path is only a coarse semantic envelope for selection/editing.",
              "- Do NOT set a document clipping path.",
            ]
    if any(x["edge_class"]=="SOFT_ALPHA" for x in r["path_layers"]):
        lines.append("- For every SOFT_ALPHA path layer, preserve the real soft alpha; do not harden hair/fur/translucent edges to match the path.")
    lines += [
      "- Do not rasterize Saved Paths into layer masks.",
      "- Do not silently merge cutout layers.",
      "- Preserve supplied approved pixels, identity, registration, composition and lighting; this PSD step is packaging/editability work, not a redesign pass.",
    ]
    return lines

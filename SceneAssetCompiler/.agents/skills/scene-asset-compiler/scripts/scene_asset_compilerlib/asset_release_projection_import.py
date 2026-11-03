"""Read, validate, and exact-copy an SMC projection into a SAC run."""
from __future__ import annotations

from pathlib import Path
from typing import Any

from .asset_release_contracts import (
    PROJECTION_RECEIPT_RELATIVE_PATH,
    SOURCE_KIND,
    AssetReleaseError,
    _hex,
    _relative_path,
)
from .asset_release_files import _copy_exact, _read_json, _strict_root
from .asset_release_projection_contracts import (
    _validate_cinematic_sources,
    _validate_projection,
    _validate_scene_graph,
)


def import_nvc_released_scene_projection(receipt_path: Path, project_root: Path, destination_root: Path, expected_sha256: str) -> dict[str, Any]:
    """Copy a verified SMC projection into a SceneAssetCompiler run without byte drift."""

    source_root = _strict_root(Path(project_root))
    receipt_path = Path(receipt_path)
    try:
        receipt_relative = receipt_path.absolute().relative_to(source_root)
    except ValueError as error:
        raise AssetReleaseError("PATH_UNSAFE:projectionReceipt") from error
    receipt_relative = _relative_path(receipt_relative.as_posix(), "projectionReceipt")
    if receipt_relative != PROJECTION_RECEIPT_RELATIVE_PATH:
        raise AssetReleaseError(f"PROJECTION_PATH_UNSUPPORTED:{receipt_relative}")
    receipt, receipt_bytes, actual_receipt_hash = _read_json(source_root, receipt_relative, "projectionReceipt", _hex(expected_sha256, "projectionReceipt.sha256"))
    _validate_projection(receipt)
    graph_relative = receipt["sceneGraphPath"]
    graph, graph_bytes, actual_graph_raw_hash = _read_json(source_root, graph_relative, "sceneGraph")
    _validate_scene_graph(graph, receipt["sceneGraphHash"], receipt)
    _validate_cinematic_sources(receipt, graph)
    destination_root = Path(destination_root)
    if not destination_root.exists():
        destination_root.mkdir(parents=True)
    _strict_root(destination_root)
    if destination_root != destination_root.absolute():
        destination_root = destination_root.absolute()
    _copy_exact(receipt_bytes, actual_receipt_hash, destination_root, receipt_relative, "projectionReceipt")
    _copy_exact(graph_bytes, actual_graph_raw_hash, destination_root, graph_relative, "sceneGraph")
    # The graph's raw hash is useful for diagnostics, but the contract binds
    # the canonical SceneGraph hash in the receipt.
    return {
        "schemaVersion": 2,
        "sourceKind": SOURCE_KIND,
        "projectionReceiptPath": receipt_relative,
        "projectionReceiptSha256": actual_receipt_hash,
        "sceneGraphPath": graph_relative,
        "sceneGraphRawSha256": actual_graph_raw_hash,
        "sceneGraphHash": receipt["sceneGraphHash"],
        "sourceSemanticManifestHash": receipt["sourceSemanticManifestHash"],
        "semanticManifestHash": receipt["semanticManifestHash"],
        "sceneCount": receipt["sceneCount"],
        "sceneIds": list(receipt["sceneIds"]),
        "receipt": receipt,
        "sceneGraph": graph,
        "sceneGraphBytes": graph_bytes,
        "receiptBytes": receipt_bytes,
    }

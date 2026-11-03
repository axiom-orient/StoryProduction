//! Standalone HTML projection with explicit, captured image attachments.

use super::html_escape;
use crate::encoding::base64_encode;
use crate::engine::read_materialized_image;
use narrative_courier_contracts::{AssetIndex, DeliveryPlan, DirectorPlan};
use narrative_courier_runtime::{Error, Result};
use std::collections::BTreeMap;
use std::path::Path;

pub(in crate::render) fn html_document(
    project: &Path,
    plan: &DirectorPlan,
    delivery: &DeliveryPlan,
    asset_index: &AssetIndex,
) -> Result<String> {
    let items: BTreeMap<_, _> = asset_index
        .assets
        .iter()
        .map(|item| (item.asset_id.as_str(), item))
        .collect();
    let requests: BTreeMap<_, _> = plan
        .asset_requests
        .iter()
        .map(|request| (request.asset_id.as_str(), request))
        .collect();
    let mut body = String::new();
    for unit in &delivery.units {
        let mut media = String::new();
        for asset_id in &unit.asset_refs {
            let item = items.get(asset_id.as_str()).copied().ok_or_else(|| {
                Error::Contract(format!("dangling materialized asset: {asset_id}"))
            })?;
            let request = requests
                .get(item.source_request_id.as_str())
                .copied()
                .ok_or_else(|| {
                    Error::Contract(format!(
                        "dangling asset request: {}",
                        item.source_request_id
                    ))
                })?;
            if item.kind != narrative_courier_contracts::AssetKind::Image
                || !matches!(
                    item.mime.as_str(),
                    "image/png" | "image/jpeg" | "image/webp"
                )
            {
                return Err(Error::Contract(format!(
                    "UNSUPPORTED_CAPABILITY: HTML cannot represent asset {asset_id} {:?} {}",
                    item.kind, item.mime
                )));
            }
            let bytes = read_materialized_image(project, item)?;
            media.push_str(&format!(
                "<figure data-asset=\"{}\"><img alt=\"{}\" src=\"data:{};base64,{}\"></figure>\n",
                html_escape(asset_id),
                html_escape(&request.role),
                html_escape(&item.mime),
                base64_encode(&bytes)
            ));
        }
        body.push_str(&format!(
            "<section data-unit=\"{}\">{}<pre>{}</pre></section>\n",
            html_escape(&unit.unit_id),
            media,
            html_escape(unit.content.as_deref().unwrap_or(""))
        ));
    }
    Ok(format!(
        r#"<!doctype html>
<html><head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>NarrativeCourier Delivery</title>
<style>
:root{{font-family:ui-monospace,SFMono-Regular,Menlo,monospace;color:#171717;background:#fff}}
body{{margin:0 auto;max-width:960px;padding:40px 28px}}
section{{border-bottom:1px solid #ddd;padding:0 0 28px;margin:0 0 28px}}
figure{{margin:0 0 20px}}img{{display:block;max-width:100%;height:auto}}
pre{{white-space:pre-wrap;overflow-wrap:anywhere;font:inherit;font-size:16px;line-height:1.65;margin:0}}
@media screen and (max-width:600px){{body{{padding:24px 16px}}}}
@media print{{body{{max-width:none;padding:0}}section{{break-inside:avoid}}}}
</style>
</head><body><main>
{}
</main></body></html>"#,
        body
    ))
}

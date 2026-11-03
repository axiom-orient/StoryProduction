//! Exact output admission and unambiguous backend selection.

use super::types::{validate, BackendDescriptor, BackendOperation};
use narrative_courier_contracts::OutputRequest;
use narrative_courier_runtime::{Error, Result};

fn supports(output: &OutputRequest, descriptor: &BackendDescriptor) -> bool {
    if !descriptor.operations.contains(&BackendOperation::Render)
        || !descriptor.formats.contains(&output.format)
    {
        return false;
    }
    if let Some(multiple) = descriptor.limits.width_multiple {
        if !output
            .width
            .is_some_and(|width| width > 0 && width % multiple == 0)
        {
            return false;
        }
    }
    if let Some(multiple) = descriptor.limits.height_multiple {
        if !output
            .height
            .is_some_and(|height| height > 0 && height % multiple == 0)
        {
            return false;
        }
    }
    true
}

pub fn route_output(
    output: &OutputRequest,
    descriptors: &[BackendDescriptor],
) -> Result<BackendDescriptor> {
    let mut ids = std::collections::BTreeSet::new();
    for descriptor in descriptors {
        validate(descriptor)?;
        if !ids.insert(descriptor.backend_id.as_str()) {
            return Err(Error::Contract(
                "AMBIGUOUS_BACKEND: duplicate backend registration".into(),
            ));
        }
    }
    if let Some(id) = output.backend_id.as_deref() {
        let backend = descriptors
            .iter()
            .find(|d| d.backend_id == id)
            .ok_or_else(|| {
                Error::Contract(format!(
                    "UNSUPPORTED_CAPABILITY: backend not available: {id}"
                ))
            })?;
        if !supports(output, backend) {
            return Err(Error::Contract(format!(
                "UNSUPPORTED_CAPABILITY: {id} cannot satisfy {} {:?} with requested limits",
                output.output_id, output.format
            )));
        }
        return Ok(backend.clone());
    }
    let mut first_compatible = None;
    let mut compatible_count = 0_usize;
    for descriptor in descriptors {
        if supports(output, descriptor) {
            compatible_count += 1;
            if first_compatible.is_none() {
                first_compatible = Some(descriptor);
            }
        }
    }
    if compatible_count > 1 {
        return Err(Error::Contract(format!(
            "AMBIGUOUS_BACKEND: {} compatible backends for {:?}; pin backend_id",
            compatible_count, output.format
        )));
    }
    first_compatible.cloned().ok_or_else(|| {
        Error::Contract(format!(
            "UNSUPPORTED_CAPABILITY: no backend for {} {:?} with requested limits",
            output.output_id, output.format
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::super::types::BackendLimits;
    use super::{route_output, BackendDescriptor, BackendOperation};
    use narrative_courier_contracts::{OutputFormat, OutputRequest};

    fn html_output() -> OutputRequest {
        OutputRequest {
            output_id: "preview".into(),
            format: OutputFormat::Html,
            backend_id: None,
            width: None,
            height: None,
            aspect_ratio: None,
            fps: None,
            duration_frames: None,
            constraints: vec![],
        }
    }

    fn descriptor(operation: BackendOperation, limits: BackendLimits) -> BackendDescriptor {
        let mut descriptor = BackendDescriptor {
            backend_id: "test-backend".into(),
            version: "test".into(),
            fingerprint: String::new(),
            operations: vec![operation],
            formats: vec![OutputFormat::Html],
            limits,
        };
        descriptor.fingerprint =
            super::super::types::fingerprint(&descriptor).expect("fixture descriptor fingerprint");
        descriptor
    }

    #[test]
    fn render_routing_rejects_materialize_only_backends() {
        let result = route_output(
            &html_output(),
            &[descriptor(
                BackendOperation::Materialize,
                BackendLimits::default(),
            )],
        );
        assert!(result
            .expect_err("materialize-only backend cannot render")
            .to_string()
            .contains("no backend"));
    }

    #[test]
    fn render_routing_rejects_zero_dimension_multiples() {
        let output = OutputRequest {
            width: Some(1280),
            ..html_output()
        };
        let result = route_output(
            &output,
            &[descriptor(
                BackendOperation::Render,
                BackendLimits {
                    width_multiple: Some(0),
                    height_multiple: None,
                },
            )],
        );
        assert!(result
            .expect_err("zero multiple must fail capability admission")
            .to_string()
            .contains("zero dimension multiple"));
    }
}

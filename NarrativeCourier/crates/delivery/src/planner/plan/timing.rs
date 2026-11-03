//! Exact deterministic frame allocation shared by plan construction and preflight.

use narrative_courier_contracts::{OutputFormat, OutputRequest};
use narrative_courier_runtime::{Error, Result};

#[derive(Clone, Copy)]
pub(in crate::planner) enum FrameAllocation {
    Static,
    Timed { base: u32, remainder: u32 },
}

impl FrameAllocation {
    pub(in crate::planner) fn for_output(output: &OutputRequest, units: u32) -> Result<Self> {
        if units == 0 {
            return Err(Error::Contract(
                "delivery requires at least one selected scene".into(),
            ));
        }
        if !matches!(output.format, OutputFormat::Mp4 | OutputFormat::Webm) {
            return Ok(Self::Static);
        }
        let total = output.duration_frames.ok_or_else(|| {
            Error::Contract(format!(
                "{} video requires duration_frames",
                output.output_id
            ))
        })?;
        if total < units {
            return Err(Error::Contract(format!(
                "{} duration_frames is smaller than selected scene count",
                output.output_id
            )));
        }
        Ok(Self::Timed {
            base: total / units,
            remainder: total % units,
        })
    }

    pub(in crate::planner) fn duration(self, order: u32) -> Option<u32> {
        match self {
            Self::Static => None,
            Self::Timed { base, remainder } => Some(base + u32::from(order < remainder)),
        }
    }
}

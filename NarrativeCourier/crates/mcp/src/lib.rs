//! NarrativeCourier MCP crate facade.
//!
//! Tool schemas, engine adapters, root configuration, and stdio startup remain
//! separate owners behind the established public StsMcp and serve_stdio paths.

mod params;
mod roots;
mod server;
mod tools;

pub use params::{
    ExportScreenplayParams, ImportScreenplayParams, ProjectParams, ReviewedSynopsisParams,
    RunDeliveryParams, ScreenplayFormat,
};
pub use roots::resolve_roots;
pub use server::serve_stdio;
pub use tools::StsMcp;

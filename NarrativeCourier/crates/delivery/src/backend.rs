//! Backend discovery, descriptor integrity and output routing owners.

mod discovery;
mod routing;
mod types;

pub use discovery::describe_backends;
pub use routing::route_output;
pub use types::{BackendDescriptor, BackendLimits, BackendOperation};

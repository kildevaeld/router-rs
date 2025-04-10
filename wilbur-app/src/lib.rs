mod app;
mod body;
mod context;
mod error;

pub use self::{app::App, body::Body, context::*, error::Error};

pub use wilbur_core as core;
pub use wilbur_routing as router;

pub use wilbur_core::{handler, middleware};

pub mod prelude {
    pub use wilbur_container::{Container, ReadableContainer};
    pub use wilbur_routing::{Routing, RoutingExt};
}

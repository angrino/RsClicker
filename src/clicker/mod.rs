mod engine;
pub mod types;

pub use engine::ClickEngine;
pub use types::{
    ClickConfig, ClickKind, ClickTarget, InjectionBackend, MAX_INTERVAL_MS, MouseButtonKind,
};

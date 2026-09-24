//! 🔥 Flame simulation engine.
//!
//! Pure logic — no platform dependencies, no rendering.
//! The `FireStateMachine` owns intensity / fuel / ember heat,
//! and the `FireEngine` turns those into a smooth RGBA flame.

pub mod atlas;
pub mod compositor;
pub mod engine;
pub mod palette;
pub mod state_machine;

pub use atlas::PixelCampfireAtlas;
pub use compositor::compose_campfire_frame;
pub use engine::FireEngine;
pub use palette::{FlameColorMix, SourceFlameColors};
pub use state_machine::{
    FirePhase, FirePreviewStyle, FireSnapshot, FireStateMachine, FireTier, FlameSize,
};

// Re-export render dimensions for convenience.
pub use engine::{CANVAS_PADDING, FIRE_HEIGHT, FIRE_WIDTH, LOG_HEIGHT, LOG_WIDTH};

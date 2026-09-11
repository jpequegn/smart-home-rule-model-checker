//! Pure offline, bounded automation analysis. No IO or wall-clock access.
pub const FORMAT_VERSION: u32 = 1;
mod model;
pub use model::*;
mod engine;
pub use engine::*;
mod lint;
pub use lint::*;

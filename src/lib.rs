//! Testable calculator logic shared by the GPUI Kit UI.

pub mod calc;
pub mod theme;

pub use calc::{evaluate_tokens, format_number, parse_pasted_number, Calculator, Key};
pub use theme::{parse_hex_color, OmarchyPalette, RgbaColor};

// HTTP 1.1 compliant server built by Raresreras
// Declares the types and submodules

pub mod types;
pub mod parser;

pub use types::{http_methods, http_request};
pub use parser::http_parser;
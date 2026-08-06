// HTTP 1.1 compliant server built by Raresreras
// Declares the types and submodules

pub mod types;
pub mod parser;
pub mod handler;

pub use types::{http_methods, http_request, http_header, http_response, http_status};
pub use parser::http_parser;
pub use handler::{router};
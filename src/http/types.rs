// HTTP 1.1 compliant server built by Raresreras
// Types File containing structs and enums

// We will see how many we manage to import	
#[derive(Debug)]
pub enum http_methods {
	GET,
	HEAD,
	POST,
	PUT,
	DELETE,
	CONNECT,
	OPTIONS,
	TRACE,
	PATCH,
	NONE, // used as default
}

impl Default for http_methods {
	fn default() -> Self {
		http_methods::NONE // arbitrary — this is the awkward part
	}
}

#[derive(Debug)]
pub struct http_header {
	pub header_name: String,
	pub header_value: String,
}

#[derive(Default, Debug)]
pub struct http_request {
	pub method: http_methods,
	pub path: String,
	// TODO! Add query here
	pub version: String,
	pub header_vector: Vec<http_header>,
	pub body: Vec<u8>,
}
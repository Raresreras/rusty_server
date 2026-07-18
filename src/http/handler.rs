// HTTP 1.1 compliant server built by Raresreras
// HTTP handler file

use super::types::*;
use std::collections::HashMap;

// Routing table

pub struct router {
	routes: HashMap<(http_methods, String), route_func>,
}

impl router {
	// Constructor
	pub fn new() -> Self {
		router { routes: HashMap::new() }
	}

	// Adds entries into the routing table
	pub fn add_route(&mut self, method: http_methods, path: &str, handler: route_func) {
		self.routes.insert((method, path.to_string()), handler);
	}

	// Gets route from the table, returning status and body
	pub fn handle(&self, input: &http_request) -> (http_status, Vec<u8>) {
		match self.routes.get(&(input.method.clone(), input.path.clone())) {
			Some(handler) => handler(input),
			None => (http_status::Not_Found, b"404 Not Found".to_vec()),
		}
	}
}
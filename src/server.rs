// HTTP 1.1 compliant server built by Raresreras

use crate::http::*;
use crate::http::types::*;
use crate::http::parser::http_parser;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::thread::JoinHandle;

pub struct http_server {
	http_router: router,
	running: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl http_server {
	// Constructor
	pub fn new() -> Self {
		http_server {
			http_router: router::new(),
			running: Arc::new(AtomicBool::new(false)),
			handle: None,
		}
	}

	// Adding route to the http_router
	pub fn add_route(&mut self, method: http_methods, path: &str, handler: route_func) {
		self.http_router.add_route(method, path, handler);
	}

	// Starts the server
	pub fn run() {

	}

	// Closes the server
	pub fn close() {}
}
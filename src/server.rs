// HTTP 1.1 compliant server built by Raresreras

use crate::http::*;
use crate::http::types::*;
use crate::http::parser::http_parser;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;
use std::net::{TcpListener, TcpStream};
use std::time::Duration;
use std::io::{Read, Write};

// Main struct containing:
//	- router
//	- running status
//	- thread handle 
pub struct http_server {
	http_router: Arc<Mutex<router>>,
	running: Arc<AtomicBool>,
	handle: Option<JoinHandle<()>>,
}

// Main implementation
impl http_server {
	// Constructor
	pub fn new() -> Self {
		http_server {
			http_router: Arc::new(Mutex::new(router::new())),
			running: Arc::new(AtomicBool::new(false)),
			handle: None,
		}
	}

	// Adding route to the http_router
	pub fn add_route(&mut self, method: http_methods, path: &str, handler: route_func) {
		self.http_router
			.lock()
			.unwrap()
			.add_route(method, path, handler);
	}

	// Starts the server
	pub fn run(&mut self, addr: &str) {
		// Setting the running flag to true
		self.running.store(true, Ordering::SeqCst);

		// Clones for threads
		let running_clone = self.running.clone();
		let addr_clone = addr.to_string();
		let http_router = Arc::clone(&self.http_router);

		// Main thread
		self.handle = Some(thread::spawn(move || {
			// Opening the TCP port
			let listener = TcpListener::bind(&addr_clone)
				.expect("Failed to bind to address");			

			println!("Server listening on {}", &addr_clone);

			// Setting the connection to non-blocking
			// so we can close it outside the thread
			listener.set_nonblocking(true).expect("Failed to set non-blocking");

			// Main loop
			while running_clone.load(Ordering::SeqCst) {
				match listener.accept() {
					Ok((mut stream, addr)) => {
						// Logging
						println!("New connection: {}", addr);
						println!("{:?}", stream);

						// Extra http_router clone for the next thread
						let http_router = Arc::clone(&http_router);
						
						// Per request thread
						let process_handle = thread::spawn(move || {
							let mut buffer = &mut [0u8; 1024];

							// Listening + parsing
							// TODO! use res for detecting errors

							let res = stream.read(buffer).unwrap_or(0);
							let parsed_request = http_parser(buffer);
							// println!("{:?}", parsed_request);

							// Router
							let response = http_router
								.lock()
								.unwrap()
								.handle(&parsed_request.unwrap());

							println!("{:?}", response);
							//stream.write(response);
						});
					}
					Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
						// No connection available so we sleep the cpu 100ms
						thread::sleep(Duration::from_millis(100));
					}
					Err(e) => {
						// Proper error handling here would be useful
						println!("{:?}", e);
					}
				}
			}
		}));
	}

	// Closes the server
	pub fn close(&mut self) {
		// Setting the running flag to false, which kills the server thread
		self.running.store(false, Ordering::SeqCst);

		// Joining the handle to get the thread data
		if let Some(handle) = self.handle.take() {
			handle.join().unwrap();
		}
	}
}
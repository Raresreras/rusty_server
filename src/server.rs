// HTTP 1.1 compliant server built by Raresreras

use crate::http::*;
use crate::http::types::*;
use crate::http::parser::http_parser;

use std::sync::atomic::{AtomicBool, Ordering, AtomicUsize};
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;
use std::net::{TcpListener, TcpStream};
use std::time::Duration;
use std::io::{Read, Write, BufReader, BufRead, BufWriter, ErrorKind, Error};
use std::io;

// Main struct containing:
//	- router
//	- running status
//	- thread handle 
// - number of active connections
pub struct http_server {
	http_router: Arc<Mutex<router>>,
	running: Arc<AtomicBool>,
	handle: Option<JoinHandle<()>>,
	active_connections: Arc<AtomicUsize>
}

// TODO! move these to a settings struct so they arent hardcoded anymore
// Maximum header size
const MAX_HEADER_BYTES: usize = 16 * 1024;
const MAX_CONNECTIONS: usize = 1024;
const TIMEOUT: Duration = Duration::from_secs(10);

// Http header reading function
fn read_headers(reader: &mut BufReader<&mut TcpStream>) -> io::Result<Vec<u8>> {
	let mut buffer = Vec::new();

	loop {
		let start = buffer.len();
		let remaining = MAX_HEADER_BYTES.saturating_sub(buffer.len());

		if remaining == 0 {
			return Err(Error::new(ErrorKind::InvalidData, "headers too large"));
		}

		let n = reader
			.by_ref()
			.take(remaining as u64)
			.read_until(b'\n', &mut buffer)?;

		if n == 0 {
			return Err(Error::new(
                ErrorKind::UnexpectedEof,
                "connection closed before end of headers",
            ));
		}

		let line = &buffer[start..];
        if line == b"\r\n" || line == b"\n" {
            return Ok(buffer);
        }        
	}
} 

// Main implementation
impl http_server {
	// Constructor
	pub fn new() -> Self {
		http_server {
			http_router: Arc::new(Mutex::new(router::new())),
			running: Arc::new(AtomicBool::new(false)),
			handle: None,
			active_connections: Arc::new(AtomicUsize::new(0))
		}
	}

	// Adding route to the http_router
	pub fn add_route(&mut self, method: http_methods, path: &str, handler: route_func) {
		self.http_router
			.lock()
			.unwrap_or_else(|e| e.into_inner())
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
		let active_connections = Arc::clone(&self.active_connections);

		// Main thread
		self.handle = Some(thread::spawn(move || {
			
			// Opening the TCP port
			let listener = match TcpListener::bind(&addr_clone)  {
				Ok(listener) => listener,
				Err(e) => {
					eprintln!("Error: {}", e);
					return ();
				}
			};

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

						// Timeouts
						stream.set_read_timeout(Some(TIMEOUT)).ok();
						stream.set_write_timeout(Some(TIMEOUT)).ok();

						// Connection limit checking
						if active_connections.load(Ordering::SeqCst) > MAX_CONNECTIONS {
							continue;
						}

						// Adding the current connection
						active_connections.fetch_add(1, Ordering::SeqCst);

						// Extra http_router clone for the next thread
						let http_router = Arc::clone(&http_router);
						
						// Per request thread
						let process_handle = thread::spawn(move || {

							let mut reader = BufReader::new(&mut stream);

							// Router
							let response = match read_headers(&mut reader) {
								Ok(buffer) => match http_parser(&buffer) {
									Some(request) => {
										// Lock is released at the end of this statement
										let handler = http_router
											.lock()
											.unwrap_or_else(|poisoned| poisoned.into_inner())
											.handle(&request);

										// Mutex lock is dropped here
										match handler {
											Some(handler) => handler(&request),
											None => http_response::error(),
										}
									}
									None => http_response::empty(http_status::Bad_Request),
								},
								Err(_) => http_response::empty(http_status::Bad_Request),
							};

							// Writer
							if let Err(e) = stream.write_all(&response.to_bytes()) {
								eprintln!("Error: {}", e);
							};
						});

						// Substracting the current connection
							active_connections.fetch_sub(1, Ordering::SeqCst);
					}

					Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
						// No connection available
						thread::sleep(Duration::from_millis(1));
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
	// HTTP 1.1 compliant server built by Raresreras
// Rust seems to be fun - 11/07/2026
// Send help - 6/08/2026

// TODO! Delete this later
#![allow(warnings)]

mod http;
mod server;
use crate::http::*;
use crate::server::*;

fn main() {
	let test_request = "GET / HTTP/1.1\r\nHost: localhost\r\nUser-Agent: test-client\r\nAccept: */*\r\nConnection: close\r\n\r\n".to_string();

	let mut server = http_server::new();

	server.add_route(http_methods::GET, "/", |_request| {
		let mut response = http_response::empty(http_status::OK);
		response.body = "Hello world".to_string().into_bytes();
		response
	});

	server.add_route(http_methods::GET, "/query", |_request| {
		let mut response = http_response::empty(http_status::OK);
		let text = match _request.query_vector.get(0) {
			Some(text) => text.clone(),
			None => "no query params".to_string(),
		};
		response.body = text.into_bytes();		
		response
	});

	server.run("127.0.0.1:6767");

	// Ctrl+C based keep-alive
	let (tx, rx) = std::sync::mpsc::channel();
	ctrlc::set_handler(move || tx.send(()).expect("Could not send signal"))
		.expect("Error setting Ctrl+C handler");

	println!("Press Ctrl+C to stop the server");
	rx.recv().expect("Could not receive ctrl+c signal");

	println!("\nClosing server");
	server.close();
}
// HTTP 1.1 compliant server built by Raresreras
// Rust seems to be fun - 11/07/2026

mod http;

use http::http_parser;

fn main() {
	let test_request = "GET /index.html HTTP/1.1\r\nHost: localhost\r\nUser-Agent: test-client\r\nAccept: */*\r\nConnection: close\r\n\r\n".to_string();

	let test_http_parsed = http_parser(test_request.as_bytes()).unwrap();

	println!("{:#?}", test_http_parsed);
}
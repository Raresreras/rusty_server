// HTTP 1.1 compliant server built by Raresreras
// Rust seems to be fun - 11/07/2026

// TODO! Delete this later
#![allow(warnings)]

mod http;
mod server;

fn main() {
	let test_request = "GET / HTTP/1.1\r\nHost: localhost\r\nUser-Agent: test-client\r\nAccept: */*\r\nConnection: close\r\n\r\n".to_string();

	/*let test_http_parsed = http_parser(test_request.as_bytes()).unwrap();

	let mut http_router = router::new();

	http_router.add_route(http_methods::GET, "/", |_request| {
		(http_status::OK, "Hello world!".into())
	});

	let response = http_router.handle(&test_http_parsed);

	println!("{:#?}", response);
	*/
}
	// HTTP 1.1 compliant server built by Raresreras
// Rust seems to be fun - 11/07/2026
// Send help - 6/08/2026

// TODO! Delete this later
#![allow(warnings)]

use rusty_server::http::*;
use rusty_server::server::*;

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

// ---------------------------------------------------------------------------
// Unit tests
//
// Run with:
//     cargo test
//
// The tests use the library facade so they exercise the same parser, router
// and response code the server uses. A couple of helpers start the real server
// on an ephemeral port for end-to-end socket tests.
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
	use std::io::{Read, Write};
	use std::net::{Shutdown, SocketAddr, TcpStream};
	use std::thread;
	use std::time::{Duration, Instant};

	use rusty_server::http::{
		http_header, http_methods, http_parser, http_request, http_response, http_status,
		router,
	};
	use rusty_server::server::http_server;

	// ---- test routes -----------------------------------------------------

	fn hello(_request: &http_request) -> http_response {
		let mut response = http_response::empty(http_status::OK);
		response.body = b"Hello world".to_vec();
		response
	}

	fn query(request: &http_request) -> http_response {
		let mut response = http_response::empty(http_status::OK);
		let text = request
			.query_vector
			.first()
			.cloned()
			.unwrap_or_else(|| "no query params".to_string());
		response.body = text.into_bytes();
		response
	}

	fn first(_request: &http_request) -> http_response {
		let mut response = http_response::empty(http_status::OK);
		response.body = b"first".to_vec();
		response
	}

	fn second(_request: &http_request) -> http_response {
		let mut response = http_response::empty(http_status::OK);
		response.body = b"second".to_vec();
		response
	}

	fn created(_request: &http_request) -> http_response {
		let mut response = http_response::empty(http_status::Created);
		response.body = b"created".to_vec();
		response
	}

	fn echo(request: &http_request) -> http_response {
		let mut response = http_response::empty(http_status::OK);
		response.body = request.body.clone();
		response
	}

	fn with_header(_request: &http_request) -> http_response {
		let mut response = http_response::empty(http_status::OK);
		response.body = b"with header".to_vec();
		response.header_vector.push(http_header {
			header_name: "X-Test".to_string(),
			header_value: "yes".to_string(),
		});
		response
	}

	fn no_content(_request: &http_request) -> http_response {
		http_response::empty(http_status::No_Content)
	}

	fn head_hello(_request: &http_request) -> http_response {
		let mut response = http_response::empty(http_status::OK);
		response.body = b"Hello world".to_vec();
		response
	}

	fn slow(_request: &http_request) -> http_response {
		thread::sleep(Duration::from_millis(150));
		hello(_request)
	}

	// ---- socket helpers --------------------------------------------------

	fn start_server() -> (http_server, SocketAddr) {
		let mut server = http_server::new();
		server.set_verbose(false);
		server.add_route(http_methods::GET, "/", hello);
		server.add_route(http_methods::GET, "/query", query);
		server.add_route(http_methods::POST, "/echo", echo);
		server.add_route(http_methods::GET, "/headers", with_header);
		server.add_route(http_methods::GET, "/no-content", no_content);
		server.add_route(http_methods::HEAD, "/head", head_hello);
		server.add_route(http_methods::GET, "/slow", slow);
		server.run("127.0.0.1:0");

		let addr = server.local_addr().expect("server should bind a port");
		(server, addr)
	}

	fn raw_request(addr: SocketAddr, raw: &[u8]) -> String {
		let mut stream = TcpStream::connect(addr).expect("connect failed");
		stream
			.set_read_timeout(Some(Duration::from_secs(2)))
			.expect("failed to set read timeout");
		stream.write_all(raw).expect("write failed");
		stream.shutdown(Shutdown::Write).expect("shutdown failed");

		let mut response = Vec::new();
		match stream.read_to_end(&mut response) {
			Ok(_) => {}
			// The server may close with unread request bytes (for example when
			// rejecting oversized headers), which turns the close into a RST.
			// Keep whatever response bytes arrived before the reset.
			Err(error) if error.kind() == std::io::ErrorKind::ConnectionReset => {}
			Err(error) => panic!("read failed: {error}"),
		}
		String::from_utf8_lossy(&response).into_owned()
	}

	fn get(addr: SocketAddr, path: &str) -> String {
		let raw = format!(
			"GET {path} HTTP/1.1\r\nHost: localhost\r\nUser-Agent: test\r\nConnection: close\r\n\r\n"
		);
		raw_request(addr, raw.as_bytes())
	}

	fn request(method: http_methods, path: &str) -> http_request {
		http_request {
			method,
			path: path.to_string(),
			..Default::default()
		}
	}

	// ---- parser ----------------------------------------------------------

	#[test]
	fn parser_reads_request_line() {
		let parsed = http_parser(b"GET /hello HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();

		assert_eq!(parsed.method, http_methods::GET);
		assert_eq!(parsed.path, "/hello");
		assert_eq!(parsed.version, "HTTP/1.1");
	}

	#[test]
	fn parser_splits_query_string() {
		let parsed =
			http_parser(b"GET /query?a=1&b=two&c= HTTP/1.1\r\nHost: x\r\n\r\n").unwrap();

		assert_eq!(parsed.path, "/query");
		assert_eq!(parsed.query_vector, vec!["a=1", "b=two", "c="]);
	}

	#[test]
	fn parser_keeps_empty_query_parts() {
		let parsed = http_parser(b"GET /query?a=1&&b=2 HTTP/1.1\r\nHost: x\r\n\r\n").unwrap();

		assert_eq!(parsed.query_vector, vec!["a=1", "", "b=2"]);
	}

	#[test]
	fn parser_handles_query_marker_without_value() {
		let parsed = http_parser(b"GET /query? HTTP/1.1\r\nHost: x\r\n\r\n").unwrap();

		assert_eq!(parsed.path, "/query");
		assert_eq!(parsed.query_vector, vec![""]);
	}

	#[test]
	fn parser_leaves_query_empty_when_absent() {
		let parsed = http_parser(b"GET /query HTTP/1.1\r\nHost: x\r\n\r\n").unwrap();

		assert_eq!(parsed.path, "/query");
		assert!(parsed.query_vector.is_empty());
	}

	#[test]
	fn parser_does_not_decode_percent_escapes() {
		let parsed =
			http_parser(b"GET /query?name=hello%20world HTTP/1.1\r\nHost: x\r\n\r\n").unwrap();

		assert_eq!(parsed.query_vector, vec!["name=hello%20world"]);
	}

	#[test]
	fn parser_trims_header_name_and_value() {
		let parsed =
			http_parser(b"GET / HTTP/1.1\r\n  X-Test  :   spaced value   \r\n\r\n").unwrap();

		assert_eq!(parsed.header_vector.len(), 1);
		assert_eq!(parsed.header_vector[0].header_name, "X-Test");
		assert_eq!(parsed.header_vector[0].header_value, "spaced value");
	}

	#[test]
	fn parser_keeps_colons_inside_header_values() {
		let parsed = http_parser(b"GET / HTTP/1.1\r\nHost: localhost:6767\r\n\r\n").unwrap();

		assert_eq!(parsed.header_vector.len(), 1);
		assert_eq!(parsed.header_vector[0].header_value, "localhost:6767");
	}

	#[test]
	fn parser_skips_header_lines_without_colon() {
		let parsed =
			http_parser(b"GET / HTTP/1.1\r\nBogusHeader\r\nHost: x\r\n\r\n").unwrap();

		assert_eq!(parsed.header_vector.len(), 1);
		assert_eq!(parsed.header_vector[0].header_name, "Host");
	}

	#[test]
	fn parser_keeps_duplicate_headers_in_order() {
		let parsed =
			http_parser(b"GET / HTTP/1.1\r\nX-Test: one\r\nX-Test: two\r\n\r\n").unwrap();

		assert_eq!(parsed.header_vector.len(), 2);
		assert_eq!(parsed.header_vector[0].header_value, "one");
		assert_eq!(parsed.header_vector[1].header_value, "two");
	}

	#[test]
	fn parser_reads_body() {
		let parsed =
			http_parser(b"POST /echo HTTP/1.1\r\nHost: x\r\nContent-Length: 5\r\n\r\nhello")
				.unwrap();

		assert_eq!(parsed.method, http_methods::POST);
		assert_eq!(parsed.body, b"hello");
	}

	#[test]
	fn parser_keeps_binary_body() {
		let parsed = http_parser(b"POST /echo HTTP/1.1\r\nHost: x\r\n\r\n\x00\xff\x01").unwrap();

		assert_eq!(parsed.body, vec![0x00, 0xff, 0x01]);
	}

	#[test]
	fn parser_rejects_missing_header_terminator() {
		assert!(http_parser(b"GET / HTTP/1.1\r\nHost: localhost\r\n").is_none());
		assert!(http_parser(b"GET / HTTP/1.1").is_none());
		assert!(http_parser(b"").is_none());
	}

	#[test]
	fn parser_rejects_lf_only_requests() {
		assert!(http_parser(b"GET / HTTP/1.1\nHost: localhost\n\n").is_none());
	}

	#[test]
	fn parser_rejects_invalid_utf8() {
		assert!(http_parser(b"GET / HTTP/1.1\r\nX-Binary: \xff\xfe\r\n\r\n").is_none());
	}

	#[test]
	fn parser_rejects_missing_path_or_version() {
		assert!(http_parser(b"GET\r\n\r\n").is_none());
		assert!(http_parser(b"GET /\r\n\r\n").is_none());
	}

	#[test]
	fn parser_maps_all_known_methods() {
		let cases = [
			("GET", http_methods::GET),
			("HEAD", http_methods::HEAD),
			("POST", http_methods::POST),
			("PUT", http_methods::PUT),
			("DELETE", http_methods::DELETE),
			("CONNECT", http_methods::CONNECT),
			("OPTIONS", http_methods::OPTIONS),
			("TRACE", http_methods::TRACE),
			("PATCH", http_methods::PATCH),
		];

		for (method, expected) in cases {
			let raw = format!("{method} / HTTP/1.1\r\nHost: x\r\n\r\n");
			let parsed = http_parser(raw.as_bytes()).unwrap();
			assert_eq!(parsed.method, expected, "method {method}");
		}
	}

	#[test]
	fn parser_unknown_method_is_none() {
		for method in ["BREW", "get", "PoSt", ""] {
			let raw = format!("{method} / HTTP/1.1\r\nHost: x\r\n\r\n");
			let parsed = http_parser(raw.as_bytes()).unwrap();
			assert_eq!(parsed.method, http_methods::NONE, "method {method:?}");
		}
	}

	#[test]
	fn parser_accepts_any_version_string() {
		let parsed = http_parser(b"GET / HTTP/9.9\r\nHost: x\r\n\r\n").unwrap();

		assert_eq!(parsed.version, "HTTP/9.9");
	}

	#[test]
	fn parser_handles_request_line_only() {
		let parsed = http_parser(b"GET / HTTP/1.1\r\n\r\n").unwrap();

		assert!(parsed.header_vector.is_empty());
		assert!(parsed.body.is_empty());
	}

	// ---- router ----------------------------------------------------------

	#[test]
	fn router_starts_empty() {
		let router = router::new();

		assert!(router.handle(&request(http_methods::GET, "/")).is_none());
	}

	#[test]
	fn router_resolves_exact_method_and_path() {
		let mut router = router::new();
		router.add_route(http_methods::GET, "/", first);

		let req = request(http_methods::GET, "/");
		let handler = router.handle(&req).expect("route should exist");
		let response = handler(&req);

		assert_eq!(response.status, 200);
		assert_eq!(response.body, b"first");
	}

	#[test]
	fn router_requires_matching_method() {
		let mut router = router::new();
		router.add_route(http_methods::GET, "/", first);

		assert!(router.handle(&request(http_methods::POST, "/")).is_none());
		assert!(router.handle(&request(http_methods::HEAD, "/")).is_none());
	}

	#[test]
	fn router_paths_are_case_sensitive() {
		let mut router = router::new();
		router.add_route(http_methods::GET, "/Hello", first);

		assert!(router.handle(&request(http_methods::GET, "/Hello")).is_some());
		assert!(router.handle(&request(http_methods::GET, "/hello")).is_none());
	}

	#[test]
	fn router_trailing_slash_is_a_different_path() {
		let mut router = router::new();
		router.add_route(http_methods::GET, "/hello", first);

		assert!(router.handle(&request(http_methods::GET, "/hello/")).is_none());
		assert!(router.handle(&request(http_methods::GET, "/hello")).is_some());
	}

	#[test]
	fn router_supports_same_path_with_different_methods() {
		let mut router = router::new();
		router.add_route(http_methods::GET, "/resource", first);
		router.add_route(http_methods::POST, "/resource", created);

		let get_req = request(http_methods::GET, "/resource");
		let get_response = router.handle(&get_req).unwrap()(&get_req);
		assert_eq!(get_response.status, 200);
		assert_eq!(get_response.body, b"first");

		let post_req = request(http_methods::POST, "/resource");
		let post_response = router.handle(&post_req).unwrap()(&post_req);
		assert_eq!(post_response.status, 201);
		assert_eq!(post_response.body, b"created");
	}

	#[test]
	fn router_later_registration_replaces_handler() {
		let mut router = router::new();
		router.add_route(http_methods::GET, "/", first);
		router.add_route(http_methods::GET, "/", second);

		let req = request(http_methods::GET, "/");
		let response = router.handle(&req).unwrap()(&req);

		assert_eq!(response.body, b"second");
	}

	#[test]
	fn router_empty_path_is_valid() {
		let mut router = router::new();
		router.add_route(http_methods::GET, "", first);

		assert!(router.handle(&request(http_methods::GET, "")).is_some());
	}

	#[test]
	fn router_does_not_match_unregistered_methods() {
		let mut router = router::new();
		router.add_route(http_methods::GET, "/", first);

		for method in [
			http_methods::PUT,
			http_methods::DELETE,
			http_methods::PATCH,
			http_methods::OPTIONS,
			http_methods::TRACE,
			http_methods::CONNECT,
			http_methods::NONE,
		] {
			assert!(
				router.handle(&request(method.clone(), "/")).is_none(),
				"unexpected match for {method:?}"
			);
		}
	}

	// ---- status codes and responses -------------------------------------

	#[test]
	fn status_codes_are_correct() {
		let cases = [
			(http_status::Continue, 100),
			(http_status::Switching_Protocols, 101),
			(http_status::OK, 200),
			(http_status::Created, 201),
			(http_status::Accepted, 202),
			(http_status::Non_Authoritative_Information, 203),
			(http_status::No_Content, 204),
			(http_status::Reset_Content, 205),
			(http_status::Partial_Content, 206),
			(http_status::Multiple_Choices, 300),
			(http_status::Moved_Permanently, 301),
			(http_status::Found, 302),
			(http_status::See_Other, 303),
			(http_status::Not_Modified, 304),
			(http_status::Use_Proxy, 305),
			(http_status::Temporary_Redirect, 307),
			(http_status::Bad_Request, 400),
			(http_status::Unauthorized, 401),
			(http_status::Payment_Required, 402),
			(http_status::Forbidden, 403),
			(http_status::Not_Found, 404),
			(http_status::Method_Not_Allowed, 405),
			(http_status::Not_Acceptable, 406),
			(http_status::Proxy_Authentication_Required, 407),
			(http_status::Request_Timeout, 408),
			(http_status::Conflict, 409),
			(http_status::Gone, 410),
			(http_status::Length_Required, 411),
			(http_status::Precondition_Failed, 412),
			(http_status::Request_Entity_Too_Large, 413),
			(http_status::Request_URI_Too_Long, 414),
			(http_status::Unsupported_Media_Type, 415),
			(http_status::Requested_Range_Not_Satisfiable, 416),
			(http_status::Expectation_Failed, 417),
			(http_status::Internal_Server_Error, 500),
			(http_status::Not_Implemented, 501),
			(http_status::Bad_Gateway, 502),
			(http_status::Service_Unavailable, 503),
			(http_status::Gateway_Timeout, 504),
			(http_status::HTTP_Version_Not_Supported, 505),
		];

		for (status, expected) in cases {
			assert_eq!(status.code(), expected, "wrong code for {status:?}");
		}
	}

	#[test]
	fn defaults_are_sane() {
		assert_eq!(http_status::default().code(), 200);
		assert_eq!(http_methods::default(), http_methods::NONE);

		let req = http_request::default();
		assert_eq!(req.method, http_methods::NONE);
		assert!(req.path.is_empty());
		assert!(req.version.is_empty());
		assert!(req.query_vector.is_empty());
		assert!(req.header_vector.is_empty());
		assert!(req.body.is_empty());
	}

	#[test]
	fn empty_response_wire_format() {
		let response = http_response::empty(http_status::OK);

		assert_eq!(
			response.to_bytes(),
			b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n"
		);
	}

	#[test]
	fn response_with_body_wire_format() {
		let mut response = http_response::empty(http_status::OK);
		response.body = b"Hello world".to_vec();

		assert_eq!(
			response.to_bytes(),
			b"HTTP/1.1 200 OK\r\nContent-Length: 11\r\n\r\nHello world"
		);
	}

	#[test]
	fn content_length_counts_bytes_not_characters() {
		let mut response = http_response::empty(http_status::OK);
		response.body = "héllo".as_bytes().to_vec(); // 6 bytes, 5 characters

		let text = String::from_utf8(response.to_bytes()).unwrap();

		assert!(text.contains("Content-Length: 6\r\n"));
		assert!(text.ends_with("héllo"));
	}

	#[test]
	fn reason_phrases_are_mapped() {
		let cases = [
			(100, "Continue"),
			(101, "Switching Protocols"),
			(200, "OK"),
			(201, "Created"),
			(204, "No Content"),
			(301, "Moved Permanently"),
			(304, "Not Modified"),
			(307, "Temporary Redirect"),
			(400, "Bad Request"),
			(404, "Not Found"),
			(405, "Method Not Allowed"),
			(418, "Unknown"),
			(500, "Internal Server Error"),
			(505, "HTTP Version Not Supported"),
			(999, "Unknown"),
		];

		for (status, expected) in cases {
			let response = http_response {
				status,
				..Default::default()
			};
			assert_eq!(response.reason_from_code(), expected, "status {status}");
		}
	}

	#[test]
	fn error_response_is_404() {
		let response = http_response::error();

		assert_eq!(response.version, "HTTP/1.1");
		assert_eq!(response.status, 404);
		assert_eq!(response.reason_from_code(), "Not Found");
		assert!(response.body.is_empty());
	}

	// ---- live server over TCP -------------------------------------------

	#[test]
	fn server_binds_an_ephemeral_port() {
		let (_server, addr) = start_server();

		assert_eq!(addr.ip().to_string(), "127.0.0.1");
		assert_ne!(addr.port(), 0);
	}

	#[test]
	fn server_serves_root_route() {
		let (_server, addr) = start_server();

		let response = get(addr, "/");

		assert!(response.starts_with("HTTP/1.1 200 OK\r\n"));
		assert!(response.contains("Content-Length: 11\r\n"));
		assert!(response.ends_with("Hello world"));
	}

	#[test]
	fn server_serves_first_query_parameter() {
		let (_server, addr) = start_server();

		let response = get(addr, "/query?foo=bar&baz=qux");

		assert!(response.starts_with("HTTP/1.1 200 OK\r\n"));
		assert!(response.ends_with("foo=bar"));
	}

	#[test]
	fn server_serves_query_without_parameters() {
		let (_server, addr) = start_server();

		let response = get(addr, "/query");

		assert!(response.starts_with("HTTP/1.1 200 OK\r\n"));
		assert!(response.ends_with("no query params"));
	}

	#[test]
	fn server_serves_empty_query_marker_as_empty_body() {
		let (_server, addr) = start_server();

		let response = get(addr, "/query?");

		assert!(response.starts_with("HTTP/1.1 200 OK\r\n"));
		assert!(response.contains("Content-Length: 0\r\n"));
	}

	#[test]
	fn server_returns_404_for_unknown_path() {
		let (_server, addr) = start_server();

		let response = get(addr, "/does-not-exist");

		assert!(response.starts_with("HTTP/1.1 404 Not Found\r\n"));
	}

	#[test]
	fn server_returns_404_for_wrong_method() {
		let (_server, addr) = start_server();

		let response = raw_request(
			addr,
			b"POST / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
		);

		assert!(response.starts_with("HTTP/1.1 404 Not Found\r\n"));
	}

	#[test]
	fn server_returns_400_for_malformed_request() {
		let (_server, addr) = start_server();

		let response = raw_request(addr, b"GET / HTTP/1.1\r\nHost: localhost\r\n");

		assert!(response.starts_with("HTTP/1.1 400 Bad Request\r\n"));
	}

	#[test]
	fn server_returns_400_for_empty_request() {
		let (_server, addr) = start_server();

		let response = raw_request(addr, b"");

		assert!(response.starts_with("HTTP/1.1 400 Bad Request\r\n"));
	}

	#[test]
	fn server_handles_concurrent_requests() {
		let (_server, addr) = start_server();

		let workers: Vec<_> = (0..16)
			.map(|index| {
				thread::spawn(move || {
					if index % 2 == 0 {
						let response = get(addr, "/");
						assert!(response.ends_with("Hello world"));
					} else {
						let response = get(addr, "/query?worker=ready");
						assert!(response.ends_with("worker=ready"));
					}
				})
			})
			.collect();

		for worker in workers {
			worker.join().expect("worker thread panicked");
		}
	}

	#[test]
	fn server_handles_repeated_connections() {
		let (_server, addr) = start_server();

		for _ in 0..10 {
			let response = get(addr, "/");
			assert!(response.starts_with("HTTP/1.1 200 OK\r\n"));
		}
	}

	#[test]
	fn server_closes_after_one_response() {
		let (_server, addr) = start_server();

		let mut stream = TcpStream::connect(addr).unwrap();
		stream
			.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n")
			.unwrap();

		let mut response = Vec::new();
		stream.read_to_end(&mut response).unwrap();

		assert!(response.starts_with(b"HTTP/1.1 200"));
		assert!(response.ends_with(b"Hello world"));
	}

	#[test]
	fn server_close_stops_accepting_connections() {
		let (mut server, addr) = start_server();

		assert!(TcpStream::connect_timeout(&addr, Duration::from_millis(500)).is_ok());

		server.close();
		thread::sleep(Duration::from_millis(50));

		assert!(
			TcpStream::connect_timeout(&addr, Duration::from_millis(500)).is_err(),
			"listener should be closed"
		);
	}

	// ---- behaviour called out by TODO comments / ANALYSIS.md -------------

	#[test]
	fn server_discards_request_body() {
		// ANALYSIS.md #1: the server stops reading at the blank header line.
		let (_server, addr) = start_server();

		let response = raw_request(
			addr,
			b"POST /echo HTTP/1.1\r\nHost: localhost\r\nContent-Length: 11\r\nConnection: close\r\n\r\nhello world",
		);

		assert!(response.starts_with("HTTP/1.1 200 OK\r\n"));
		assert!(response.contains("Content-Length: 0\r\n"));
		assert!(response.ends_with("\r\n\r\n"));
	}

	#[test]
	fn server_sends_only_content_length_header() {
		// ANALYSIS.md #6: no Connection/Date/Content-Type/Server headers.
		let (_server, addr) = start_server();

		let response = get(addr, "/");

		assert!(response.contains("Content-Length: 11\r\n"));
		assert!(!response.contains("Connection:"));
		assert!(!response.contains("Content-Type:"));
		assert!(!response.contains("Date:"));
		assert!(!response.contains("Server:"));
	}

	#[test]
	fn response_header_vector_is_not_serialized_yet() {
		// types.rs: "TODO! add headers" (ANALYSIS.md #7).
		let mut response = http_response::empty(http_status::OK);
		response.header_vector.push(http_header {
			header_name: "X-Test".to_string(),
			header_value: "yes".to_string(),
		});

		let text = String::from_utf8(response.to_bytes()).unwrap();

		assert!(!text.contains("X-Test"));
		assert!(!text.contains("X-Test: yes"));
	}

	#[test]
	fn no_content_response_still_includes_content_length() {
		// ANALYSIS.md #8: Content-Length is emitted for 204.
		let (_server, addr) = start_server();

		let response = get(addr, "/no-content");

		assert!(response.starts_with("HTTP/1.1 204 No Content\r\n"));
		assert!(response.contains("Content-Length: 0\r\n"));
	}

	#[test]
	fn head_route_emits_a_body() {
		// ANALYSIS.md #10: HEAD is routed like GET and still writes a body.
		let (_server, addr) = start_server();

		let response = raw_request(
			addr,
			b"HEAD /head HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
		);

		assert!(response.starts_with("HTTP/1.1 200 OK\r\n"));
		assert!(response.ends_with("Hello world"));
	}

	#[test]
	fn http_version_is_not_validated() {
		// ANALYSIS.md #14: any version string is accepted.
		let (_server, addr) = start_server();

		let response = raw_request(
			addr,
			b"GET / HTTP/9.9\r\nHost: localhost\r\nConnection: close\r\n\r\n",
		);

		assert!(response.starts_with("HTTP/1.1 200 OK\r\n"));
		assert!(response.ends_with("Hello world"));
	}

	#[test]
	fn lf_only_request_returns_400() {
		// ANALYSIS.md #11: the socket reader accepts LF, the parser wants CRLF.
		let (_server, addr) = start_server();

		let response = raw_request(addr, b"GET / HTTP/1.1\nHost: localhost\n\n");

		assert!(response.starts_with("HTTP/1.1 400 Bad Request\r\n"));
	}

	#[test]
	fn query_is_not_percent_decoded_over_socket() {
		// parser.rs: "TODO! Add further parsing here" (ANALYSIS.md #13).
		let (_server, addr) = start_server();

		let response = get(addr, "/query?name=hello%20world");

		assert!(response.ends_with("name=hello%20world"));
	}

	#[test]
	fn keep_alive_is_not_supported_over_one_connection() {
		// ANALYSIS.md #6: the connection is closed after one response.
		let (_server, addr) = start_server();

		let mut stream = TcpStream::connect(addr).unwrap();
		stream
			.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\nConnection: keep-alive\r\n\r\n")
			.unwrap();

		let mut first = Vec::new();
		stream.read_to_end(&mut first).unwrap();
		assert!(first.starts_with(b"HTTP/1.1 200"));

		let _ = stream.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n");
		let mut second = Vec::new();
		let _ = stream.read_to_end(&mut second);

		assert!(second.is_empty());
	}

	#[test]
	fn oversized_headers_return_400() {
		// server.rs TODO: the limits are hardcoded constants (MAX_HEADER_BYTES).
		let (_server, addr) = start_server();

		let mut raw = b"GET / HTTP/1.1\r\nX-Big: ".to_vec();
		raw.extend(std::iter::repeat_n(b'a', 17 * 1024));
		raw.extend_from_slice(b"\r\n\r\n");

		let response = raw_request(addr, &raw);

		assert!(response.starts_with("HTTP/1.1 400 Bad Request\r\n"));
	}

	#[test]
	fn header_size_limit_is_16_kib() {
		// Pins the hardcoded MAX_HEADER_BYTES = 16 * 1024 boundary.
		fn request_with_header_size(total: usize) -> Vec<u8> {
			let prefix = b"GET / HTTP/1.1\r\nX-Big: ";
			let suffix = b"\r\n\r\n";
			assert!(total > prefix.len() + suffix.len());

			let mut raw = Vec::with_capacity(total);
			raw.extend_from_slice(prefix);
			raw.extend(std::iter::repeat_n(b'a', total - prefix.len() - suffix.len()));
			raw.extend_from_slice(suffix);
			raw
		}

		let (_server, addr) = start_server();

		let accepted = raw_request(addr, &request_with_header_size(16 * 1024));
		assert!(accepted.starts_with("HTTP/1.1 200 OK\r\n"));

		let rejected = raw_request(addr, &request_with_header_size(16 * 1024 + 1));
		assert!(rejected.starts_with("HTTP/1.1 400 Bad Request\r\n"));
	}

	#[test]
	fn bind_failure_leaves_server_stopped() {
		// ANALYSIS.md #4: bind happens before the accept thread is spawned.
		let (mut first, addr) = start_server();

		let mut second = http_server::new();
		second.set_verbose(false);
		second.run(&addr.to_string());

		assert!(second.local_addr().is_none());
		assert!(get(addr, "/").starts_with("HTTP/1.1 200 OK\r\n"));

		first.close();
		second.close();
	}

	#[test]
	fn handlers_are_not_serialized_by_the_router_lock() {
		// ANALYSIS.md #9: the router lock is released before the handler runs.
		let (_server, addr) = start_server();
		let started = Instant::now();

		let workers: Vec<_> = (0..4)
			.map(|_| {
				thread::spawn(move || {
					let response = get(addr, "/slow");
					assert!(response.starts_with("HTTP/1.1 200 OK\r\n"));
				})
			})
			.collect();

		for worker in workers {
			worker.join().expect("worker thread panicked");
		}

		let elapsed = started.elapsed();
		assert!(
			elapsed < Duration::from_millis(450),
			"handlers were serialized; 4 x 150 ms took {elapsed:?}"
		);
	}

	#[test]
	fn server_survives_a_bad_request() {
		// ANALYSIS.md #2: parse failures are answered with 400, not a 404.
		let (_server, addr) = start_server();

		let bad = raw_request(addr, b"GET\r\n\r\n");
		assert!(bad.starts_with("HTTP/1.1 400 Bad Request\r\n"));

		let good = get(addr, "/");
		assert!(good.starts_with("HTTP/1.1 200 OK\r\n"));
	}

	#[test]
	fn head_on_get_only_route_returns_404() {
		// ANALYSIS.md #10: HEAD is only served when registered explicitly.
		let (_server, addr) = start_server();

		let response = raw_request(
			addr,
			b"HEAD / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
		);

		assert!(response.starts_with("HTTP/1.1 404 Not Found\r\n"));
	}

	#[test]
	fn run_called_twice_starts_a_second_listener() {
		// ANALYSIS.md (Low): `run` can be called twice and spawn duplicate
		// listeners; the first handle is simply overwritten.
		let mut server = http_server::new();
		server.set_verbose(false);
		server.add_route(http_methods::GET, "/", hello);

		server.run("127.0.0.1:0");
		let first = server.local_addr().expect("first listener");
		server.run("127.0.0.1:0");
		let second = server.local_addr().expect("second listener");

		assert_ne!(first, second);
		assert!(get(first, "/").starts_with("HTTP/1.1 200 OK\r\n"));
		assert!(get(second, "/").starts_with("HTTP/1.1 200 OK\r\n"));

		server.close();
	}

	#[test]
	fn invalid_bind_address_leaves_server_stopped() {
		// ANALYSIS.md #4: bind errors are reported before any thread starts.
		let mut server = http_server::new();
		server.set_verbose(false);
		server.run("this is not a socket address");

		assert!(server.local_addr().is_none());
		server.close();
	}

	// ---- known gaps: intended behaviour, currently failing ----------------
	//
	// These tests assert the correct HTTP behaviour and are expected to fail
	// until the corresponding item in ANALYSIS.md / TODO is fixed. They stay
	// in the normal suite so the gaps are visible in `cargo test` output.

	#[test]
	fn gap_request_body_reaches_handler() {
		let (_server, addr) = start_server();

		let response = raw_request(
			addr,
			b"POST /echo HTTP/1.1\r\nHost: localhost\r\nContent-Length: 11\r\nConnection: close\r\n\r\nhello world",
		);

		assert!(response.ends_with("hello world"));
	}

	#[test]
	fn gap_wrong_method_returns_405() {
		let (_server, addr) = start_server();

		let response = raw_request(
			addr,
			b"POST / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
		);

		assert!(response.starts_with("HTTP/1.1 405 Method Not Allowed\r\n"));
	}

	#[test]
	fn gap_custom_response_header_is_serialized() {
		let (_server, addr) = start_server();

		let response = get(addr, "/headers");

		assert!(response.contains("X-Test: yes\r\n"));
	}

	#[test]
	fn gap_no_content_omits_content_length() {
		let (_server, addr) = start_server();

		let response = get(addr, "/no-content");

		assert!(response.starts_with("HTTP/1.1 204 No Content\r\n"));
		assert!(!response.contains("Content-Length:"));
	}

	#[test]
	fn gap_head_response_has_no_body() {
		let (_server, addr) = start_server();

		let response = raw_request(
			addr,
			b"HEAD /head HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
		);

		assert!(!response.ends_with("Hello world"));
	}

	#[test]
	fn gap_unsupported_http_version_returns_505() {
		let (_server, addr) = start_server();

		let response = raw_request(
			addr,
			b"GET / HTTP/9.9\r\nHost: localhost\r\nConnection: close\r\n\r\n",
		);

		assert!(response.starts_with("HTTP/1.1 505 HTTP Version Not Supported\r\n"));
	}

	#[test]
	fn gap_query_values_are_percent_decoded() {
		let (_server, addr) = start_server();

		let response = get(addr, "/query?name=hello%20world");

		assert!(response.ends_with("name=hello world"));
	}

	#[test]
	fn gap_response_includes_connection_header() {
		let (_server, addr) = start_server();

		let response = get(addr, "/");

		assert!(response.contains("Connection: close\r\n"));
	}

	#[test]
	fn gap_keep_alive_serves_two_requests() {
		let (_server, addr) = start_server();

		let mut stream = TcpStream::connect(addr).unwrap();
		stream
			.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\nConnection: keep-alive\r\n\r\n")
			.unwrap();

		let mut first = Vec::new();
		stream.read_to_end(&mut first).unwrap();
		assert!(first.starts_with(b"HTTP/1.1 200"));

		let _ = stream.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n");
		let mut second = Vec::new();
		let _ = stream.read_to_end(&mut second);

		assert!(!second.is_empty());
		assert!(second.starts_with(b"HTTP/1.1 200"));
	}
}

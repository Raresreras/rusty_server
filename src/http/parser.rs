// HTTP 1.1 compliant server built by Raresreras
// Main parser

use super::types::{http_methods, http_request, http_header};

pub fn http_parser(input: &[u8]) -> Option<http_request> {
	let mut request = http_request::default();

	// Slicing the body away from the header

	let header_end = find_subslice(input, b"\r\n\r\n")?;
	let header_section = &input[..header_end]; // Slicing till the end of the header
	let body = &input[header_end + 4..]; // skip past \r\n\r\n

	let text = std::str::from_utf8(header_section).ok()?;
	let binding = text.replace("\r\n", " ");
	let mut split_input = binding.split(' ');

	// HTTP method extraction
	
	let method_string = split_input.next()?.to_string();
	request.method = match method_string.as_str() {
		"GET"     => http_methods::GET,
		"HEAD"    => http_methods::HEAD,
		"POST"    => http_methods::POST,
		"PUT"     => http_methods::PUT,
		"DELETE"  => http_methods::DELETE,
		"CONNECT" => http_methods::CONNECT,
		"OPTIONS" => http_methods::OPTIONS,
		"TRACE"   => http_methods::TRACE,
		"PATCH"   => http_methods::PATCH,
		_         => http_methods::NONE,
	};

	// HTTP path extraction
	// TODO! implement query extraction
	
	request.path = split_input.next()?.to_string();

	// HTTP version extraction
	
	request.version = split_input.next()?.to_string();

	// HTTP headers

	while let Some(header) = split_input.next() {
		if let Some(value) = split_input.next() {
			request.header_vector.push(http_header{
				header_name: header.to_string(),
				header_value: value.to_string()
			});
		}
	}

	// HTTP body

	Some(request)
}

// Helper function
// Find the byte index where `needle` first occurs in `haystack`
fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
	haystack
		.windows(needle.len())
		.position(|window| window == needle)
}
// HTTP 1.1 compliant server built by Raresreras
// HTTP parsing functions file

use super::types::{http_methods, http_request, http_header};

// Main parser
// This parser works by slicing the http request in two parts, header and body
// and parsing them isolated one from another
pub fn http_parser(input: &[u8]) -> Option<http_request> {
	let mut request = http_request::default();

	// Slicing the body away from the header
	let header_end = find_subslice(input, b"\r\n\r\n")?;
	let header_section = &input[..header_end]; // Slicing till the end of the header
	let body = &input[header_end + 4..]; // skip past \r\n\r\n

	let text = std::str::from_utf8(header_section).ok()?;
	let mut lines = text.split("\r\n");

	// Request line detection
	let request_line = lines.next()?;
	let mut request_parts = request_line.splitn(3, " ");

	// HTTP method extraction
	let method_string = request_parts.next()?.to_string();
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
	let temp_path = request_parts.next()?.to_string();

	// HTTP query extraction from the path
	if let Some(query_pos) = temp_path.find('?') {
		request.path = (&temp_path[..query_pos]).to_string();

		let query = (&temp_path[query_pos+1..]).to_string();

		let query_parts = query.split('&');
		for part in query_parts {
			//TODO! Add further parsing here, so we dont spit out raw data
			request.query_vector.push(part.to_string());
		}
	} else {
		request.path = temp_path;
	}

	// HTTP version extraction
	request.version = request_parts.next()?.to_string();

	// HTTP headers
	for line in lines {
		if let Some(separator_pos) = line.find(':') {
			// Name and value extraction
			let name = (&line[..separator_pos]).trim().to_string();
			let value = (&line[separator_pos+1..]).trim().to_string();

			request.header_vector.push(http_header{
				header_name: name,
				header_value: value
			});
		} else {
			// No separator found, header skipped
			continue;
		}
	}

	// HTTP body
	request.body = body.into();

	Some(request)
}

// Helper function
// Find the byte index where `needle` first occurs in `haystack`
fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
	haystack
		.windows(needle.len())
		.position(|window| window == needle)
}
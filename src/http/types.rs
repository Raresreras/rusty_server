// HTTP 1.1 compliant server built by Raresreras
// Types File containing structs and enums

// We will see how many we manage to implement

// HTTP methods enumaration + default
#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub enum http_methods {
	GET,
	HEAD,
	POST,
	PUT,
	DELETE,
	CONNECT,
	OPTIONS,
	TRACE,
	PATCH,
	NONE, // used as default
}

impl Default for http_methods {
	fn default() -> Self {
		http_methods::NONE
	}
}

// Header struct
#[derive(Debug)]
pub struct http_header {
	pub header_name: String,
	pub header_value: String,
}

// Request struct
#[derive(Default, Debug)]
pub struct http_request {
	pub method: http_methods,
	pub path: String,
	pub version: String,
	pub query_vector: Vec<String>,
	pub header_vector: Vec<http_header>,
	pub body: Vec<u8>,
}

// HTTP status code enumaration + auxiliary implementations
#[derive(Debug)]
pub enum http_status {
	// 1xx Informational
	Continue,
	Switching_Protocols,

	// 2xx Success
	OK,
	Created,
	Accepted,
	Non_Authoritative_Information,
	No_Content,
	Reset_Content,
	Partial_Content,

	// 3xx Redirection
	Multiple_Choices,
	Moved_Permanently,
	Found,
	See_Other,
	Not_Modified,
	Use_Proxy,
	// 306 is unused/reserved in the spec — intentionally omitted
	Temporary_Redirect,

	// 4xx Client Error
	Bad_Request,
	Unauthorized,
	Payment_Required,
	Forbidden,
	Not_Found,
	Method_Not_Allowed,
	Not_Acceptable,
	Proxy_Authentication_Required,
	Request_Timeout,
	Conflict,
	Gone,
	Length_Required,
	Precondition_Failed,
	Request_Entity_Too_Large,
	Request_URI_Too_Long,
	Unsupported_Media_Type,
	Requested_Range_Not_Satisfiable,
	Expectation_Failed,

	// 5xx Server Error
	Internal_Server_Error,
	Not_Implemented,
	Bad_Gateway,
	Service_Unavailable,
	Gateway_Timeout,
	HTTP_Version_Not_Supported,
}

impl Default for http_status {
	fn default() -> Self {
		http_status::OK
	}
}

impl http_status {
	pub fn code(&self) -> u16 {
		match self {
			// 1xx Informational
			http_status::Continue                       => 100,
			http_status::Switching_Protocols            => 101,

			// 2xx Success
			http_status::OK                             => 200,
			http_status::Created                        => 201,
			http_status::Accepted                       => 202,
			http_status::Non_Authoritative_Information  => 203,
			http_status::No_Content                     => 204,
			http_status::Reset_Content                  => 205,
			http_status::Partial_Content                => 206,

			// 3xx Redirection
			http_status::Multiple_Choices               => 300,
			http_status::Moved_Permanently              => 301,
			http_status::Found                          => 302,
			http_status::See_Other                      => 303,
			http_status::Not_Modified                   => 304,
			http_status::Use_Proxy                      => 305,
			http_status::Temporary_Redirect             => 307,

			// 4xx Client Error
			http_status::Bad_Request                    => 400,
			http_status::Unauthorized                   => 401,
			http_status::Payment_Required               => 402,
			http_status::Forbidden                      => 403,
			http_status::Not_Found                      => 404,
			http_status::Method_Not_Allowed             => 405,
			http_status::Not_Acceptable                 => 406,
			http_status::Proxy_Authentication_Required  => 407,
			http_status::Request_Timeout                => 408,
			http_status::Conflict                       => 409,
			http_status::Gone                           => 410,
			http_status::Length_Required                => 411,
			http_status::Precondition_Failed            => 412,
			http_status::Request_Entity_Too_Large       => 413,
			http_status::Request_URI_Too_Long           => 414,
			http_status::Unsupported_Media_Type         => 415,
			http_status::Requested_Range_Not_Satisfiable => 416,
			http_status::Expectation_Failed             => 417,

			// 5xx Server Error
			http_status::Internal_Server_Error          => 500,
			http_status::Not_Implemented                => 501,
			http_status::Bad_Gateway                    => 502,
			http_status::Service_Unavailable            => 503,
			http_status::Gateway_Timeout                => 504,
			http_status::HTTP_Version_Not_Supported     => 505,
		}
	}
}

// Response struct
#[derive(Default, Debug)]
pub struct http_response {
	pub version: String,
	pub status: u16,
	pub header_vector: Vec<http_header>,
	pub body: Vec<u8>,
}

// Default respones
impl http_response {
	pub fn empty(status: http_status) -> Self {
		http_response {
			version: "HTTP/1.1".to_string(),
			status: status.code() as u16,
			..Default::default()
		}
	}

	pub fn error() -> Self {
		http_response {
			version: "HTTP/1.1".to_string(),
			status: http_status::Not_Found.code() as u16,
			..Default::default()
		}
	}

	pub fn reason_from_code(&self) -> &str {
		match self.status {
			// 1xx Informational
			100 => "Continue",
			101 => "Switching Protocols",
			// 2xx Success
			200 => "OK",
			201 => "Created",
			202 => "Accepted",
			203 => "Non-Authoritative Information",
			204 => "No Content",
			205 => "Reset Content",
			206 => "Partial Content",
			// 3xx Redirection
			300 => "Multiple Choices",
			301 => "Moved Permanently",
			302 => "Found",
			303 => "See Other",
			304 => "Not Modified",
			305 => "Use Proxy",
			307 => "Temporary Redirect",
			// 4xx Client Error
			400 => "Bad Request",
			401 => "Unauthorized",
			402 => "Payment Required",
			403 => "Forbidden",
			404 => "Not Found",
			405 => "Method Not Allowed",
			406 => "Not Acceptable",
			407 => "Proxy Authentication Required",
			408 => "Request Timeout",
			409 => "Conflict",
			410 => "Gone",
			411 => "Length Required",
			412 => "Precondition Failed",
			413 => "Request Entity Too Large",
			414 => "Request-URI Too Long",
			415 => "Unsupported Media Type",
			416 => "Requested Range Not Satisfiable",
			417 => "Expectation Failed",
			// 5xx Server Error
			500 => "Internal Server Error",
			501 => "Not Implemented",
			502 => "Bad Gateway",
			503 => "Service Unavailable",
			504 => "Gateway Timeout",
			505 => "HTTP Version Not Supported",
			_ => "Unknown",
		}
	}

	pub fn to_bytes(&self) -> Vec<u8> {
		// Creating the output bytes vector
		let mut out = Vec::new();
		out.extend(self.version.as_bytes());
		out.extend(b" ");
		out.extend(self.status.to_string().as_bytes());
		out.extend(b" ");
		out.extend(self.reason_from_code().as_bytes());
		out.extend(b"\r\n");

		// TODO! add headers

		out.extend(b"\r\n");
		out.extend(self.body.clone());
		out
	}
}

// Routing table function type
pub type route_func = fn(&http_request) -> http_response;
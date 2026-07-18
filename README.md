# Rusty Server

Simple HTTP 1.1 server made with Rust for educational purposes.

## File tree

├── Cargo.toml  
├── README.md  
└── src  
    ├── http  
    │	├── handler.rs  
    │	├── parser.rs  
    │	└── types.rs  
    ├── http.rs  
    └── main.rs  

## Current architecture

### Parser

Simple http parser( which needs to be rewritten :) )

```
	pub fn http_parser(input: &[u8]) -> Option<http_request>
```

### Handler

The http handler uses a routing table, which has the following methods:

```
	pub fn new() -> Self
	pub fn add_route(&mut self, method: http_methods, path: &str, handler: route_func)
	pub fn handle(&self, input: &http_request) -> (http_status, Vec<u8>)
```


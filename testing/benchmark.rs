//! Simple benchmark: this server vs. a tiny axum server.
//!
//! Both servers are started on ephemeral localhost ports. The same load
//! generator sends the same raw HTTP/1.1 requests to each one, using a fresh
//! connection per request (just like the server currently behaves).
//!
//! Run from the repository root:
//!
//!   cargo run --release -p rusty_server_testing --bin benchmark
//!   cargo run --release -p rusty_server_testing --bin benchmark -- \
//!       --requests 10000 --concurrency 64 --warmup 1000

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::{Duration, Instant};

use axum::extract::RawQuery;
use axum::routing::get;
use axum::Router;
use rusty_server::http::{http_methods, http_request, http_response, http_status};
use rusty_server::server::http_server;

const PATHS: [(&str, &str); 2] = [("/", "Hello world"), ("/query?name=rares", "name=rares")];

struct Config {
	requests: usize,
	concurrency: usize,
	warmup: usize,
}

impl Config {
	fn from_args() -> Self {
		let mut config = Config {
			requests: 2_000,
			concurrency: 32,
			warmup: 200,
		};

		let mut args = std::env::args().skip(1);
		while let Some(arg) = args.next() {
			match arg.as_str() {
				"--requests" => config.requests = read_usize(&mut args, "--requests"),
				"--concurrency" => config.concurrency = read_usize(&mut args, "--concurrency"),
				"--warmup" => config.warmup = read_usize(&mut args, "--warmup"),
				"-h" | "--help" => {
					println!(
						"benchmark [--requests N] [--concurrency N] [--warmup N]\n\
						 defaults: --requests 2000 --concurrency 32 --warmup 200"
					);
					std::process::exit(0);
				}
				other => {
					eprintln!("unknown argument: {other}");
					std::process::exit(2);
				}
			}
		}

		assert!(config.requests > 0, "--requests must be greater than 0");
		assert!(config.concurrency > 0, "--concurrency must be greater than 0");
		config
	}
}

fn read_usize(args: &mut impl Iterator<Item = String>, flag: &str) -> usize {
	args.next()
		.unwrap_or_else(|| {
			eprintln!("{flag} requires a value");
			std::process::exit(2);
		})
		.parse()
		.unwrap_or_else(|_| {
			eprintln!("{flag} must be a positive integer");
			std::process::exit(2);
		})
}

fn main() {
	let config = Config::from_args();

	let (mut rusty, rusty_addr) = start_rusty();
	let axum_addr = start_axum();

	wait_until_ready(rusty_addr);
	wait_until_ready(axum_addr);

	println!("rusty_server: http://{rusty_addr}");
	println!("axum:         http://{axum_addr}");
	println!(
		"requests: {}, concurrency: {}, warmup: {}\n",
		config.requests, config.concurrency, config.warmup
	);

	for (path, expected_body) in PATHS {
		println!("GET {path}");

		// Warm both servers up before timing either one.
		let _ = run_load(rusty_addr, path, expected_body, config.warmup, config.concurrency);
		let _ = run_load(axum_addr, path, expected_body, config.warmup, config.concurrency);

		let (rusty_time, rusty_errors) =
			run_load(rusty_addr, path, expected_body, config.requests, config.concurrency);
		let (axum_time, axum_errors) =
			run_load(axum_addr, path, expected_body, config.requests, config.concurrency);

		let rusty_rps = print_result("rusty_server", rusty_time, config.requests, rusty_errors);
		let axum_rps = print_result("axum", axum_time, config.requests, axum_errors);

		if rusty_rps > 0.0 {
			println!("  axum / rusty: {:.2}x\n", axum_rps / rusty_rps);
		}
	}

	rusty.close();
}

fn print_result(name: &str, elapsed: Duration, requests: usize, errors: usize) -> f64 {
	let rps = requests as f64 / elapsed.as_secs_f64();
	println!(
		"  {name:<16} {:>10.0} req/s   {:>8.1} ms total   errors: {errors}",
		rps,
		elapsed.as_secs_f64() * 1000.0
	);
	rps
}

fn start_rusty() -> (http_server, SocketAddr) {
	let mut server = http_server::new();
	server.set_verbose(false);
	server.add_route(http_methods::GET, "/", hello);
	server.add_route(http_methods::GET, "/query", query);
	server.run("127.0.0.1:0");

	let addr = server
		.local_addr()
		.expect("rusty_server did not bind an ephemeral port");

	(server, addr)
}

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

fn start_axum() -> SocketAddr {
	let (tx, rx) = mpsc::channel();

	thread::spawn(move || {
		let runtime = tokio::runtime::Builder::new_multi_thread()
			.enable_all()
			.build()
			.expect("failed to build tokio runtime");

		runtime.block_on(async move {
			let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
				.await
				.expect("axum failed to bind");
			let addr = listener.local_addr().expect("failed to read axum address");

			tx.send(addr).expect("main thread went away");
			axum::serve(listener, axum_router())
				.await
				.expect("axum server error");
		});
	});

	rx.recv().expect("axum did not start")
}

fn axum_router() -> Router {
	Router::new()
		.route("/", get(|| async { "Hello world" }))
		.route(
			"/query",
			get(|RawQuery(raw): RawQuery| async move {
				raw.unwrap_or_default()
					.split('&')
					.next()
					.unwrap_or_default()
					.to_string()
			}),
		)
}

fn wait_until_ready(addr: SocketAddr) {
	for _ in 0..200 {
		if TcpStream::connect_timeout(&addr, Duration::from_millis(100)).is_ok() {
			return;
		}
		thread::sleep(Duration::from_millis(10));
	}
	panic!("server at {addr} did not become ready");
}

/// Run `requests` requests with `concurrency` worker threads.
///
/// Returns the elapsed time and the number of failed requests.
fn run_load(
	addr: SocketAddr,
	path: &str,
	expected_body: &str,
	requests: usize,
	concurrency: usize,
) -> (Duration, usize) {
	if requests == 0 {
		return (Duration::ZERO, 0);
	}

	let next = Arc::new(AtomicUsize::new(0));
	let errors = Arc::new(AtomicUsize::new(0));
	let raw_request = build_request(path);
	let started = Instant::now();
	let mut workers = Vec::new();

	for _ in 0..concurrency.min(requests) {
		let next = Arc::clone(&next);
		let errors = Arc::clone(&errors);
		let raw_request = raw_request.clone();
		let expected_body = expected_body.to_string();

		workers.push(thread::spawn(move || loop {
			if next.fetch_add(1, Ordering::Relaxed) >= requests {
				break;
			}

			if one_request(addr, &raw_request, expected_body.as_bytes()).is_err() {
				errors.fetch_add(1, Ordering::Relaxed);
			}
		}));
	}

	for worker in workers {
		worker.join().expect("benchmark worker panicked");
	}

	(started.elapsed(), errors.load(Ordering::Relaxed))
}

fn build_request(path: &str) -> Vec<u8> {
	format!(
		"GET {path} HTTP/1.1\r\nHost: localhost\r\nUser-Agent: benchmark\r\nConnection: close\r\n\r\n"
	)
	.into_bytes()
}

fn one_request(addr: SocketAddr, request: &[u8], expected_body: &[u8]) -> std::io::Result<()> {
	let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(1))?;
	stream.set_nodelay(true)?;
	stream.set_read_timeout(Some(Duration::from_secs(5)))?;
	stream.write_all(request)?;

	let mut response = Vec::new();
	stream.read_to_end(&mut response)?;

	if !response.starts_with(b"HTTP/1.1 200") {
		return Err(std::io::Error::new(
			std::io::ErrorKind::InvalidData,
			"unexpected status line",
		));
	}
	if !response.ends_with(expected_body) {
		return Err(std::io::Error::new(
			std::io::ErrorKind::InvalidData,
			"unexpected response body",
		));
	}

	Ok(())
}

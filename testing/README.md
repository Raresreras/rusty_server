# Benchmark

A tiny comparison between this server and an axum server.

Run from the repository root:

```sh
cargo run --release -p rusty_server_testing --bin benchmark
```

Custom load:

```sh
cargo run --release -p rusty_server_testing --bin benchmark -- \
    --requests 10000 --concurrency 64 --warmup 1000
```

Both servers start on ephemeral localhost ports. Each request uses a fresh TCP
connection, and the benchmark reports requests/sec for `GET /` and `GET /query`.

The unit tests live in `src/main.rs`:

```sh
cargo test
```

Some tests document known gaps from `ANALYSIS.md` and the `TODO` comments.
They are part of the normal suite and are expected to fail until the
corresponding behaviour is fixed:

```sh
cargo test
```

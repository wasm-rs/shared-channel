# Shared Channel for WebAssembly
[![Crate](https://img.shields.io/crates/v/wasm-rs-shared-channel.svg)](https://crates.io/crates/wasm-rs-shared-channel)
[![API](https://docs.rs/wasm-rs-shared-channel/badge.svg)](https://docs.rs/wasm-rs-shared-channel)
[![Chat](https://img.shields.io/discord/807386653852565545.svg?logo=discord)](https://discord.gg/qbcbjHWjaD)

This crate provides a way for WebAssembly threads to receive messages from other threads using
a JavaScript primitive called `SharedArrayBuffer` which allows to share memory and use atomics
between different threads.

This allows us to deploy Rust code as a worker process communicating with the main thread.

## Usage

Include this dependency in your `Cargo.toml`:

```toml
[dependencies]
wasm-rs-shared-channel = "0.1.0"
```

Take a look at the
[example](https://github.com/wasm-rs/shared-channel/tree/master/example) to see
how `wasm-rs-shared-channel` can be integrated.

## License

Licensed under either of

 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT) at your option.

# ntex redis [![build status](https://github.com/ntex-rs/ntex-redis/workflows/CI%20%28Linux%29/badge.svg?branch=master&event=push)](https://github.com/ntex-rs/ntex-redis/actions?query=workflow%3A"CI+(Linux)") [![codecov](https://codecov.io/gh/ntex-rs/ntex-redis/branch/master/graph/badge.svg)](https://codecov.io/gh/ntex-rs/ntex-redis) [![crates.io](https://img.shields.io/crates/v/ntex-redis)](https://crates.io/crates/ntex-redis)

Redis client for the [ntex](https://github.com/ntex-rs/ntex) framework.

## Documentation & community resources

* [API Documentation](https://docs.rs/ntex-redis)
* [ntex Documentation](https://ntex.rs)
* Minimum supported Rust version: 1.97 or later

## Usage

Add `ntex-redis` and `ntex` with a runtime feature (`tokio`, `compio`,
`neon-polling` or `neon-uring`) to your `Cargo.toml`:

```toml
[dependencies]
ntex = { version = "4", features = ["tokio"] }
ntex-redis = "3"
```

## Example

```rust
use ntex_redis::{RedisConnector, cmd};

#[ntex::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let redis = RedisConnector::new("127.0.0.1:6379").connect().await?;

    // create list with one value
    redis.exec(cmd::LPush("test", "value")).await?;

    // get value by index
    let value = redis.exec(cmd::LIndex("test", 0)).await?;
    assert_eq!(value.unwrap(), "value");

    // remove key
    redis.exec(cmd::Del("test")).await?;

    Ok(())
}
```

`RedisConnector` also supports authentication (`.password(..)`), custom
I/O configuration (`.config(..)`) and custom connectors, e.g. for TLS
(`.connector(..)`). Use `connect_simple()` for a non-pipelined client
with pub/sub support, see [examples/pubsub.rs](examples/pubsub.rs).

## License

This project is licensed under either of

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or [http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0))
* MIT license ([LICENSE-MIT](LICENSE-MIT) or [http://opensource.org/licenses/MIT](http://opensource.org/licenses/MIT))

at your option.

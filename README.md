# ntex redis [![build status](https://github.com/ntex-rs/ntex-redis/workflows/CI%20%28Linux%29/badge.svg?branch=master&event=push)](https://github.com/ntex-rs/ntex-redis/actions?query=workflow%3A"CI+(Linux)") [![codecov](https://codecov.io/gh/ntex-rs/ntex-redis/branch/master/graph/badge.svg)](https://codecov.io/gh/ntex-rs/ntex-redis) [![crates.io](https://img.shields.io/crates/v/ntex-redis)](https://crates.io/crates/ntex-redis)

redis client for ntex framework

## Documentation & community resources

* [Documentation](https://docs.rs/ntex-redis)
* Minimum supported Rust version: 1.65 or later

## Example

```rust
use ntex_redis::{cmd, RedisConnector};

#[ntex::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let redis = RedisConnector::new("127.0.0.1:6379").connect().await?;

    // create list with one value
    redis.exec(cmd::LPush("test", "value"));

    // get value by index
    let value = redis.exec(cmd::LIndex("test", 0)).await?;
    assert_eq!(value.unwrap(), "value");

    // remove key
    redis.exec(cmd::Del("test")).await?;

    Ok(())
}
```

## License

This project is licensed under

* MIT license ([LICENSE](LICENSE) or [http://opensource.org/licenses/MIT](http://opensource.org/licenses/MIT))

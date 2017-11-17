# tic - time interval counts and statistics
a high-performance stats library focused on rates and latencies from timestamped events

[![conduct-badge][]][conduct] [![travis-badge][]][travis] [![downloads-badge][] ![release-badge][]][crate] [![license-badge][]](#license)

[conduct-badge]: https://img.shields.io/badge/%E2%9D%A4-code%20of%20conduct-blue.svg
[travis-badge]: https://img.shields.io/travis/brayniac/tic/master.svg
[downloads-badge]: https://img.shields.io/crates/d/tic.svg
[release-badge]: https://img.shields.io/crates/v/tic.svg
[license-badge]: https://img.shields.io/crates/l/tic.svg
[conduct]: https://brayniac.github.io/conduct
[travis]: https://travis-ci.org/brayniac/tic
[crate]: https://crates.io/crates/tic
[Cargo]: https://github.com/rust-lang/cargo

## Code of Conduct

**NOTE**: All conversations and contributions to this project shall adhere to the [Code of Conduct][conduct]

## Usage

The API documentation of this library can be found at
[docs.rs/tic](https://docs.rs/tic/)

## Performance and Example

Performance is a top-priority. To test performance, we use tic to benchmark itself.

```shell
cargo run --release --example benchmark -- --help
cargo run --release --example benchmark
```

## Features

* high throughput - millions of samples per second
* latched histogram - for analyzing the full distribution of sample lengths
* heatmaps - to generate distribution traces and waterfalls
* meters - to expose readings for client usage
* http metrics - simple metrics on http for scraping and monitoring, Prometheus compatible
* generic - channel type is generic, and used to label the type of sample
* flexible - per channel stats are accessible by registering appropriate `Interest`s

## License

Licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.

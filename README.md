# tic - [WIP] time interval counter with stats

tic is a specialized stats library, with a focus on deriving statistics from timestamped events

Features will include:


[![Build Status](https://travis-ci.org/brayniac/tic.svg?branch=master)](https://travis-ci.org/brayniac/tic)
[![crates.io](http://meritbadge.herokuapp.com/tic)](https://crates.io/crates/tic)
[![License](http://img.shields.io/:license-mit-blue.svg)](http://opensource.org/licenses/MIT)
[![License](http://img.shields.io/badge/license-APACHE2-blue.svg)](http://www.apache.org/licenses/LICENSE-2.0)

## Usage

To use `tic`, first add this to your `Cargo.toml`:

```toml
[dependencies]
tic = "*"
```

Then, add this to your crate root:

```rust
extern crate tic;
```

The API documentation of this library can be found at
[brayniac.github.io/tic](https://brayniac.github.io/tic/)

## Features

tic is still work-in-progress - probably not ready for public consumption

* latched counters - for calculating frequency / rate of an event
* latched histogram - for analyzing the full distribution of event durations
* gauges - to expose derived stats (rates, percentiles, etc)

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

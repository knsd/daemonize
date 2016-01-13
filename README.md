daemonize [![Build Status](https://travis-ci.org/knsd/daemonize.svg?branch=master)](https://travis-ci.org/knsd/daemonize) [![Latest Version](https://img.shields.io/crates/v/daemonize.svg)](https://crates.io/crates/daemonize/)
=========


daemonize is a library for writing system daemons. Inspired by the Python library [thesharp/daemonize](https://github.com/thesharp/daemonize).

The documentation is located at http://knsd.github.io/daemonize/.

Usage example:

```rust
#[macro_use] extern crate log;
extern crate daemonize;

use daemonize::{Daemonize};

fn main() {
    let daemonize = Daemonize::new()
        .pid_file("/tmp/test.pid") // Every method except `new` and `start`
        .chown_pid_file(true)      // is optional, see `Daemonize` documentation
        .working_directory("/tmp") // for default behaviour.
        .user("nobody")
        .group("daemon") // Group name
        .group(2)        // Or group id
        .privileged_action(|| "Executed before drop privileges");

     match daemonize.start() {
         Ok(_) => info!("Success, daemonized"),
         Err(e) => error!("{}", e),
     }
 }
```

### License

Licensed under either of
 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you shall be dual licensed as above, without any
additional terms or conditions.

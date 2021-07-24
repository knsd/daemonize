daemonize [![Build Status](https://github.com/knsd/daemonize/actions/workflows/rust.yml/badge.svg)](https://github.com/knsd/daemonize/actions/workflows/rust.yml/) [![Latest Version](https://img.shields.io/crates/v/daemonize.svg)](https://crates.io/crates/daemonize/) [![docs](https://docs.rs/daemonize/badge.svg)](https://docs.rs/daemonize)
=========


daemonize is a library for writing system daemons. Inspired by the Python library [thesharp/daemonize](https://github.com/thesharp/daemonize).

Usage example:

```rust
extern crate daemonize;

use std::fs::File;

use daemonize::Daemonize;

fn main() {
    let stdout = File::create("/tmp/daemon.out").unwrap();
    let stderr = File::create("/tmp/daemon.err").unwrap();

    let daemonize = Daemonize::new()
        .pid_file("/tmp/test.pid") // Every method except `new` and `start`
        .chown_pid_file(true)      // is optional, see `Daemonize` documentation
        .working_directory("/tmp") // for default behaviour.
        .user("nobody")
        .group("daemon") // Group name
        .group(2)        // or group id.
        .umask(0o777)    // Set umask, `0o027` by default.
        .stdout(stdout)  // Redirect stdout to `/tmp/daemon.out`.
        .stderr(stderr)  // Redirect stderr to `/tmp/daemon.err`.
        .privileged_action(|| "Executed before drop privileges");

    match daemonize.start() {
        Ok(_) => println!("Success, daemonized"),
        Err(e) => eprintln!("Error, {}", e),
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

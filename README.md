daemonize [![Build Status](https://travis-ci.org/knsd/daemonize.svg?branch=master)](https://travis-ci.org/knsd/daemonize)
=========


daemonize is a library for writing system daemons. Inspired by the Python library [thesharp/daemonize](https://github.com/thesharp/daemonize).

The documentation is located at http://knsd.github.io/daemonize/.

Usage example:

```rust
#[macro_use] extern crate log;
extern crate daemonize;

use daemonize::{Daemonize};

fn main() {
    let daemonize = Daemonize::new().pid_file("/tmp/test.pid")
                                    .chown_pid_file(true)
                                    .working_directory("/tmp")
                                    .user("nobody")
                                    .group("daemon") // Group name
                                    .group(2) // Or group id
                                    .privileged_action(|| "Executed before drop privileges");
     match daemonize.start() {
         Ok(_) => info!("Success, daemonized"),
         Err(e) => error!("{}", e),
     }
 }
```

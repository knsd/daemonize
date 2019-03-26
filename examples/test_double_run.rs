extern crate daemonize;

use daemonize::Daemonize;
use std::io::prelude::*;

fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    let pid = &args[1];
    let result_file = &args[2];

    let mut file = std::fs::File::create(result_file).unwrap();
    match Daemonize::new().pid_file(pid).start() {
        Ok(()) => {
            file.write_all(b"ok").unwrap();
            std::thread::sleep(std::time::Duration::from_secs(10));
        }
        Err(_) => file.write_all(b"error").unwrap(),
    };
}

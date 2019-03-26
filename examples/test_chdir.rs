extern crate daemonize;

use std::io::prelude::*;

use daemonize::Daemonize;

fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    let chdir = &args[1];
    let file = &args[2];
    let umask = args[3].parse().unwrap();

    Daemonize::new()
        .working_directory(chdir)
        .umask(umask)
        .start()
        .unwrap();
    std::fs::File::create(file)
        .unwrap()
        .write_all(b"test")
        .unwrap();
}

extern crate daemonize;

use std::io::prelude::*;

use daemonize::{Daemonize};

fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    let ref chdir = args[1];
    let ref file = args[2];

    Daemonize::new().working_directory(chdir).start().unwrap();
    std::fs::File::create(file).unwrap().write_all(b"test").unwrap();
}

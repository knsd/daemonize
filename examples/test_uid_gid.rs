extern crate daemonize;
extern crate libc;

use std::io::prelude::*;

use daemonize::Daemonize;

fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    let user = &(*args[1]);
    let group = &(*args[2]);
    let file = &args[3];

    let mut file = std::fs::File::create(file).unwrap();
    Daemonize::new().user(user).group(group).start().unwrap();
    unsafe {
        file.write_all(format!("{} {}", libc::getuid(), libc::getgid()).as_bytes())
            .unwrap();
    }
}

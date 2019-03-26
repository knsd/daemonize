extern crate daemonize;
extern crate libc;

use std::io::prelude::*;

use daemonize::Daemonize;

fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    let ref user = *args[1];
    let ref group = *args[2];
    let ref file = *args[3];

    let mut file = std::fs::File::create(file).unwrap();
    Daemonize::new().user(user).group(group).start().unwrap();
    unsafe {
        file.write_all(format!("{} {}", libc::getuid(), libc::getgid()).as_bytes())
            .unwrap();
    }
}

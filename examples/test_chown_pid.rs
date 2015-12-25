extern crate daemonize;

use daemonize::{Daemonize};

fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    let ref user = *args[1];
    let ref group = *args[2];
    let ref pid = *args[3];

    Daemonize::new().pid_file(pid).user(user).group(group).start().unwrap();
}

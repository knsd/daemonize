extern crate daemonize;

use daemonize::Daemonize;

fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    let user = &(*args[1]);
    let group = &(*args[2]);
    let pid = &(*args[3]);

    Daemonize::new()
        .pid_file(pid)
        .user(user)
        .group(group)
        .start()
        .unwrap();
}

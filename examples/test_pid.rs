extern crate daemonize;

use daemonize::Daemonize;

fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    let pid = &args[1];

    Daemonize::new().pid_file(pid).start().unwrap();
}

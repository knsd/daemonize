extern crate daemonize;
extern crate syslog;
#[macro_use] extern crate log;

use daemonize::{Daemonize};

fn main() {
    println!("{:?}", syslog::init(syslog::Facility::LOG_USER, log::LogLevelFilter::Debug, Some("test")));
    let result = Daemonize::new().pid_file("/tmp/test.pid")
                                 .privileged_action(|| println!("foo"))
                                 .chown_pid_file(false)
                                 .working_directory("/tmp/")
                                 // .user(10050)
                                 // .group("nobody")
                                 .start();
    error!("test, {:?}", result);
}

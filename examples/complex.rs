extern crate daemonize;
extern crate syslog;
#[macro_use] extern crate log;

use daemonize::{Daemonize};

fn main() {
    syslog::init(syslog::Facility::LOG_USER, log::LogLevelFilter::Debug, Some("daemon-name")).unwrap();

    let daemonize = Daemonize::new().pid_file("/tmp/test.pid")
                                    .chown_pid_file(true)
                                    .working_directory("/tmp")
                                    .user("nobody")
                                    .group("daemon")
                                    .group(2)
                                    .privileged_action(|| "Executed before drop privileges");

    info!("{:?}", daemonize.start());
}

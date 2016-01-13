extern crate daemonize;
extern crate syslog;
#[macro_use] extern crate log;

use daemonize::{Daemonize};

fn main() {
    syslog::init(syslog::Facility::LOG_USER, log::LogLevelFilter::Debug, Some("daemon-name")).unwrap();

    let daemonize = Daemonize::new()
        .pid_file("/tmp/test.pid") // Every method except `new` and `start`
        .chown_pid_file(true)      // is optional, see `Daemonize` documentation
        .working_directory("/tmp") // for default behaviour.
        .user("nobody")
        .group("daemon") // Group name
        .group(2)        // Or group id
        .privileged_action(|| "Executed before drop privileges");

    info!("{:?}", daemonize.start());
}

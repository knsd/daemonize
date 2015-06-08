extern crate daemonize;

use std::path::{Path};

use daemonize::{DaemonOptions, User, Group, daemonize};

fn main() {
    let res = daemonize(DaemonOptions{
        pid_file: Some(Path::new("/tmp/test.pid")),
        directory: None,
        user: Some(User::Name("nobody")),
        group: Some(Group::Id(10)),
    }, &(| | {println!("before drop");}));

    println!("after drop, res: {:?}", res);
}

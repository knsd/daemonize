extern crate tempdir;

use std::io::prelude::*;

use tempdir::{TempDir};

#[test]
fn test_chdir() {
    let tmpdir = TempDir::new("chdir").unwrap();

    let mut cmd = std::process::Command::new("target/debug/examples/chdir");
    cmd.arg(tmpdir.path()).arg("test");
    cmd.status().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(100));

    let mut data = Vec::new();
    std::fs::File::open(tmpdir.path().join("test")).unwrap().read_to_end(&mut data).unwrap();
    assert!(data == b"test")
}

#[test]
fn test_pid() {
    let tmpdir = TempDir::new("chdir").unwrap();
    let pid_file = tmpdir.path().join("pid");

    let mut cmd = std::process::Command::new("target/debug/examples/pid");
    cmd.arg(&pid_file);
    let mut child = cmd.spawn().unwrap();
    let child_pid = child.id();
    child.wait().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(100));

    let mut data = String::new();
    std::fs::File::open(pid_file).unwrap().read_to_string(&mut data).unwrap();
    let pid: u32 = data.parse().unwrap();
    assert!(pid != child_pid)
}

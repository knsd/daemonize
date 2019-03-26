extern crate libc;
extern crate tempdir;

use std::ffi::OsStr;
use std::io::prelude::*;

use tempdir::TempDir;

fn run<S: AsRef<OsStr>>(cmd: S, args: &[S]) -> u32 {
    let mut cmd = std::process::Command::new(cmd);
    for arg in args {
        cmd.arg(arg);
    }
    let mut child = cmd.spawn().unwrap();
    let pid = child.id();
    child.wait().unwrap();

    std::thread::sleep(std::time::Duration::from_millis(100));
    pid
}

#[test]
fn test_umask_chdir() {
    let tmpdir = TempDir::new("chdir").unwrap();

    // third argument is the umask: 255 == 0o377
    let args = vec![tmpdir.path().to_str().unwrap(), "test", "255"];
    run("target/debug/examples/test_chdir", &args);

    let filename = tmpdir.path().join("test");
    let mut data = Vec::new();
    std::fs::File::open(&filename)
        .unwrap()
        .read_to_end(&mut data)
        .unwrap();
    assert!(data == b"test");
    // due to the umask, the file should have been created with -w
    assert!(filename.metadata().unwrap().permissions().readonly());
}

#[test]
fn test_pid() {
    let tmpdir = TempDir::new("chdir").unwrap();
    let pid_file = tmpdir.path().join("pid");

    let args = vec![pid_file.to_str().unwrap()];
    let child_pid = run("target/debug/examples/test_pid", &args);

    let mut data = String::new();
    std::fs::File::open(&pid_file)
        .unwrap()
        .read_to_string(&mut data)
        .unwrap();
    let pid: u32 = data.parse().unwrap();
    assert!(pid != child_pid)
}

#[test]
fn double_run() {
    let tmpdir = TempDir::new("double_run").unwrap();
    let pid_file = tmpdir.path().join("pid");
    let first_result = tmpdir.path().join("first");
    let second_result = tmpdir.path().join("second");

    for file in &[&first_result, &second_result] {
        let args = vec![pid_file.to_str().unwrap(), file.to_str().unwrap()];
        run("target/debug/examples/test_double_run", &args);
    }
    std::thread::sleep(std::time::Duration::from_millis(100));

    {
        let mut data = String::new();
        std::fs::File::open(&first_result)
            .unwrap()
            .read_to_string(&mut data)
            .unwrap();
        assert!(data == "ok")
    }

    {
        let mut data = String::new();
        std::fs::File::open(&second_result)
            .unwrap()
            .read_to_string(&mut data)
            .unwrap();
        assert!(data == "error")
    }
}

#[test]
#[cfg(target_os = "macos")]
fn test_uid_gid() {
    let tmpdir = TempDir::new("uid_gid").unwrap();
    let result_file = tmpdir.path().join("result");

    let args = vec!["nobody", "daemon", &result_file.to_str().unwrap()];
    run("target/debug/examples/test_uid_gid", &args);

    let own_uid_gid_string = unsafe { format!("{} {}", libc::getuid(), libc::getgid()) };

    let mut data = String::new();
    std::fs::File::open(&result_file)
        .unwrap()
        .read_to_string(&mut data)
        .unwrap();
    assert!(!data.is_empty());
    assert!(data != own_uid_gid_string)
}

#[test]
fn test_redirect_streams() {
    let tmpdir = TempDir::new("redirect").unwrap();
    let stdout_file = tmpdir.path().join("stdout");
    let stderr_file = tmpdir.path().join("stderr");

    let args = vec![stdout_file.to_str().unwrap(), stderr_file.to_str().unwrap()];
    run("target/debug/examples/test_redirect_streams", &args);

    std::thread::sleep(std::time::Duration::from_millis(100));

    let mut stdout = String::new();
    std::fs::File::open(&stdout_file)
        .unwrap()
        .read_to_string(&mut stdout)
        .unwrap();

    let mut stderr = String::new();
    std::fs::File::open(&stderr_file)
        .unwrap()
        .read_to_string(&mut stderr)
        .unwrap();

    assert_eq!(stdout, "stdout\nnewline\n");
    assert_eq!(stderr, "stderr\nnewline\n");
}

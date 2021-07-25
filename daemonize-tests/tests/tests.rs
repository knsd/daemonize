extern crate daemonize_tests;
extern crate tempfile;

use daemonize_tests::Tester;
use tempfile::TempDir;

#[test]
fn simple() {
    let result = Tester::new().run();
    assert!(result.is_ok())
}

#[test]
fn chdir() {
    let result = Tester::new().run();
    assert_eq!(result.unwrap().cwd.as_str(), "/");

    let result = Tester::new().working_directory("/usr").run();
    assert_eq!(result.unwrap().cwd.as_str(), "/usr");
}

#[test]
fn umask() {
    let tmpdir = TempDir::new().unwrap();
    let path = tmpdir.path().join("umask-test");

    let result = Tester::new().umask(0o222).additional_file(&path).run();
    assert!(result.is_ok());
    assert!(path.metadata().unwrap().permissions().readonly());
}

#[test]
fn pid() {
    let tmpdir = TempDir::new().unwrap();
    let path = tmpdir.path().join("pid");

    let result = Tester::new()
        .pid_file(&path)
        .sleep(std::time::Duration::from_secs(5))
        .run();
    let pid = std::fs::read_to_string(&path).unwrap().parse().unwrap();
    assert_eq!(result.unwrap().pid, pid);

    let result = Tester::new().pid_file(&path).run();
    assert!(result.is_err());
}

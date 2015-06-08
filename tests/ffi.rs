extern crate libc;
extern crate daemonize;

use daemonize::ffi::{get_gid_by_name, get_uid_by_name};

#[test]
fn test_get_gid_by_name() {
    unsafe {
        let gid = get_gid_by_name(&"nobody");
        assert_eq!(gid.unwrap(), -2)
    }
}

#[test]
fn test_get_uid_by_name() {
    unsafe {
        let uid = get_uid_by_name(&"nobody");
        assert_eq!(uid.unwrap(), -2)
    }
}

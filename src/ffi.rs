// Copyright (c) 2016 Fedor Gogolev <knsd@knsd.net>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

extern crate libc;

use std::ffi::CString;

#[repr(C)]
#[allow(dead_code)]
struct passwd {
    pw_name: *const libc::c_char,
    pw_passwd: *const libc::c_char,
    pw_uid: libc::uid_t,
    pw_gid: libc::gid_t,
    pw_gecos: *const libc::c_char,
    pw_dir: *const libc::c_char,
    pw_shell: *const libc::c_char,
}

#[repr(C)]
#[allow(dead_code)]
struct group {
    gr_name: *const libc::c_char,
    gr_passwd: *const libc::c_char,
    gr_gid: libc::gid_t,
    gr_mem: *const *const libc::c_char,
}

extern "C" {
    fn getgrnam(name: *const libc::c_char) -> *const group;
    fn getpwnam(name: *const libc::c_char) -> *const passwd;
    pub fn flock(fd: libc::c_int, operation: libc::c_int) -> libc::c_int;
    pub fn chroot(fd: *const libc::c_char) -> libc::c_int;
}

pub unsafe fn get_gid_by_name(name: &CString) -> Option<libc::gid_t> {
    let ptr = getgrnam(name.as_ptr() as *const libc::c_char);
    if ptr.is_null() {
        None
    } else {
        let s = &*ptr;
        Some(s.gr_gid)
    }
}

pub unsafe fn get_uid_by_name(name: &CString) -> Option<libc::uid_t> {
    let ptr = getpwnam(name.as_ptr() as *const libc::c_char);
    if ptr.is_null() {
        None
    } else {
        let s = &*ptr;
        Some(s.pw_uid)
    }
}

#[cfg(test)]
mod tests {
    use libc;

    use super::{get_gid_by_name, get_uid_by_name};

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    unsafe fn nobody_uid_gid() -> libc::uid_t {
        (u16::max_value() - 1) as libc::uid_t
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    unsafe fn nobody_uid_gid() -> libc::uid_t {
        (u32::max_value() - 1) as libc::uid_t
    }

    #[cfg(target_os = "openbsd")]
    unsafe fn nobody_uid_gid() -> libc::uid_t {
        (i16::max_value()) as libc::uid_t
    }

    #[test]
    fn test_get_gid_by_name() {
        let group_name =
            ::std::ffi::CString::new(match ::std::fs::metadata("/etc/debian_version") {
                Ok(_) => "nogroup",
                Err(_) => "nobody",
            })
            .unwrap();
        unsafe {
            let gid = get_gid_by_name(&group_name);
            assert_eq!(gid, Some(nobody_uid_gid()))
        }
    }

    #[test]
    fn test_get_uid_by_name() {
        let user_name = ::std::ffi::CString::new("nobody").unwrap();
        unsafe {
            let uid = get_uid_by_name(&user_name);
            assert_eq!(uid, Some(nobody_uid_gid()))
        }
    }
}

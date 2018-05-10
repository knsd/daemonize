// Copyright (c) 2016 Fedor Gogolev <knsd@knsd.net>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use libc;

use std::ffi::{CString};

pub unsafe fn get_gid_by_name(name: &CString) -> Option<libc::gid_t> {
    let ptr = libc::getgrnam(name.as_ptr() as *const libc::c_char);
    if ptr.is_null() {
        None
    } else {
        let s = &*ptr;
        Some(s.gr_gid)
    }
}

pub unsafe fn get_uid_by_name(name: &CString) -> Option<libc::uid_t> {
    let ptr = libc::getpwnam(name.as_ptr() as *const libc::c_char);
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
        let group_name = ::std::ffi::CString::new(match ::std::fs::metadata("/etc/debian_version") {
            Ok(_) => "nogroup",
            Err(_) => "nobody",
        }).unwrap();
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

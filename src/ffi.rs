extern crate libc;

#[repr(C)]
#[allow(dead_code)]
struct passwd {
    pw_name:   *const libc::c_char,
    pw_passwd: *const libc::c_char,
    pw_uid:    libc::uid_t,
    pw_gid:    libc::gid_t,
    pw_gecos:  *const libc::c_char,
    pw_dir:    *const libc::c_char,
    pw_shell:  *const libc::c_char,
}

#[repr(C)]
#[allow(dead_code)]
struct group {
    gr_name:   *const libc::c_char,
    gr_passwd: *const libc::c_char,
    gr_gid:    libc::gid_t,
    gr_mem:    *const [*const libc::c_char],
}

extern {
    fn getgrnam(name: *const str) -> *const group;
    fn getpwnam(name: *const str) -> *const passwd;
    pub fn umask(mask: libc::mode_t) -> libc::mode_t;
    pub fn getpid() -> libc::pid_t;
    pub fn flock(fd: libc::c_int, operation: libc::c_int) -> libc::c_int;
}

pub unsafe fn get_gid_by_name(name: &str) -> Option<libc::gid_t> {
    let rname: *const str = name;
    let ptr = getgrnam(rname);
    if ptr.is_null() {
        None
    } else {
        let ref s = *ptr;
        Some(s.gr_gid)
    }
}

pub unsafe fn get_uid_by_name(name: &str) -> Option<libc::uid_t> {
    let rname: *const str = name;
    let ptr = getpwnam(rname);
    if ptr.is_null() {
        None
    } else {
        let ref s = *ptr;
        Some(s.pw_uid)
    }
}


#[doc(hidden)]
pub mod ffi;

extern crate libc;

use std::env::{set_current_dir};
use std::ffi::{CString};
use std::os::unix::ffi::OsStringExt;
use std::path::{Path};
use std::process::{exit};

use libc::funcs::posix88::unistd;
use libc::funcs::posix88::stdio::{fileno};
use libc::funcs::c95::stdio;

use self::ffi::{flock, get_gid_by_name, get_uid_by_name, umask};

macro_rules! tryret {
    ($expr:expr, $ret:expr, $err:expr) => (
        if $expr == -1 {
            return Err($err)
        } else {
            $ret
        }
    )
}

#[derive(Debug)]
pub enum DaemonizeError {
    /// Unable to fork
    Fork,
    /// Unable to create new session
    DetachSession,
    /// Group not found
    GroupNotFound,
    /// Unable to set group
    SetGroup,
    /// User not found
    UserNotFound,
    /// Unable to set user
    SetUser,
    /// Unable to change directory
    ChangeDirectory,
    /// pid_file options contains NUL
    PathContainsNull,
    /// Unable to open pid file
    UnableOpenPidfile,
    /// Unable to lock pid file
    UnableLockPidfile,
    /// Unable to redirect standard streams to /dev/null
    UnableRedirectStreams,
    /// Unable to write self pid to pid file
    UnableWritePid,
}

pub type Result<T> = std::result::Result<T, DaemonizeError>;

#[derive(Debug)]
pub enum User {
    Name(&'static str),
    Id(libc::uid_t),
}

#[derive(Debug)]
pub enum Group {
    Name(&'static str),
    Id(libc::gid_t),
}

#[derive(Debug)]
pub struct DaemonOptions {
    /// Create pid file and lock exclusively
    pub pid_file:          Option<&'static Path>,
    /// Change working directory
    pub directory:         Option<&'static Path>,
    /// Drop privileges to user
    pub user:              Option<User>,
    /// Drop privileges to group
    pub group:             Option<Group>,
}

///
/// Parameter `privileged_action` is an action that will be executed before drop
/// privileges if user or group option is provided.
pub fn daemonize<T>(options: DaemonOptions, privileged_action: &Fn() -> T) -> Result<(T)> {
    unsafe {
        let pid_file_fd = match options.pid_file {
            None => None,
            Some(pid_file) => match create_pid_file(pid_file) {
                Ok(pid_file) => Some(pid_file),
                Err(err) => return Err(err),
            }
        };

        try!(perform_fork());
        try!(set_sid());

        // try!(redirect_standard_streams());

        umask(0o27); // FIXME: WAT

        try!(options.directory.map_or(Ok(()), |d| {
            match set_current_dir(&Path::new(d)) {
                Ok(()) => Ok(()),
                Err(_) => Err(DaemonizeError::ChangeDirectory)
            }
        }));

        let privileged_action_result = privileged_action();

        try!(options.group.map_or(Ok(()), |g| set_group(g)));
        try!(options.user.map_or(Ok(()), |u| set_user(u)));

        match pid_file_fd {
            Some(fd) => try!(write_pid_file(fd)),
            None => ()
        };

        Ok(privileged_action_result)
    }
}

unsafe fn perform_fork() -> Result<()> {
    let pid = unistd::fork();
    if pid < 0 {
        Err(DaemonizeError::Fork)
    } else if pid == 0 {
        Ok(())
    } else {
        exit(0)
    }
}

unsafe fn set_sid() -> Result<()> {
    tryret!(unistd::setsid(), Ok(()), DaemonizeError::DetachSession)
}

unsafe fn redirect_standard_streams() -> Result<()> {
    macro_rules! for_every_stream {
        ($expr:expr) => (
            for stream in [libc::STDIN_FILENO, libc::STDOUT_FILENO, libc::STDERR_FILENO].iter() {
                tryret!($expr(*stream), (), DaemonizeError::UnableRedirectStreams);
            }
        )
    }
    for_every_stream!(unistd::close);

    let devnull_file = stdio::fopen(b"/dev/null" as *const u8 as *const i8,
                                    b"w+" as *const u8 as *const i8);
    if devnull_file.is_null() {
        return Err(DaemonizeError::UnableRedirectStreams)
    };

    let devnull_fd = fileno(devnull_file);
    for_every_stream!(|stream| unistd::dup2(devnull_fd, stream));

    Ok(())
}

unsafe fn set_group(group: Group) -> Result<()> {
    match group {
        Group::Id(id) => {
            tryret!(unistd::setgid(id), Ok(()), DaemonizeError::SetGroup)
        },
        Group::Name(name) => {
            match get_gid_by_name(name) {
                Some(id) => set_group(Group::Id(id)),
                None => Err(DaemonizeError::GroupNotFound)
            }
        }
    }
}

unsafe fn set_user(user: User) -> Result<()> {
    match user {
        User::Id(id) => {
            tryret!(unistd::setuid(id), Ok(()), DaemonizeError::SetUser)
        },
        User::Name(name) => {
            match get_uid_by_name(name) {
                Some(id) => set_user(User::Id(id)),
                None => Err(DaemonizeError::UserNotFound)
            }
        }
    }
}

unsafe fn create_pid_file(path: &Path) -> Result<libc::c_int> {
    let path_cstring = try!({
        match CString::new(path.as_os_str().to_owned().into_vec()) {
            Ok(s) => Ok(s),
            Err(_) => Err(DaemonizeError::PathContainsNull),
        }
    });
    let path_ptr = path_cstring.as_ptr();
    let f = stdio::fopen(path_ptr, b"w" as *const u8 as *const i8);
    if f.is_null() {
        return Err(DaemonizeError::UnableOpenPidfile)
    }

    let fd = fileno(f);
    tryret!(flock(fd, 10), Ok(fd), DaemonizeError::UnableLockPidfile)
}

unsafe fn write_pid_file(fd: libc::c_int) -> Result<()> {
    let pid = unistd::getpid();
    let pid_string = format!("{}", pid);
    let pid_length = pid_string.len() as u64;
    let pid_buf = CString::new(pid_string.into_bytes()).unwrap().as_ptr() as *const libc::c_void;
    if unistd::write(fd, pid_buf, pid_length) < pid_length as i64 {
        Err(DaemonizeError::UnableWritePid)
    } else {
        Ok(())
    }
}

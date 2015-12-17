#[doc(hidden)]
pub mod ffi;

extern crate libc;
#[macro_use] extern crate log;
#[macro_use] extern crate quick_error;

use std::env::{set_current_dir};
use std::error::{Error};
use std::ffi::{CString};
use std::os::unix::ffi::OsStringExt;
use std::mem::{transmute};
use std::path::{Path};
use std::process::{exit};

use libc::funcs::posix88::unistd;
use libc::funcs::posix88::stdio::{fileno};
use libc::funcs::c95::stdio;
pub use libc::{uid_t, gid_t};

use self::ffi::{errno, flock, get_gid_by_name, get_uid_by_name, umask};

macro_rules! tryret {
    ($expr:expr, $ret:expr, $err:expr) => (
        if $expr == -1 {
            return Err($err(errno()))
        } else {
            $ret
        }
    )
}

quick_error! {
    #[derive(Debug)]
    pub enum DaemonizeError {
        /// Unable to fork
        Fork {
            description("unable to fork")
        }
        /// Unable to create new session
        DetachSession(errno: libc::c_int) {
            description("unable to create new session")
        }
        /// Group not found
        GroupNotFound {
            description("group not found")
        }
        /// Unable to set group
        SetGroup(errno: libc::c_int) {
            description("unable to set group")
        }
        /// User not found
        UserNotFound {
            description("user not found")
        }
        /// Unable to set user
        SetUser(errno: libc::c_int) {
            description("unable to set user")
        }
        /// Unable to change directory
        ChangeDirectory {
            description("unable to change directory")
        }
        /// pid_file options contains NUL
        PathContainsNull {
            description("pid_file option contains NUL")
        }
        /// Unable to open pid file
        UnableOpenPidfile {
            description("unable to open pid file")
        }
        /// Unable to lock pid file
        UnableLockPidfile(errno: libc::c_int) {
            description("unable to lock pid file")
        }
        /// Unable to redirect standard streams to /dev/null
        UnableRedirectStreams(errno: libc::c_int) {
            description("unable to redirect standard streams to /dev/null")
        }
        /// Unable to write self pid to pid file
        UnableWritePid {
            description("unable to write self pid to pid file")
        }
    }
}

type Result<T> = std::result::Result<T, DaemonizeError>;

#[derive(Debug)]
pub enum User {
    Name(String),
    Id(uid_t),
}

impl<'a> From<&'a str> for User {
    fn from(t: &'a str) -> User {
        User::Name(t.to_string())
    }
}

impl From<uid_t> for User {
    fn from(t: uid_t) -> User {
        User::Id(t)
    }
}

#[derive(Debug)]
pub enum Group {
    Name(String),
    Id(gid_t),
}

impl<'a> From<&'a str> for Group {
    fn from(t: &'a str) -> Group {
        Group::Name(t.to_string())
    }
}

impl From<gid_t> for Group {
    fn from(t: gid_t) -> Group {
        Group::Id(t)
    }
}

// #[derive(Debug)]
pub struct Daemonize<'a, T> {
    name: &'a str,
    directory: &'a Path,

    pid_file: Option<&'a Path>,
    user: Option<User>,
    group: Option<Group>,
    privileged_action: Box<Fn() -> T>
}

impl<'a> Daemonize<'a, ()> {

    pub fn new(name: &'a str) -> Self {
        Daemonize {
            name: name,
            directory: &Path::new("/"),
            pid_file: None,
            user: None,
            group: None,
            privileged_action: Box::new(|| ())
        }
    }
}

impl<'a, T> Daemonize<'a, T> {

    pub fn set_pid_file<F: AsRef<Path>>(mut self, s: &'a F) -> Self {
        self.pid_file = Some(s.as_ref());
        self
    }

    pub fn set_working_directory<F: AsRef<Path>>(mut self, s: &'a F) -> Self {
        self.directory = s.as_ref();
        self
    }

    pub fn set_user<U: Into<User>>(mut self, user: U) -> Self {
        self.user = Some(user.into());
        self
    }

    pub fn set_group<G: Into<Group>>(mut self, group: G) -> Self {
        self.group = Some(group.into());
        self
    }

    pub fn set_privileged_action<N>(self, action: Box<Fn() -> N>) -> Daemonize<'a, N> {
        let mut new: Daemonize<'a, N> = unsafe { transmute(self) };
        new.privileged_action = action;
        new
    }

    pub fn start(self) -> std::result::Result<T, DaemonizeError> {
        let name = self.name;

        match self.inner_start() {
            Ok(t) => Ok(t),
            Err(t) => {
                error!(target: name, "{}", t.description());
                Err(t)
            }
        }
    }

    fn inner_start(self) -> std::result::Result<T, DaemonizeError> {
        /// Maps an Option<T> to Option<U> by applying a function Fn(T) -> U to a contained value
        /// and try! it's result
        macro_rules! maptry {
            ($expr:expr, $f: expr) => (
                match $expr {
                    None => None,
                    Some(x) => Some(try!($f(x)))
                };
            )
        }

        unsafe {
            let pid_file_fd = maptry!(self.pid_file, create_pid_file);

            try!(perform_fork());
            try!(set_sid());

            try!(redirect_standard_streams());

            umask(0o027);

            try!(set_current_dir(self.directory).map_err(|_| DaemonizeError::ChangeDirectory));

            let privileged_action_result = (self.privileged_action)();

            maptry!(self.group, set_group);
            maptry!(self.user, set_user);
            maptry!(pid_file_fd, write_pid_file);

            Ok(privileged_action_result)
        }
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
        return Err(DaemonizeError::UnableRedirectStreams(libc::ENOENT))
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
            match get_gid_by_name(&name) {
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
            match get_uid_by_name(&name) {
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

mod ffi;

extern crate libc;
#[macro_use] extern crate quick_error;

use std::env::{set_current_dir};
use std::ffi::{CString};
use std::os::unix::ffi::OsStringExt;
use std::mem::{transmute};
use std::path::{Path, PathBuf};
use std::process::{exit};

pub use libc::{uid_t, gid_t, c_int, fopen, write, close, fileno, fork, getpid, setsid, setuid, setgid, dup2};

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
        DetachSession(errno: c_int) {
            description("unable to create new session")
        }
        /// Group not found
        GroupNotFound {
            description("group not found")
        }
        /// group option contains NUL
        GroupContainsNul {
            description("group option contains NUL")
        }
        /// Unable to set group
        SetGroup(errno: c_int) {
            description("unable to set group")
        }
        /// User not found
        UserNotFound {
            description("user not found")
        }
        /// user option contains NUL
        UserContainsNul {
            description("user option contains NUL")
        }
        /// Unable to set user
        SetUser(errno: c_int) {
            description("unable to set user")
        }
        /// Unable to change directory
        ChangeDirectory {
            description("unable to change directory")
        }
        /// pid_file option contains NUL
        PathContainsNul {
            description("pid_file option contains NUL")
        }
        /// Unable to open pid file
        OpenPidfile {
            description("unable to open pid file")
        }
        /// Unable to lock pid file
        LockPidfile(errno: c_int) {
            description("unable to lock pid file")
        }
        /// Unable to chown pid file
        ChownPidfile(errno: c_int) {
            description("unable to chown pid file")
        }
        /// Unable to redirect standard streams to /dev/null
        RedirectStreams(errno: c_int) {
            description("unable to redirect standard streams to /dev/null")
        }
        /// Unable to write self pid to pid file
        WritePid {
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
pub struct Daemonize<T> {
    directory: PathBuf,
    pid_file: Option<PathBuf>,
    chown_pid_file: bool,
    user: Option<User>,
    group: Option<Group>,
    privileged_action: Box<Fn() -> T>
}

impl Daemonize<()> {

    pub fn new() -> Self {
        Daemonize {
            directory: Path::new("/").to_owned(),
            pid_file: None,
            chown_pid_file: true,
            user: None,
            group: None,
            privileged_action: Box::new(|| ()),
        }
    }
}

impl<T> Daemonize<T> {

    pub fn pid_file<F: AsRef<Path>>(mut self, path: F) -> Self {
        self.pid_file = Some(path.as_ref().to_owned());
        self
    }

    pub fn chown_pid_file(mut self, chown: bool) -> Self {
        self.chown_pid_file = chown;
        self
    }

    pub fn working_directory<F: AsRef<Path>>(mut self, path: F) -> Self {
        self.directory = path.as_ref().to_owned();
        self
    }

    pub fn user<U: Into<User>>(mut self, user: U) -> Self {
        self.user = Some(user.into());
        self
    }

    pub fn group<G: Into<Group>>(mut self, group: G) -> Self {
        self.group = Some(group.into());
        self
    }

    pub fn privileged_action<N, F: Fn() -> N + Sized + 'static>(self, action: F) -> Daemonize<N> {
        let mut new: Daemonize<N> = unsafe { transmute(self) };
        new.privileged_action = Box::new(action);
        new
    }

    pub fn start(self) -> std::result::Result<T, DaemonizeError> {
        // Maps an Option<T> to Option<U> by applying a function Fn(T) -> U to a contained value
        // and try! it's result
        macro_rules! maptry {
            ($expr:expr, $f: expr) => (
                match $expr {
                    None => None,
                    Some(x) => Some(try!($f(x)))
                };
            )
        }

        unsafe {
            let pid_file_fd = maptry!(self.pid_file.clone(), create_pid_file);

            try!(perform_fork());
            try!(set_sid());

            try!(redirect_standard_streams());

            umask(0o027);

            try!(set_current_dir(self.directory).map_err(|_| DaemonizeError::ChangeDirectory));

            let uid = maptry!(self.user, get_user);
            let gid = maptry!(self.group, get_group);

            if self.chown_pid_file {
                let args: Option<(PathBuf, uid_t, gid_t)> = match (self.pid_file, gid, uid) {
                    (Some(pid), Some(uid), Some(gid)) => Some((pid, uid, gid)),
                    (Some(pid), None, Some(gid)) => Some((pid, uid_t::max_value() - 1, gid)),
                    (Some(pid), Some(uid), None) => Some((pid, uid, gid_t::max_value() - 1)),
                    // Or pid file is not provided, or both user and group
                    _ => None
                };

                maptry!(args, |(pid, uid, gid)| chown_pid_file(pid, uid, gid));
            }

            let privileged_action_result = (self.privileged_action)();

            maptry!(uid, set_user);
            maptry!(gid, set_group);

            maptry!(pid_file_fd, write_pid_file);

            Ok(privileged_action_result)
        }
    }

}

unsafe fn perform_fork() -> Result<()> {
    let pid = fork();
    if pid < 0 {
        Err(DaemonizeError::Fork)
    } else if pid == 0 {
        Ok(())
    } else {
        exit(0)
    }
}

unsafe fn set_sid() -> Result<()> {
    tryret!(setsid(), Ok(()), DaemonizeError::DetachSession)
}

unsafe fn redirect_standard_streams() -> Result<()> {
    macro_rules! for_every_stream {
        ($expr:expr) => (
            for stream in [libc::STDIN_FILENO, libc::STDOUT_FILENO, libc::STDERR_FILENO].iter() {
                tryret!($expr(*stream), (), DaemonizeError::RedirectStreams);
            }
        )
    }
    for_every_stream!(close);

    let devnull_path_ptr = try!(create_path("/dev/null"));


    let devnull_file = fopen(devnull_path_ptr,
                                    b"w+" as *const u8 as *const libc::c_char);
    if devnull_file.is_null() {
        return Err(DaemonizeError::RedirectStreams(libc::ENOENT))
    };

    let devnull_fd = fileno(devnull_file);
    for_every_stream!(|stream| dup2(devnull_fd, stream));

    Ok(())
}

unsafe fn get_group(group: Group) -> Result<gid_t> {
    match group {
        Group::Id(id) => Ok(id),
        Group::Name(name) => {
            let s = try!(CString::new(name).map_err(|_| DaemonizeError::GroupContainsNul));
            match get_gid_by_name(&s) {
                Some(id) => get_group(Group::Id(id)),
                None => Err(DaemonizeError::GroupNotFound)
            }
        }
    }
}

unsafe fn set_group(group: gid_t) -> Result<()> {
    tryret!(setgid(group), Ok(()), DaemonizeError::SetGroup)
}

unsafe fn get_user(user: User) -> Result<uid_t> {
    match user {
        User::Id(id) => Ok(id),
        User::Name(name) => {
            let s = try!(CString::new(name).map_err(|_| DaemonizeError::UserContainsNul));
            match get_uid_by_name(&s) {
                Some(id) => get_user(User::Id(id)),
                None => Err(DaemonizeError::UserNotFound)
            }
        }
    }
}

unsafe fn set_user(user: uid_t) -> Result<()> {
    tryret!(setuid(user), Ok(()), DaemonizeError::SetUser)
}

unsafe fn create_pid_file(path: PathBuf) -> Result<libc::c_int> {
    let path_ptr = try!(create_path(path));

    let f = fopen(path_ptr, b"w" as *const u8 as *const libc::c_char);
    if f.is_null() {
        return Err(DaemonizeError::OpenPidfile)
    }

    let fd = fileno(f);
    tryret!(flock(fd, 10), Ok(fd), DaemonizeError::LockPidfile)
}

unsafe fn chown_pid_file(path: PathBuf, uid: uid_t, gid: gid_t) -> Result<()> {
    let path_ptr = try!(create_path(path));
    tryret!(libc::chown(path_ptr, uid, gid), Ok(()), DaemonizeError::ChownPidfile)
}

unsafe fn write_pid_file(fd: libc::c_int) -> Result<()> {
    let pid = getpid();
    let pid_string = format!("{}", pid);
    let pid_length = pid_string.len() as usize;
    let pid_buf = CString::new(pid_string.into_bytes()).unwrap().as_ptr() as *const libc::c_void;
    if write(fd, pid_buf, pid_length) < pid_length as isize {
        Err(DaemonizeError::WritePid)
    } else {
        Ok(())
    }
}

unsafe fn create_path<F: AsRef<Path>>(path: F) -> Result<*const libc::c_char> {
    let path_cstring = try!(CString::new(path.as_ref().as_os_str().to_owned().into_vec()).map_err(|_| DaemonizeError::PathContainsNul));
    Ok(path_cstring.as_ptr())
}

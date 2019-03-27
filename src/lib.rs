// Copyright (c) 2016 Fedor Gogolev <knsd@knsd.net>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//!
//! daemonize is a library for writing system daemons. Inspired by the Python library [thesharp/daemonize](https://github.com/thesharp/daemonize).
//!
//! The respository is located at https://github.com/knsd/daemonize/.
//!
//! Usage example:
//!
//! ```
//! extern crate daemonize;
//!
//! use std::fs::File;
//!
//! use daemonize::Daemonize;
//!
//! fn main() {
//!     let stdout = File::create("/tmp/daemon.out").unwrap();
//!     let stderr = File::create("/tmp/daemon.err").unwrap();
//!
//!     let daemonize = Daemonize::new()
//!         .pid_file("/tmp/test.pid") // Every method except `new` and `start`
//!         .chown_pid_file(true)      // is optional, see `Daemonize` documentation
//!         .working_directory("/tmp") // for default behaviour.
//!         .user("nobody")
//!         .group("daemon") // Group name
//!         .group(2)        // or group id.
//!         .umask(0o777)    // Set umask, `0o027` by default.
//!         .stdout(stdout)  // Redirect stdout to `/tmp/daemon.out`.
//!         .stderr(stderr)  // Redirect stderr to `/tmp/daemon.err`.
//!         .exit_action(|| println!("Executed before master process exits"))
//!         .privileged_action(|| "Executed before drop privileges");
//!
//!     match daemonize.start() {
//!         Ok(_) => println!("Success, daemonized"),
//!         Err(e) => eprintln!("Error, {}", e),
//!     }
//! }
//! ```

mod ffi;

extern crate boxfnonce;
extern crate libc;

use std::env::set_current_dir;
use std::ffi::CString;
use std::fmt;
use std::fs::File;
use std::io;
use std::mem::transmute;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::exit;

use boxfnonce::BoxFnOnce;
use libc::{
    c_int, close, dup2, fork, ftruncate, getpid, open, setgid, setsid, setuid, umask, write,
    LOCK_EX, LOCK_NB,
};
pub use libc::{gid_t, mode_t, uid_t};

use self::ffi::{chroot, flock, get_gid_by_name, get_uid_by_name};

macro_rules! tryret {
    ($expr:expr, $ret:expr, $err:expr) => {
        if $expr == -1 {
            return Err($err(errno()));
        } else {
            #[allow(clippy::unused_unit)]
            {
                $ret
            }
        }
    };
}

pub type Errno = c_int;

/// This error type for `Daemonize` `start` method.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum DaemonizeError {
    /// Unable to fork
    Fork,
    /// Unable to create new session
    DetachSession(Errno),
    /// Unable to resolve group name to group id
    GroupNotFound,
    /// Group option contains NUL
    GroupContainsNul,
    /// Unable to set group
    SetGroup(Errno),
    /// Unable to resolve user name to user id
    UserNotFound,
    /// User option contains NUL
    UserContainsNul,
    /// Unable to set user
    SetUser(Errno),
    /// Unable to change directory
    ChangeDirectory,
    /// pid_file option contains NUL
    PathContainsNul,
    /// Unable to open pid file
    OpenPidfile,
    /// Unable to lock pid file
    LockPidfile(Errno),
    /// Unable to chown pid file
    ChownPidfile(Errno),
    /// Unable to redirect standard streams to /dev/null
    RedirectStreams(Errno),
    /// Unable to write self pid to pid file
    WritePid,
    /// Unable to chroot
    Chroot(Errno),
    // Hints that destructuring should not be exhaustive.
    // This enum may grow additional variants, so this makes sure clients
    // don't count on exhaustive matching. Otherwise, adding a new variant
    // could break existing code.
    #[doc(hidden)]
    __Nonexhaustive,
}

impl DaemonizeError {
    fn __description(&self) -> &str {
        match *self {
            DaemonizeError::Fork => "unable to fork",
            DaemonizeError::DetachSession(_) => "unable to create new session",
            DaemonizeError::GroupNotFound => "unable to resolve group name to group id",
            DaemonizeError::GroupContainsNul => "group option contains NUL",
            DaemonizeError::SetGroup(_) => "unable to set group",
            DaemonizeError::UserNotFound => "unable to resolve user name to user id",
            DaemonizeError::UserContainsNul => "user option contains NUL",
            DaemonizeError::SetUser(_) => "unable to set user",
            DaemonizeError::ChangeDirectory => "unable to change directory",
            DaemonizeError::PathContainsNul => "pid_file option contains NUL",
            DaemonizeError::OpenPidfile => "unable to open pid file",
            DaemonizeError::LockPidfile(_) => "unable to lock pid file",
            DaemonizeError::ChownPidfile(_) => "unable to chown pid file",
            DaemonizeError::RedirectStreams(_) => {
                "unable to redirect standard streams to /dev/null"
            }
            DaemonizeError::WritePid => "unable to write self pid to pid file",
            DaemonizeError::Chroot(_) => "unable to chroot into directory",
            DaemonizeError::__Nonexhaustive => unreachable!(),
        }
    }
}

impl std::fmt::Display for DaemonizeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.__description().fmt(f)
    }
}

impl std::error::Error for DaemonizeError {
    fn description(&self) -> &str {
        self.__description()
    }
}

type Result<T> = std::result::Result<T, DaemonizeError>;

/// Expects system user id or name. If name is provided it will be resolved to id later.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum User {
    Name(String),
    Id(uid_t),
}

impl<'a> From<&'a str> for User {
    fn from(t: &'a str) -> User {
        User::Name(t.to_owned())
    }
}

impl From<uid_t> for User {
    fn from(t: uid_t) -> User {
        User::Id(t)
    }
}

/// Expects system group id or name. If name is provided it will be resolved to id later.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum Group {
    Name(String),
    Id(gid_t),
}

impl<'a> From<&'a str> for Group {
    fn from(t: &'a str) -> Group {
        Group::Name(t.to_owned())
    }
}

impl From<gid_t> for Group {
    fn from(t: gid_t) -> Group {
        Group::Id(t)
    }
}

#[derive(Debug)]
enum StdioImp {
    Devnull,
    RedirectToFile(File),
}

/// Describes what to do with a standard I/O stream for a child process.
#[derive(Debug)]
pub struct Stdio {
    inner: StdioImp,
}

impl Stdio {
    fn devnull() -> Self {
        Self {
            inner: StdioImp::Devnull,
        }
    }
}

impl From<File> for Stdio {
    fn from(file: File) -> Self {
        Self {
            inner: StdioImp::RedirectToFile(file),
        }
    }
}

/// Daemonization options.
///
/// Fork the process in the background, disassociate from its process group and the control terminal.
/// Change umask value to `0o027`, redirect all standard streams to `/dev/null`. Change working
/// directory to `/` or provided value.
///
/// Optionally:
///
///   * maintain and lock the pid-file;
///   * drop user privileges;
///   * drop group privileges;
///   * change root directory;
///   * change the pid-file ownership to provided user (and/or) group;
///   * execute any provided action just before dropping privileges.
///
pub struct Daemonize<T> {
    directory: PathBuf,
    pid_file: Option<PathBuf>,
    chown_pid_file: bool,
    user: Option<User>,
    group: Option<Group>,
    umask: mode_t,
    root: Option<PathBuf>,
    privileged_action: BoxFnOnce<'static, (), T>,
    exit_action: BoxFnOnce<'static, (), ()>,
    stdin: Stdio,
    stdout: Stdio,
    stderr: Stdio,
}

impl<T> fmt::Debug for Daemonize<T> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("Daemonize")
            .field("directory", &self.directory)
            .field("pid_file", &self.pid_file)
            .field("chown_pid_file", &self.chown_pid_file)
            .field("user", &self.user)
            .field("group", &self.group)
            .field("umask", &self.umask)
            .field("root", &self.root)
            .field("stdin", &self.stdin)
            .field("stdout", &self.stdout)
            .field("stderr", &self.stderr)
            .finish()
    }
}

impl Daemonize<()> {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Daemonize {
            directory: Path::new("/").to_owned(),
            pid_file: None,
            chown_pid_file: false,
            user: None,
            group: None,
            umask: 0o027,
            privileged_action: BoxFnOnce::new(|| ()),
            exit_action: BoxFnOnce::new(|| ()),
            root: None,
            stdin: Stdio::devnull(),
            stdout: Stdio::devnull(),
            stderr: Stdio::devnull(),
        }
    }
}

impl<T> Daemonize<T> {
    /// Create pid-file at `path`, lock it exclusive and write daemon pid.
    pub fn pid_file<F: AsRef<Path>>(mut self, path: F) -> Self {
        self.pid_file = Some(path.as_ref().to_owned());
        self
    }

    /// If `chown` is true, daemonize will change the pid-file ownership, if user or group are provided
    pub fn chown_pid_file(mut self, chown: bool) -> Self {
        self.chown_pid_file = chown;
        self
    }

    /// Change working directory to `path` or `/` by default.
    pub fn working_directory<F: AsRef<Path>>(mut self, path: F) -> Self {
        self.directory = path.as_ref().to_owned();
        self
    }

    /// Drop privileges to `user`.
    pub fn user<U: Into<User>>(mut self, user: U) -> Self {
        self.user = Some(user.into());
        self
    }

    /// Drop privileges to `group`.
    pub fn group<G: Into<Group>>(mut self, group: G) -> Self {
        self.group = Some(group.into());
        self
    }

    /// Change umask to `mask` or `0o027` by default.
    pub fn umask(mut self, mask: mode_t) -> Self {
        self.umask = mask;
        self
    }

    /// Change root to `path`
    pub fn chroot<F: AsRef<Path>>(mut self, path: F) -> Self {
        self.root = Some(path.as_ref().to_owned());
        self
    }

    /// Execute `action` just before dropping privileges. Most common usecase is to open listening socket.
    /// Result of `action` execution will be returned by `start` method.
    pub fn privileged_action<N, F: FnOnce() -> N + 'static>(self, action: F) -> Daemonize<N> {
        let mut new: Daemonize<N> = unsafe { transmute(self) };
        new.privileged_action = BoxFnOnce::new(action);
        new
    }

    /// Execute `action` just before exiting the parent process. Most common usecase is to synchronize with
    /// forked processes.
    pub fn exit_action<F: FnOnce() + 'static>(mut self, action: F) -> Daemonize<T> {
        self.exit_action = BoxFnOnce::new(action);
        self
    }

    /// Configuration for the child process's standard output stream.
    pub fn stdout<S: Into<Stdio>>(mut self, stdio: S) -> Self {
        self.stdout = stdio.into();
        self
    }

    /// Configuration for the child process's standard error stream.
    pub fn stderr<S: Into<Stdio>>(mut self, stdio: S) -> Self {
        self.stderr = stdio.into();
        self
    }

    /// Start daemonization process.
    pub fn start(self) -> std::result::Result<T, DaemonizeError> {
        // Maps an Option<T> to Option<U> by applying a function Fn(T) -> Result<U, DaemonizeError>
        // to a contained value and try! it's result
        macro_rules! maptry {
            ($expr:expr, $f: expr) => {
                match $expr {
                    None => None,
                    Some(x) => Some(try!($f(x))),
                };
            };
        }

        unsafe {
            let pid_file_fd = maptry!(self.pid_file.clone(), create_pid_file);

            perform_fork(Some(self.exit_action))?;

            set_current_dir(&self.directory).map_err(|_| DaemonizeError::ChangeDirectory)?;
            set_sid()?;
            umask(self.umask);

            perform_fork(None)?;

            redirect_standard_streams(self.stdin, self.stdout, self.stderr)?;

            let uid = maptry!(self.user, get_user);
            let gid = maptry!(self.group, get_group);

            if self.chown_pid_file {
                let args: Option<(PathBuf, uid_t, gid_t)> = match (self.pid_file, uid, gid) {
                    (Some(pid), Some(uid), Some(gid)) => Some((pid, uid, gid)),
                    (Some(pid), None, Some(gid)) => Some((pid, uid_t::max_value() - 1, gid)),
                    (Some(pid), Some(uid), None) => Some((pid, uid, gid_t::max_value() - 1)),
                    // Or pid file is not provided, or both user and group
                    _ => None,
                };

                maptry!(args, |(pid, uid, gid)| chown_pid_file(pid, uid, gid));
            }

            let privileged_action_result = self.privileged_action.call();

            maptry!(self.root, change_root);

            maptry!(gid, set_group);
            maptry!(uid, set_user);

            maptry!(pid_file_fd, write_pid_file);

            Ok(privileged_action_result)
        }
    }
}

unsafe fn perform_fork(exit_action: Option<BoxFnOnce<'static, (), ()>>) -> Result<()> {
    let pid = fork();
    if pid < 0 {
        Err(DaemonizeError::Fork)
    } else if pid == 0 {
        Ok(())
    } else {
        if let Some(exit_action) = exit_action {
            exit_action.call()
        }
        exit(0)
    }
}

unsafe fn set_sid() -> Result<()> {
    tryret!(setsid(), Ok(()), DaemonizeError::DetachSession)
}

unsafe fn redirect_standard_streams(stdin: Stdio, stdout: Stdio, stderr: Stdio) -> Result<()> {
    let devnull_fd = open(b"/dev/null\0" as *const [u8; 10] as _, libc::O_RDWR);
    if -1 == devnull_fd {
        return Err(DaemonizeError::RedirectStreams(errno()));
    }

    let process_stdio = |fd, stdio: Stdio| {
        tryret!(close(fd), (), DaemonizeError::RedirectStreams);
        match stdio.inner {
            StdioImp::Devnull => {
                tryret!(dup2(devnull_fd, fd), (), DaemonizeError::RedirectStreams);
            }
            StdioImp::RedirectToFile(file) => {
                let raw_fd = file.as_raw_fd();
                tryret!(dup2(raw_fd, fd), (), DaemonizeError::RedirectStreams);
            }
        };
        Ok(())
    };

    process_stdio(libc::STDIN_FILENO, stdin)?;
    process_stdio(libc::STDOUT_FILENO, stdout)?;
    process_stdio(libc::STDERR_FILENO, stderr)?;

    tryret!(close(devnull_fd), (), DaemonizeError::RedirectStreams);

    Ok(())
}

unsafe fn get_group(group: Group) -> Result<gid_t> {
    match group {
        Group::Id(id) => Ok(id),
        Group::Name(name) => {
            let s = CString::new(name).map_err(|_| DaemonizeError::GroupContainsNul)?;
            match get_gid_by_name(&s) {
                Some(id) => get_group(Group::Id(id)),
                None => Err(DaemonizeError::GroupNotFound),
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
            let s = CString::new(name).map_err(|_| DaemonizeError::UserContainsNul)?;
            match get_uid_by_name(&s) {
                Some(id) => get_user(User::Id(id)),
                None => Err(DaemonizeError::UserNotFound),
            }
        }
    }
}

unsafe fn set_user(user: uid_t) -> Result<()> {
    tryret!(setuid(user), Ok(()), DaemonizeError::SetUser)
}

unsafe fn create_pid_file(path: PathBuf) -> Result<libc::c_int> {
    let path_c = pathbuf_into_cstring(path)?;

    let fd = open(path_c.as_ptr(), libc::O_WRONLY | libc::O_CREAT, 0o666);
    if -1 == fd {
        return Err(DaemonizeError::OpenPidfile);
    }

    tryret!(
        flock(fd, LOCK_EX | LOCK_NB),
        Ok(fd),
        DaemonizeError::LockPidfile
    )
}

unsafe fn chown_pid_file(path: PathBuf, uid: uid_t, gid: gid_t) -> Result<()> {
    let path_c = pathbuf_into_cstring(path)?;
    tryret!(
        libc::chown(path_c.as_ptr(), uid, gid),
        Ok(()),
        DaemonizeError::ChownPidfile
    )
}

unsafe fn write_pid_file(fd: libc::c_int) -> Result<()> {
    let pid = getpid();
    let pid_buf = format!("{}", pid).into_bytes();
    let pid_length = pid_buf.len();
    let pid_c = CString::new(pid_buf).unwrap();
    if -1 == ftruncate(fd, 0) {
        return Err(DaemonizeError::WritePid);
    }
    if write(fd, pid_c.as_ptr() as *const libc::c_void, pid_length) < pid_length as isize {
        Err(DaemonizeError::WritePid)
    } else {
        Ok(())
    }
}

unsafe fn change_root(path: PathBuf) -> Result<()> {
    let path_c = pathbuf_into_cstring(path)?;

    if chroot(path_c.as_ptr()) == 0 {
        Ok(())
    } else {
        Err(DaemonizeError::Chroot(errno()))
    }
}

fn pathbuf_into_cstring(path: PathBuf) -> Result<CString> {
    CString::new(path.into_os_string().into_vec()).map_err(|_| DaemonizeError::PathContainsNul)
}

fn errno() -> Errno {
    io::Error::last_os_error().raw_os_error().expect("errno")
}

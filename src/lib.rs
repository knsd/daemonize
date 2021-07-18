// Copyright (c) 2016 Fedor Gogolev <knsd@knsd.net>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//!
//! daemonize is a library for writing system daemons. Inspired by the Python library [thesharp/daemonize](https://hub.com/thesharp/daemonize).
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

mod error;
mod ffi;

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

pub use libc::mode_t;
use libc::{
    close, dup2, fork, ftruncate, getpid, open, setgid, setsid, setuid, umask, write, LOCK_EX,
    LOCK_NB,
};

use self::error::{Errno, ErrorKind};
use self::ffi::{chroot, flock, get_gid_by_name, get_uid_by_name};

pub use self::error::Error;

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

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
enum UserImpl {
    Name(String),
    Id(libc::uid_t),
}

/// Expects system user id or name. If name is provided it will be resolved to id later.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct User {
    inner: UserImpl,
}

impl From<&str> for User {
    fn from(t: &str) -> User {
        User {
            inner: UserImpl::Name(t.to_owned()),
        }
    }
}

impl From<libc::uid_t> for User {
    fn from(t: libc::uid_t) -> User {
        User {
            inner: UserImpl::Id(t),
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
enum GroupImpl {
    Name(String),
    Id(libc::uid_t),
}

/// Expects system group id or name. If name is provided it will be resolved to id later.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct Group {
    inner: GroupImpl,
}

impl From<&str> for Group {
    fn from(t: &str) -> Group {
        Group {
            inner: GroupImpl::Name(t.to_owned()),
        }
    }
}

impl From<libc::gid_t> for Group {
    fn from(t: libc::gid_t) -> Group {
        Group {
            inner: GroupImpl::Id(t),
        }
    }
}

#[derive(Debug)]
enum StdioImpl {
    Devnull,
    RedirectToFile(File),
}

/// Describes what to do with a standard I/O stream for a child process.
#[derive(Debug)]
pub struct Stdio {
    inner: StdioImpl,
}

impl Stdio {
    fn devnull() -> Self {
        Self {
            inner: StdioImpl::Devnull,
        }
    }
}

impl From<File> for Stdio {
    fn from(file: File) -> Self {
        Self {
            inner: StdioImpl::RedirectToFile(file),
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
    privileged_action: Box<dyn FnOnce() -> T>,
    exit_action: Box<dyn FnOnce()>,
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

impl Default for Daemonize<()> {
    fn default() -> Self {
        Self::new()
    }
}

impl Daemonize<()> {
    pub fn new() -> Self {
        Daemonize {
            directory: Path::new("/").to_owned(),
            pid_file: None,
            chown_pid_file: false,
            user: None,
            group: None,
            umask: 0o027,
            privileged_action: Box::new(|| ()),
            exit_action: Box::new(|| ()),
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
        new.privileged_action = Box::new(action);
        new
    }

    /// Execute `action` just before exiting the parent process. Most common usecase is to synchronize with
    /// forked processes.
    pub fn exit_action<F: FnOnce() + 'static>(mut self, action: F) -> Daemonize<T> {
        self.exit_action = Box::new(action);
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
    pub fn start(self) -> std::result::Result<T, Error> {
        // Maps an Option<T> to Option<U> by applying a function Fn(T) -> Result<U, ErrorKind>
        // to a contained value and try! it's result
        macro_rules! maptry {
            ($expr:expr, $f: expr) => {
                match $expr {
                    None => None,
                    Some(x) => Some($f(x)?),
                };
            };
        }

        unsafe {
            let pid_file_fd = maptry!(self.pid_file.clone(), create_pid_file);

            perform_fork(Some(self.exit_action))?;

            set_current_dir(&self.directory).map_err(|_| ErrorKind::ChangeDirectory(errno()))?;
            set_sid()?;
            umask(self.umask);

            perform_fork(None)?;

            redirect_standard_streams(self.stdin, self.stdout, self.stderr)?;

            let uid = maptry!(self.user, get_user);
            let gid = maptry!(self.group, get_group);

            if self.chown_pid_file {
                let args: Option<(PathBuf, libc::uid_t, libc::gid_t)> =
                    match (self.pid_file, uid, gid) {
                        (Some(pid), Some(uid), Some(gid)) => Some((pid, uid, gid)),
                        (Some(pid), None, Some(gid)) => Some((pid, libc::uid_t::MAX - 1, gid)),
                        (Some(pid), Some(uid), None) => Some((pid, uid, libc::gid_t::MAX - 1)),
                        // Or pid file is not provided, or both user and group
                        _ => None,
                    };

                maptry!(args, |(pid, uid, gid)| chown_pid_file(pid, uid, gid));
            }

            let privileged_action_result = (self.privileged_action)();

            maptry!(self.root, change_root);

            maptry!(gid, set_group);
            maptry!(uid, set_user);

            maptry!(pid_file_fd, write_pid_file);

            Ok(privileged_action_result)
        }
    }
}

unsafe fn perform_fork(exit_action: Option<Box<dyn FnOnce()>>) -> Result<(), ErrorKind> {
    let pid = fork();
    if pid < 0 {
        Err(ErrorKind::Fork(errno()))
    } else if pid == 0 {
        Ok(())
    } else {
        if let Some(exit_action) = exit_action {
            exit_action()
        }
        exit(0)
    }
}

unsafe fn set_sid() -> Result<(), ErrorKind> {
    tryret!(setsid(), Ok(()), ErrorKind::DetachSession)
}

unsafe fn redirect_standard_streams(
    stdin: Stdio,
    stdout: Stdio,
    stderr: Stdio,
) -> Result<(), ErrorKind> {
    let devnull_fd = open(b"/dev/null\0" as *const [u8; 10] as _, libc::O_RDWR);
    if -1 == devnull_fd {
        return Err(ErrorKind::OpenDevnull(errno()));
    }

    let process_stdio = |fd, stdio: Stdio| {
        match stdio.inner {
            StdioImpl::Devnull => {
                tryret!(dup2(devnull_fd, fd), (), ErrorKind::RedirectStreams);
            }
            StdioImpl::RedirectToFile(file) => {
                let raw_fd = file.as_raw_fd();
                tryret!(dup2(raw_fd, fd), (), ErrorKind::RedirectStreams);
            }
        };
        Ok(())
    };

    process_stdio(libc::STDIN_FILENO, stdin)?;
    process_stdio(libc::STDOUT_FILENO, stdout)?;
    process_stdio(libc::STDERR_FILENO, stderr)?;

    tryret!(close(devnull_fd), (), ErrorKind::CloseDevnull);

    Ok(())
}

unsafe fn get_group(group: Group) -> Result<libc::gid_t, ErrorKind> {
    match group.inner {
        GroupImpl::Id(id) => Ok(id),
        GroupImpl::Name(name) => {
            let s = CString::new(name).map_err(|_| ErrorKind::GroupContainsNul)?;
            match get_gid_by_name(&s) {
                Some(id) => get_group(id.into()),
                None => Err(ErrorKind::GroupNotFound),
            }
        }
    }
}

unsafe fn set_group(group: libc::gid_t) -> Result<(), ErrorKind> {
    tryret!(setgid(group), Ok(()), ErrorKind::SetGroup)
}

unsafe fn get_user(user: User) -> Result<libc::uid_t, ErrorKind> {
    match user.inner {
        UserImpl::Id(id) => Ok(id),
        UserImpl::Name(name) => {
            let s = CString::new(name).map_err(|_| ErrorKind::UserContainsNul)?;
            match get_uid_by_name(&s) {
                Some(id) => get_user(id.into()),
                None => Err(ErrorKind::UserNotFound),
            }
        }
    }
}

unsafe fn set_user(user: libc::uid_t) -> Result<(), ErrorKind> {
    tryret!(setuid(user), Ok(()), ErrorKind::SetUser)
}

unsafe fn create_pid_file(path: PathBuf) -> Result<libc::c_int, ErrorKind> {
    let path_c = pathbuf_into_cstring(path)?;

    #[cfg(target_os = "redox")]
    let open_flags = libc::O_CLOEXEC | libc::O_WRONLY | libc::O_CREAT;

    #[cfg(not(target_os = "redox"))]
    let open_flags = libc::O_WRONLY | libc::O_CREAT;

    let fd = open(path_c.as_ptr(), open_flags, 0o666);

    if fd == -1 {
        return Err(ErrorKind::OpenPidfile(errno()));
    }

    if cfg!(not(target_os = "redox")) {
        let flags = libc::fcntl(fd, libc::F_GETFD);
        if flags == -1 {
            return Err(ErrorKind::GetPidfileFlags(errno()));
        }

        if libc::fcntl(fd, libc::F_SETFD, flags | libc::FD_CLOEXEC) == -1 {
            return Err(ErrorKind::SetPidfileFlags(errno()));
        };
    };

    tryret!(flock(fd, LOCK_EX | LOCK_NB), Ok(fd), ErrorKind::LockPidfile)
}

unsafe fn chown_pid_file(
    path: PathBuf,
    uid: libc::uid_t,
    gid: libc::gid_t,
) -> Result<(), ErrorKind> {
    let path_c = pathbuf_into_cstring(path)?;
    tryret!(
        libc::chown(path_c.as_ptr(), uid, gid),
        Ok(()),
        ErrorKind::ChownPidfile
    )
}

unsafe fn write_pid_file(fd: libc::c_int) -> Result<(), ErrorKind> {
    let pid = getpid();
    let pid_buf = format!("{}", pid).into_bytes();
    let pid_length = pid_buf.len();
    let pid_c = CString::new(pid_buf).unwrap();
    if -1 == ftruncate(fd, 0) {
        return Err(ErrorKind::WritePid);
    }
    if write(fd, pid_c.as_ptr() as *const libc::c_void, pid_length) < pid_length as isize {
        Err(ErrorKind::WritePid)
    } else {
        Ok(())
    }
}

unsafe fn change_root(path: PathBuf) -> Result<(), ErrorKind> {
    let path_c = pathbuf_into_cstring(path)?;

    if chroot(path_c.as_ptr()) == 0 {
        Ok(())
    } else {
        Err(ErrorKind::Chroot(errno()))
    }
}

fn pathbuf_into_cstring(path: PathBuf) -> Result<CString, ErrorKind> {
    CString::new(path.into_os_string().into_vec()).map_err(|_| ErrorKind::PathContainsNul)
}

fn errno() -> Errno {
    io::Error::last_os_error().raw_os_error().expect("errno")
}

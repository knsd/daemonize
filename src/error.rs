use libc::c_int;

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
    // "unable to encode group"
    EncodeGroup,
    // "unable to encode user"
    EncodeUser,
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
            DaemonizeError::EncodeGroup => "unable to encode group",
            DaemonizeError::EncodeUser => "unable to encode user",
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

pub type Result<T> = std::result::Result<T, DaemonizeError>;
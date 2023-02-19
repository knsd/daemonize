extern crate arraystring;
extern crate daemonize;

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::str::FromStr;

use daemonize::{Daemonize, Error, Outcome};

const ARG_PID_FILE: &str = "--pid-file";
const ARG_CHOWN_PID_FILE: &str = "--chown-pid-file";
const ARG_WORKING_DIRECTORY: &str = "--working-directory";
const ARG_USER_STRING: &str = "--user-string";
const ARG_USER_NUM: &str = "--user-num";
const ARG_GROUP_STRING: &str = "--group-string";
const ARG_GROUP_NUM: &str = "--group-num";
const ARG_UMASK: &str = "--umask";
const ARG_CHROOT: &str = "--chroot";
const ARG_STDOUT: &str = "--stdout";
const ARG_STDERR: &str = "--stderr";
const ARG_ADDITIONAL_FILE: &str = "--additional-file";
const ARG_SLEEP_MS: &str = "--sleep-ms";
const ARG_HUMAN_READABLE: &str = "--human-readable";

pub const STDOUT_DATA: &str = "stdout data";
pub const STDERR_DATA: &str = "stderr data";
pub const ADDITIONAL_FILE_DATA: &str = "additional file data";

const TESTER_PATH: &str = "../target/debug/examples/tester";

const MAX_WAIT_DURATION: std::time::Duration = std::time::Duration::from_secs(5);

const DATA_LEN: usize = std::mem::size_of::<Result<EnvData, Error>>();

pub struct Tester {
    command: Command,
}

impl Default for Tester {
    fn default() -> Self {
        Self::new()
    }
}

impl Tester {
    pub fn new() -> Self {
        let command = Command::new(TESTER_PATH);
        Self { command }
    }

    pub fn pid_file<F: AsRef<Path>>(&mut self, pid_file: F) -> &mut Self {
        self.command.arg(ARG_PID_FILE).arg(pid_file.as_ref());
        self
    }

    pub fn chown_pid_file(&mut self) -> &mut Self {
        self.command.arg(ARG_CHOWN_PID_FILE);
        self
    }

    pub fn working_directory<F: AsRef<Path>>(&mut self, path: F) -> &mut Self {
        self.command.arg(ARG_WORKING_DIRECTORY).arg(path.as_ref());
        self
    }

    pub fn user_string(&mut self, user: &str) -> &mut Self {
        self.command.arg(ARG_USER_STRING).arg(user);
        self
    }

    pub fn user_num(&mut self, user: u32) -> &mut Self {
        self.command.arg(ARG_USER_STRING).arg(user.to_string());
        self
    }

    pub fn group_string(&mut self, group: &str) -> &mut Self {
        self.command.arg(ARG_GROUP_STRING).arg(group);
        self
    }

    pub fn group_num(&mut self, group: u32) -> &mut Self {
        self.command.arg(ARG_GROUP_STRING).arg(group.to_string());
        self
    }

    pub fn umask(&mut self, umask: u32) -> &mut Self {
        self.command.arg(ARG_UMASK).arg(umask.to_string());
        self
    }

    pub fn chroot<F: AsRef<Path>>(&mut self, path: F) -> &mut Self {
        self.command.arg(ARG_CHROOT).arg(path.as_ref());
        self
    }

    pub fn stdout<F: AsRef<Path>>(&mut self, path: F) -> &mut Self {
        self.command.arg(ARG_STDOUT).arg(path.as_ref());
        self
    }

    pub fn stderr<F: AsRef<Path>>(&mut self, path: F) -> &mut Self {
        self.command.arg(ARG_STDERR).arg(path.as_ref());
        self
    }

    pub fn additional_file<F: AsRef<Path>>(&mut self, path: F) -> &mut Self {
        self.command.arg(ARG_ADDITIONAL_FILE).arg(path.as_ref());
        self
    }

    pub fn sleep(&mut self, duration: std::time::Duration) -> &mut Self {
        self.command
            .arg(ARG_SLEEP_MS)
            .arg(duration.as_millis().to_string());
        self
    }

    pub fn run(&mut self) -> Result<EnvData, Error> {
        let mut child = self
            .command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("unable to spawn child");

        let st = std::time::Instant::now();

        let exit_status = loop {
            let now = std::time::Instant::now();
            if now - st > MAX_WAIT_DURATION {
                panic!("wait for result timeout")
            }
            match child.try_wait().expect("unable to wait for result") {
                Some(result) => break result,
                None => std::thread::sleep(std::time::Duration::from_millis(1)),
            }
        };

        if !exit_status.success() {
            let mut stderr = String::new();
            child
                .stderr
                .expect("unable to get stderr")
                .read_to_string(&mut stderr)
                .expect("unable to read tester stderr");
            panic!(
                "invalid tester exit status ({}), stderr: {}",
                exit_status.code().expect("unable to get status code"),
                stderr
            );
        }

        let mut stdout = [0; DATA_LEN];
        child
            .stdout
            .expect("unable to get stdout")
            .read_exact(&mut stdout)
            .expect("unable to read tester stdout");

        unsafe { std::mem::transmute(stdout) }
    }
}

#[derive(Debug)]
pub struct EnvData {
    pub cwd: arraystring::ArrayString<arraystring::typenum::U255>,
    pub pid: u32,
    pub euid: u32,
    pub egid: u32,
}

impl EnvData {
    fn new() -> EnvData {
        Self {
            cwd: arraystring::ArrayString::from_str(
                std::env::current_dir()
                    .expect("unable to get current dir")
                    .to_str()
                    .expect("invalid path"),
            )
            .expect("too long path"),
            pid: std::process::id(),
            euid: unsafe { libc::geteuid() as u32 },
            egid: unsafe { libc::getegid() as u32 },
        }
    }
}

pub fn execute_tester() {
    let mut daemonize = Daemonize::new();
    let mut args = std::env::args().skip(1);

    fn read_value<T: FromStr>(args: &mut dyn Iterator<Item = String>, key: &str) -> T
    where
        <T as FromStr>::Err: std::fmt::Debug,
    {
        let value = args
            .next()
            .unwrap_or_else(|| panic!("missing value for key {}", key));
        value
            .parse()
            .unwrap_or_else(|_| panic!("invalid value for key {}", key))
    }

    let mut additional_files = Vec::new();
    let mut sleep_duration = None;
    let mut human_readable = false;

    while let Some(key) = args.next() {
        daemonize = match key.as_str() {
            ARG_PID_FILE => daemonize.pid_file(read_value::<PathBuf>(&mut args, &key)),
            ARG_CHOWN_PID_FILE => daemonize.chown_pid_file(true),
            ARG_WORKING_DIRECTORY => {
                daemonize.working_directory(read_value::<PathBuf>(&mut args, &key))
            }
            ARG_USER_STRING => daemonize.user(read_value::<String>(&mut args, &key).as_str()),
            ARG_USER_NUM => daemonize.user(read_value::<u32>(&mut args, &key)),
            ARG_GROUP_STRING => daemonize.group(read_value::<String>(&mut args, &key).as_str()),
            ARG_GROUP_NUM => daemonize.group(read_value::<u32>(&mut args, &key)),
            ARG_UMASK => daemonize.umask(read_value::<u32>(&mut args, &key)),
            ARG_CHROOT => daemonize.chroot(read_value::<PathBuf>(&mut args, &key)),
            ARG_STDOUT => {
                let file = std::fs::File::create(read_value::<PathBuf>(&mut args, &key))
                    .expect("unable to open stdout file");
                daemonize.stdout(file)
            }
            ARG_STDERR => {
                let file = std::fs::File::create(read_value::<PathBuf>(&mut args, &key))
                    .expect("unable to open stder file");
                daemonize.stderr(file)
            }
            ARG_ADDITIONAL_FILE => {
                additional_files.push(read_value::<PathBuf>(&mut args, &key));
                daemonize
            }
            ARG_SLEEP_MS => {
                let ms = read_value::<u64>(&mut args, &key);
                sleep_duration = Some(std::time::Duration::from_millis(ms));
                daemonize
            }
            ARG_HUMAN_READABLE => {
                human_readable = true;
                daemonize
            }
            key => {
                panic!("unknown key: {}", key)
            }
        }
    }

    let (mut read_pipe, mut write_pipe) = os_pipe::pipe().expect("unable to open pipe");

    match daemonize.execute() {
        Outcome::Parent(_) => {
            drop(write_pipe);
            let mut data = Vec::new();
            read_pipe
                .read_to_end(&mut data)
                .expect("unable to read pipe");
            if !human_readable && data.len() != DATA_LEN {
                panic!("invalid data len");
            }
            std::io::stdout()
                .write_all(&data)
                .expect("unable to write data")
        }
        Outcome::Child(result) => {
            drop(read_pipe);
            let result = result.map(|_| EnvData::new());

            print!("{}", STDOUT_DATA);
            eprint!("{}", STDERR_DATA);

            for file_path in additional_files {
                if let Ok(mut file) = std::fs::File::create(&file_path) {
                    file.write_all(ADDITIONAL_FILE_DATA.as_bytes()).ok();
                }
            }

            if human_readable {
                write_pipe
                    .write_all(format!("{:?}\n", result).as_bytes())
                    .ok();
            } else {
                let data: [u8; DATA_LEN] = unsafe { std::mem::transmute(result) };
                write_pipe.write_all(&data).ok();
            }

            drop(write_pipe);

            if let Some(duration) = sleep_duration {
                std::thread::sleep(duration)
            }
        }
    }
}

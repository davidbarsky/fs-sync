#![cfg_attr(feature = "clippy", plugin(clippy))]
#![recursion_limit="1024"]

extern crate difference;
#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate log;
extern crate loggerv;
extern crate notify;
extern crate ssh2;
extern crate structopt;
#[macro_use]
extern crate structopt_derive;
extern crate walkdir;

use structopt::StructOpt;
use log::LogLevel;
use std::path::Path;
use cli::Opts;

mod errors {
    // Create the Error, ErrorKind, ResultExt, and Result types
    error_chain!{
        foreign_links {
            Log(::log::SetLoggerError);
            Notify(::notify::Error);
            Ssh(::ssh2::Error);
            Tcp(::std::io::Error);
            Env(::std::env::VarError);
            WalkDir(::walkdir::Error);
        }

        errors {
            EnviromentRead(env_variable: String) {
                description("Failed to read enviroment variable")
                display("Unable to read enviroment variable `{}`", env_variable)
            }

            HostConnection(host: String) {
                description("Failed to connect to host")
                display("Unable to connect to host `{}`", host)
            }

            UserAuthentication(user: String, host: String) {
                description("Failed to authenticate user with host")
                display("Unable to authenticate user `{}` with host `{}`", user, host)
            }

            Mkdir(path: String) {
                description("Failed to authenticate create directory")
                display("Unable to create directory `{}`", path)
            }

            DirectoryExists(path: String) {
                description("Failed to create directory")
                display("Directory `{}` already exists", path)
            }

            IsDirectory(path: String) {
                description("Failed to read file")
                display("Path `{}` is a directory", path)
            }

            LStat(path: String) {
                description("Failed to run lstat")
                display("Unable to run lstat on path `{}`", path)
            }

            InvalidUTF8(path: String) {
                description("Stream did not contain valid UTF-8")
                display("Unable to get a UTF-8 stream for `{}`", path)
            }
        }
    }
}
use errors::*;

pub mod cli {
    #[derive(StructOpt, Debug)]
    #[structopt(name = "fs-sync", about = "An example of fs-sync usage.")]
    pub struct Opts {
        /// The file or directory that fs-watch should observe.
        #[structopt(help = "Path to observe")]
        pub local_path: String,

        /// The ssh host/destination
        #[structopt(help = "Host to sync to")]
        pub host: String,

        /// The directory that fs-watch should write to.
        #[structopt(help = "Path that fs-watch should write to.")]
        pub host_path: String,

        /// The port that Opts::host should connect to.
        #[structopt(short = "-p", long = "port", help = "Port on host to connect to")]
        pub port: Option<i64>,
    }
}

fn main() {
    if let Err(e) = run() {
        error!("{}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args = Opts::from_args();
    let path = Path::new(&args.local_path);
    loggerv::init_with_level(LogLevel::Info)?;

    info!("Starting fs-sync");
    info!("Reading files in {}", args.local_path);
    let files = local::read_files_in_dir(&args.local_path)?;
    info!("{:?}", files.len());

    info!("Connecting to host {:?}", args.host);
    let formatted_host = local::format_host_string(&args.host, args.port);
    let user = local::read_env("USER")?;
    let connection = remote::authenticate_with_agent(&formatted_host, &user)?;

    info!("Attempting to create directory {:?}", args.host_path);
    match connection.mkdir(&args.host_path) {
        Ok(_) => info!("Created directory {:?} successfully", args.host_path),
        Err(_) => info!("Directory {:?} already exists", args.host_path),
    }

    info!("Starting to watch {:?}", path.display());
    if let Err(ref e) = local::watch(path) {
        error!("error: {:?}", e);
        panic!();
    }
    Ok(())
}

pub mod remote {
    use super::errors::*;
    use std::net::TcpStream;
    use std::path::Path;
    use ssh2::{FileStat, Session};

    pub struct Connection {
        pub stream: TcpStream,
        pub session: Session,
    }

    impl Connection {
        pub fn mkdir(&self, dir: &str) -> Result<()> {
            let dir_name = Path::new(&dir);
            let ftp = self.session.sftp()?;
            match self.stat(dir) {
                Err(_) => ftp.mkdir(dir_name, 0o755).chain_err(|| {
                    ErrorKind::Mkdir(String::from(dir_name.to_string_lossy()))
                }),
                _ => {
                    Err(ErrorKind::DirectoryExists(String::from(dir_name.to_string_lossy())).into())
                }
            }
        }

        pub fn stat(&self, dir: &str) -> Result<FileStat> {
            let dir_name = Path::new(&dir);
            let ftp = self.session.sftp()?;
            ftp.stat(dir_name).chain_err(|| {
                ErrorKind::LStat(String::from(dir_name.to_string_lossy()))
            })
        }
    }

    pub fn authenticate_with_agent(host: &str, user: &str) -> Result<Connection> {
        let tcp =
            TcpStream::connect(host).chain_err(|| ErrorKind::HostConnection(host.to_string()))?;
        // Session::new() returns an Option<Session>, so there is little
        // error propogation needed.
        let mut session = Session::new().unwrap();
        session.handshake(&tcp)?;

        session.userauth_agent(user).chain_err(|| {
            ErrorKind::UserAuthentication(user.to_string(), host.to_string())
        })?;

        Ok(Connection {
            stream: tcp,
            session: session,
        })
    }
}

pub mod local {
    use super::errors::*;

    use std::fs::File;
    use std::io::prelude::*;
    use std::path::{Component, Path, PathBuf};
    use std::sync::mpsc::channel;

    use walkdir::{WalkDir};
    use notify::{raw_watcher, RawEvent, RecommendedWatcher, RecursiveMode, Watcher};
    use difference::{Changeset, Difference};

    #[derive(Debug)]
    pub struct MemoryFile {
        pub path: PathBuf,
        pub contents: String,
    }

    pub fn watch(path: &Path) -> Result<()> {
        let (sender, reciever) = channel();

        let mut watcher: RecommendedWatcher = raw_watcher(sender)?;
        watcher.watch(path, RecursiveMode::Recursive)?;

        loop {
            let event = reciever
                .recv()
                .chain_err(|| "Unable to receive file system event.");
            handle_event(event);
        }
    }

    pub fn read_env(env_var: &str) -> Result<String> {
        ::std::env::var(env_var).chain_err(|| ErrorKind::EnviromentRead(env_var.to_string()))
    }

    pub fn diff<'a>(a: &str, b: &str) -> Option<String> {
        let changeset = Changeset::new(a, b, " ").diffs;
        for change in changeset {
            match change {
                Difference::Add(s) => return Some(s),
                _ => (),
            }
        }
        None
    }

    pub fn read_files_in_dir(path: &str) -> Result<Vec<MemoryFile>> {
        let mut files = vec!();
        let walker = WalkDir::new(path).into_iter();
        for entry in walker
            .filter_entry(|e| !any_match(e.path(), ".git"))
            .filter_map(|e| e.ok()) {
            let path = entry.path().clone();
            if path.is_file() {
                let contents = read_file_to_string(path).chain_err(||
                    ErrorKind::InvalidUTF8(path.to_str().unwrap().to_owned()))?;

                let file = MemoryFile {
                    path: path.to_owned(),
                    contents: contents,
                };
                files.push(file);
            }
        }
        Ok(files)
    }

    pub fn read_file_to_string(path: &Path) -> Result<String> {
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        Ok(contents)
    }

    pub fn format_host_string(host: &str, port: Option<i64>) -> String {
        if let Some(i) = port {
            format!("{}:{}", host, i)
        } else {
            format!("{}:22", host)
        }
    }

    fn handle_event(event: Result<RawEvent>) {
        match event {
            Ok(RawEvent {
                path: Some(path),
                op: Ok(op),
                cookie,
            }) => if !any_match(&path, ".git") {
                if !any_match(&path, "target") {
                    info!("Operation: {:?} \n Path: {:?} \n ({:?})", op, path, cookie);
                }
            },
            Ok(event) => error!("broken event: {:?}", event),
            Err(e) => error!("watch error: {:?}", e),
        }
    }

    pub fn any_match(path: &Path, predicate: &'static str) -> bool {
        path.components()
            .any(|c: Component| c == Component::Normal(predicate.as_ref()))
    }
}

#[cfg(test)]
mod local_tests {
    use local::{any_match, diff, format_host_string};
    use std::path::Path;

    #[test]
    fn test_format_host_string_with_port() {
        let original = "random.host".to_owned();
        let formatted_result = format_host_string(&original, Some(42));
        assert_eq!(formatted_result, "random.host:42");
    }

    #[test]
    fn test_format_host_string_without_port() {
        let original = "random.host".to_owned();
        let formatted_result = format_host_string(&original, None);
        assert_eq!(formatted_result, "random.host:22");
    }

    #[test]
    fn test_format_bad_host_string_with_port() {
        let original = "random.hostL=::22".to_owned();
        let formatted_result = format_host_string(&original, None);
        assert_eq!(formatted_result, "random.hostL=::22:22");
    }

    #[test]
    fn diff_diffs() {
        let a = "/Users/dbarsky/Developer/Rust/fs-sync/";
        let b = "fs-sync/";
        assert_eq!(diff(a, b), Some("fs-sync/".to_string()))
    }

    #[test]
    fn git_directory_matches_correctly() {
        let path = Path::new("~/Developer/Rust/fs-sync/.git");
        assert_eq!(any_match(path, ".git"), true)
    }

    #[test]
    fn target_directory_matches_correctly() {
        let path = Path::new("~/Developer/Rust/fs-sync/");
        assert_eq!(any_match(path, "Rust"), true)
    }
}

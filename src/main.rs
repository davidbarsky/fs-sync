#![cfg_attr(feature = "clippy", plugin(clippy))]
#![feature(type_ascription)]
#![feature(try_trait)]
#![feature(plugin)]
#![cfg_attr(test, feature(plugin))]
#![cfg_attr(test, plugin(quickcheck_macros))]


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

#[cfg(test)]
extern crate quickcheck;

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
        }
    }
}
use errors::*;

mod cli {
    #[derive(StructOpt, Debug)]
    #[structopt(name = "fs-sync", about = "An example of fs-sync usage.")]
    pub struct Opts {
        /// The file or directory that fs-watch should observe.
        #[structopt(help = "Path to observe")]
        pub source_path: String,

        /// The ssh destination
        #[structopt(help = "Host to sync to")]
        pub host: String,

        /// The directory that fs-watch should dump to.
        #[structopt(help = "Path that should be sycned to")]
        pub destination_path: String,
    }
}

quick_main!(|| -> Result<()> {
    let args: Opts = Opts::from_args();
    let path = Path::new(args.source_path.as_str());
    loggerv::init_with_level(LogLevel::Info)?;

    let host = "localhost:22";
    info!("Starting fs-sync...");
    info!("Connecting to host {:?}", host);
    remote::test(host)?;

    info!("Starting to watch {:?}...", path.display());
    if let Err(ref e) = listener::watch(path) {
        info!("{:?}", path);
        error!("error: {:?}", e);
        panic!();
    }
    Ok(())
});

mod remote {
    use super::errors::*;
    use std::net::TcpStream;
    use ssh2::Session;
    use std::env;
    use std::path::Path;
    use ssh2::{CheckResult, HostKeyType, KnownHostKeyFormat};
    use ssh2::KnownHostFileKind;

    pub fn connect_with_password(username: &str, password: &str, host: &str) -> Result<Session> {
        let tcp = TcpStream::connect(host).chain_err(|| {
            format!("TcpStream is unable to connect to host {:?}", host)
        })?;
        // Session::new() returns an Option<Session>, so there is little
        // error propogation needed.
        let mut sess = Session::new().unwrap();
        sess.handshake(&tcp)
            .chain_err(|| "Session is unable to connect with existing TcpStream")?;

        sess.userauth_password(username, password).chain_err(|| {
            format!(
                "Unable to authenticate with username ({:?}) and password ({:?})",
                username,
                password
            )
        })?;
        Ok(sess)
    }

    pub fn test(host: &str) -> Result<()> {
        let tcp = TcpStream::connect(host).chain_err(|| {
            format!("TcpStream is unable to connect to host {:?}", host)
        })?;
        // Session::new() returns an Option<Session>, so there is little
        // error propogation needed.
        let mut session = Session::new().unwrap();
        session
            .handshake(&tcp)
            .chain_err(|| "Session is unable to connect with existing TcpStream")?;

        let mut known_hosts = session.known_hosts()?;

        // Initialize the known hosts with a global known hosts file
        let file = Path::new(&env::var("HOME").unwrap()).join(".ssh/known_hosts");
        known_hosts.read_file(&file, KnownHostFileKind::OpenSSH)?;

        // Now check to see if the seesion's host key is anywhere in the known
        // hosts file
        let (key, key_type) = session.host_key().unwrap();
        match known_hosts.check(host, key) {
            CheckResult::Match => info!("{:?}", key), // all good!
            CheckResult::NotFound => error!(
                "session's host key {:?} was not found in the hosts file.",
                key
            ),
            // ok, we'll add it
            CheckResult::Mismatch => panic!("host mismatch, man in the middle attack?!"),
            CheckResult::Failure => panic!("failed to check the known hosts"),
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    extern crate ssh2;
    use ssh2::Session;
    use super::remote;

    #[test]
    fn connect_works() {
        let host = "127.0.0.1:32768";
        let session: Session = remote::connect_with_password("root", "screencast", host).unwrap();
        assert!(session.authenticated())
    }

    fn reverse<T: Clone>(xs: &[T]) -> Vec<T> {
        let mut rev = vec![];
        for x in xs {
            rev.insert(0, x.clone())
        }
        rev
    }

    #[quickcheck]
    fn double_reversal_is_identity(xs: Vec<isize>) -> bool {
        xs == reverse(&reverse(&xs))
    }
}

mod listener {
    use super::errors::*;
    use notify::{raw_watcher, RawEvent, RecommendedWatcher, RecursiveMode, Watcher};
    use std::path::{Component, Path};
    use std::sync::mpsc::channel;

    pub fn watch(path: &Path) -> Result<()> {
        let (sender, reciever) = channel();

        let mut watcher: RecommendedWatcher = raw_watcher(sender)?;
        watcher.watch(path, RecursiveMode::Recursive)?;

        loop {
            let event = reciever
                .recv()
                .chain_err(|| "Unable to recieve file system event.");
            handle_event(event);
        }
    }

    fn handle_event(event: Result<RawEvent>) {
        match event {
            Ok(RawEvent {
                path: Some(path),
                op: Ok(op),
                cookie,
            }) => if !is_git_directory(&path) {
                if !is_target_directory(&path) {
                    info!("Operation: {:?} \n Path: {:?} \n ({:?})", op, path, cookie);
                }
            },
            Ok(event) => error!("broken event: {:?}", event),
            Err(e) => error!("watch error: {:?}", e),
        }
    }

    fn is_git_directory(path: &Path) -> bool {
        path.components()
            .any(|c: Component| c == Component::Normal(".git".as_ref()))
    }

    fn is_target_directory(path: &Path) -> bool {
        path.components()
            .any(|c: Component| c == Component::Normal("target".as_ref()))
    }
}

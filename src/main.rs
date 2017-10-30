#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]
#![feature(type_ascription)]
#![feature(try_trait)]

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
        pub ssh_destination: String,

        /// The directory that fs-watch should dump to.
        #[structopt(help = "Path that should be sycned to")]
        pub destination_path: String,
    }
}

quick_main!(|| -> Result<()> {
    let args: Opts = Opts::from_args();
    let path = Path::new(args.source_path.as_str());
    loggerv::init_with_level(LogLevel::Info)?;
    let host = "127.0.0.1:32769";

    info!("Starting fs-sync...");
    info!("Connecting to host {:?}", host);
    remote::connect("root", "screencast", host)?;

    info!("Starting to watch {:?}...", path.display());
    if let Err(ref e) = fs_watch::watch(path) {
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

    pub fn connect(username: &str, password: &str, host: &str) -> Result<Session> {
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
}

#[cfg(test)]
mod tests {
    extern crate ssh2;
    use ssh2::Session;
    use super::remote::connect;

    #[test]
    fn connect_works() {
        let host = "127.0.0.1:32769";
        let session: Session = connect("root", "screencast", host).unwrap();
        assert!(session.authenticated())
    }
}

mod fs_watch {
    use super::errors::*;
    use notify::{raw_watcher, RawEvent, RecommendedWatcher, RecursiveMode, Watcher};
    use std::path::{Path, Component};
    use std::sync::mpsc::channel;

    pub fn watch(path: &Path) -> Result<()> {
        let (sender, reciever) = channel();

        let mut watcher: RecommendedWatcher = raw_watcher(sender)?;
        watcher.watch(path, RecursiveMode::Recursive)?;

        loop {
            let event = reciever.recv()
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
            Ok(event) => info!("broken event: {:?}", event),
            Err(e) => info!("watch error: {:?}", e),
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

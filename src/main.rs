#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]
#![feature(type_ascription)]

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
use notify::{raw_watcher, Op, RawEvent, RecommendedWatcher, RecursiveMode, Watcher};
use std::sync::mpsc::channel;
use log::LogLevel;
use ssh2::Session;
use std::path::Path;

mod errors {
    // Create the Error, ErrorKind, ResultExt, and Result types
    error_chain!{
        foreign_links {
            Log(::log::SetLoggerError);
            Notify(::notify::Error);
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
    use cli::Opts;
    let args: Opts = Opts::from_args();
    let path = Path::new(args.source_path.as_str());
    println!("{:?}", args);
    println!("{:?}", path);

    loggerv::init_with_level(LogLevel::Info)?;
    info!("Hi! Starting fs-sync...");
    connect()?;

    if let Err(ref e) = watch(path) {
        println!("{:?}", path);
        error!("error: {:?}", e);
        panic!();
    }
    Ok(())
});

fn connect() -> Result<Session> {
    let sess = Session::new().unwrap();
    let f = |s: &Session| {
        let mut agent = s.agent().unwrap();
        agent.connect().unwrap();
        println!("Agent Identities: {:?}", agent.list_identities().unwrap());
    };
    f(&sess);
    Ok(sess)
}

fn watch(path: &Path) -> Result<()> {
    let (tx, rx) = channel();

    let mut watcher: RecommendedWatcher = raw_watcher(tx)?;
    watcher.watch(path, RecursiveMode::Recursive)?;

    loop {
        let event = rx.recv()
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

use std::path::Component;

fn is_git_directory(path: &Path) -> bool {
    path.components()
        .any(|c: Component| c == Component::Normal(".git".as_ref()))
}

fn is_target_directory(path: &Path) -> bool {
    path.components()
        .any(|c: Component| c == Component::Normal("target".as_ref()))
}

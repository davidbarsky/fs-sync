#![cfg_attr(feature = "clippy", plugin(clippy))]
#![feature(conservative_impl_trait)]

#[macro_use]
extern crate failure;
extern crate ignore;
#[macro_use]
extern crate log;
extern crate loggerv;
extern crate notify;
extern crate ssh2;
extern crate structopt;
#[macro_use]
extern crate structopt_derive;

mod local;
mod remote;
mod paths;
mod types;

use structopt::StructOpt;
use std::path::Path;
use cli::Opts;
use log::LogLevel;
use failure::Error;
use remote::Connection;
use local::FileWatcher;

pub mod cli {
    #[derive(StructOpt, Debug)]
    #[structopt(name = "fs-sync", about = "An example of fs-sync usage.")]
    pub struct Opts {
        #[structopt]
        /// Watch this directory.
        pub local_path: String,

        /// Sync to this host.
        #[structopt]
        pub host: String,

        /// Write to this directory on remote.
        #[structopt]
        pub host_path: String,

        /// Connect to this post on host.
        #[structopt(short = "-p", long = "port", default_value = "22")]
        pub port: i64,
    }
}

fn main() {
    if let Err(e) = run() {
        error!("{}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<(), Error> {
    let args = Opts::from_args();
    loggerv::init_with_level(LogLevel::Info)?;

    info!("Starting fs-sync");
    info!("Reading files in {}", args.local_path);
    let local_path = Path::new(&args.local_path);
    let remote_path = Path::new(&args.host_path);
    let file_list = local::visit_dirs(local_path)?
        .iter()
        .map(|e| e.path().to_path_buf())
        .collect();

    let pairings = paths::zip_local_and_remote(file_list, local_path, remote_path)?;

    info!("Connecting to {:?}", args.host);
    let formatted_host = local::format_host_string(&args.host, args.port);
    let user = local::read_env("USER")?;
    let connection = Connection::new(&formatted_host, &user)?;

    info!("Attempting to create directory {:?}", args.host_path);
    match connection.initial_sync(&pairings) {
        Ok(_) => info!("Successfully made an initial sync"),
        Err(e) => return Err(e),
    }

    info!("Starting to watch {:?}", local_path.display());
    let file_watcher = FileWatcher::new(connection, local_path, remote_path);
    if let Err(ref e) = file_watcher.watch(local_path) {
        error!("{}", e);
    }
    Ok(())
}

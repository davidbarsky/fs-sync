#![cfg_attr(feature = "clippy", plugin(clippy))]
#![feature(conservative_impl_trait)]
#![recursion_limit = "1024"]

extern crate difference;
#[macro_use]
extern crate error_chain;
extern crate ignore;
#[macro_use]
extern crate im as immutable;
#[macro_use]
extern crate log;
extern crate loggerv;
extern crate notify;
extern crate ssh2;
extern crate structopt;
#[macro_use]
extern crate structopt_derive;

use structopt::StructOpt;
use std::path::{Path, PathBuf};
use cli::Opts;
use log::LogLevel;
use immutable::List;

mod local;
mod remote;
mod errors;

use errors::*;

pub mod cli {
    #[derive(StructOpt, Debug)]
    #[structopt(name = "fs-sync", about = "fs-sync syncs .")]
    pub struct Opts {
        /// Watch this directory.
        #[structopt]
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

fn run() -> Result<()> {
    let args = Opts::from_args();
    loggerv::init_with_level(LogLevel::Debug)?;

    info!("Starting fs-sync");
    info!("Reading files in {}", args.local_path);

    let local_path = Path::new(&args.local_path);
    let path_list: List<PathBuf> = local::visit_dirs(local_path)?
        .iter()
        .map(|e| e.path().to_path_buf())
        .collect();

    let pairings = paths::zip_local_and_remote(
        path_list,
        Path::new(&args.local_path).to_path_buf(),
        Path::new(&args.host_path).to_path_buf(),
    );
    debug!("Pairings: {:?}", pairings);

    info!("Connecting to host {:?}", args.host);
    let formatted_host = local::format_host_string(&args.host, args.port);
    let user = local::read_env("USER")?;
    let connection = remote::authenticate_with_agent(&formatted_host, &user)?;

    info!("Attempting to create directory {:?}", args.host_path);
    match connection.mkdir(&args.host_path) {
        Ok(_) => info!("Created directory {:?} successfully", args.host_path),
        Err(_) => info!("Directory {:?} already exists", args.host_path),
    }

    info!("Starting to watch {:?}", local_path.display());
    if let Err(ref e) = local::watch(local_path) {
        error!("error: {:?}", e);
        panic!();
    }
    Ok(())
}


pub mod paths {
    use errors::*;
    use immutable::{List, Map};
    use std::path::{Path, PathBuf};

    pub fn generate_remote_path(local_file: PathBuf, remote_directory: PathBuf) -> PathBuf {
        let mut path = PathBuf::new();

        path.push(remote_directory);
        path.push(local_file);

        path
    }

    pub fn strip_prefix(observed_path: &Path, changed_file: &Path) -> Result<PathBuf> {
        if !observed_path.is_dir() {
            bail!(format!(
                "Observed path {:?} is not a directory",
                observed_path
            ));
        }
        if !changed_file.is_file() {
            bail!(format!("path {:?} is not a file", changed_file));
        }

        let relative = changed_file.strip_prefix(observed_path)?;
        Ok(relative.to_path_buf())
    }


    pub fn zip_local_and_remote(
        local_files: List<PathBuf>,
        local_path: PathBuf,
        remote_path: PathBuf,
    ) -> Result<Map<PathBuf, PathBuf>> {
        debug!("Remote: {:?}", remote_path);
        let mut map = map!{};

        for p in local_files {
            let stripped_file = strip_prefix(&local_path, &p)?;
            let remote_path = generate_remote_path(stripped_file, remote_path.clone());
            debug!("Remote Path: {:?}", remote_path);
            map = map.insert(p.to_path_buf(), remote_path);
        }

        Ok(map)
    }
}

#[cfg(test)]
mod path_tests {
    use immutable::{List, Map};

    use paths::{generate_remote_path, strip_prefix, zip_local_and_remote};
    use std::path::{Path, PathBuf};

    #[test]
    fn generate_remote_path_works() {
        let file = Path::new("src/main.rs").to_path_buf();
        let directory = Path::new("/local/home/dbarsky/Desktop/test").to_path_buf();

        assert_eq!(
            generate_remote_path(file, directory),
            Path::new("/local/home/dbarsky/Desktop/test/src/main.rs").to_path_buf()
        )
    }

    #[test]
    fn path_is_made_relative() {
        let watched_path = Path::new("/Users/dbarsky/Developer/Rust/fs-sync/");
        let file = Path::new("/Users/dbarsky/Developer/Rust/fs-sync/src/main.rs");
        assert_eq!(
            strip_prefix(watched_path, file).unwrap(),
            Path::new("src/main.rs").to_path_buf()
        )
    }

    #[test]
    fn zip_local_and_remote_works() {
        let a = Path::new("cargo.toml").to_path_buf();
        let b = Path::new("src/main.rs").to_path_buf();
        let list = list![a, b];
        let map_actual = zip_local_and_remote(
            list,
            Path::new("/local/home/dbarsky/Desktop/test/").to_path_buf(),
        );

        let map_comp: Map<PathBuf, PathBuf> = map!{
            Path::new("cargo.toml").to_path_buf() =>
                Path::new("/local/home/dbarsky/Desktop/test/cargo.toml").to_path_buf(),
            Path::new("src/main.rs").to_path_buf() =>
                Path::new("/local/home/dbarsky/Desktop/test/src/main.rs").to_path_buf()
        };

        assert_eq!(map_actual, map_comp)
    }
}

#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]

extern crate glob;
#[macro_use]
extern crate log;
extern crate loggerv;
extern crate notify;

use notify::{raw_watcher, RawEvent, RecommendedWatcher, RecursiveMode, Watcher};
use std::sync::mpsc::channel;
use log::LogLevel;

fn main() {
    loggerv::init_with_level(LogLevel::Info).unwrap();
    // trace!("a trace example");
    // debug!("deboogging");
    // info!("such information");
    // warn!("o_O");
    // error!("boom");

    info!("Hi! Starting fs-sync...");
    if let Err(e) = watch() {
        println!("error: {:?}", e)
    }
}

fn watch() -> notify::Result<()> {
    let (tx, rx) = channel();

    let mut watcher: RecommendedWatcher = try!(raw_watcher(tx));
    try!(watcher.watch(
        "/Users/David/Developer/Rust/fs-sync",
        RecursiveMode::Recursive
    ));

    loop {
        match rx.recv() {
            Ok(RawEvent {
                path: Some(path),
                op: Ok(op),
                cookie,
            }) => {
                println!("{:?}", path.components());
                info!("Operation: {:?} \n Path: {:?} \n ({:?})", op, path, cookie);
            }
            Ok(event) => info!("broken event: {:?}", event),
            Err(e) => info!("watch error: {:?}", e),
        }
    }
}

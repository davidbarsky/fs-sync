#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]

#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate log;
extern crate loggerv;
extern crate notify;

use notify::{raw_watcher, RawEvent, RecommendedWatcher, RecursiveMode, Watcher};
use std::sync::mpsc::channel;
use std::fs::File;
use log::LogLevel;

mod errors {
    // Create the Error, ErrorKind, ResultExt, and Result types
    error_chain!{}
}

use errors::*;

quick_main!(|| -> Result<()> {
    try!(loggerv::init_with_level(LogLevel::Info));
    info!("Hi! Starting fs-sync...");

    if let Err(ref e) = run() {
        error!("error: {:?}", e);

        if let Some(backtrace) = e.backtrace() {
           println!("backtrace: {:?}", backtrace);
        }
    }
    Ok(())
});

fn run() -> Result<()> {
    use std::fs::File;

    // This operation will fail
    File::open("contacts")
        .chain_err(|| "unable to open contacts file")?;

    Ok(())
}

error_chain! {
    foreign_links {
        Log(::log::SetLoggerError);
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
        }) => {
            println!("{:?}", path.components());
            info!("Operation: {:?} \n Path: {:?} \n ({:?})", op, path, cookie);
        }
        Ok(event) => info!("broken event: {:?}", event),
        Err(e) => info!("watch error: {:?}", e),
    }
}

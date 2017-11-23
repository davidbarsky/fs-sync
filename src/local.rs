use immutable::{List, Map};
use failure::Error;
use ignore::{DirEntry, Walk};
use notify::{raw_watcher, RawEvent, RecommendedWatcher, RecursiveMode, Watcher};

use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;

use remote::Connection;

pub fn read_env(env_var: &str) -> Result<String, Error> {
    match ::std::env::var(env_var) {
        Ok(env) => Ok(env),
        Err(_) => Err(format_err!("Could not read env var: {}", env_var)),
    }
}

pub fn visit_dirs(dir: &Path) -> Result<List<DirEntry>, Error> {
    let list: List<_> = Walk::new(dir)
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .collect();

    Ok(list)
}

pub fn read_file_to_string(path: &Path) -> Result<String, Error> {
    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
}

pub fn format_host_string(host: &str, port: i64) -> String {
    format!("{}:{}", host, port)
}

pub struct FileWatcher {
    pub connection: Connection,
}

impl FileWatcher {
    pub fn watch(&self, path: &Path, file_map: &Map<PathBuf, PathBuf>) -> Result<(), Error> {
        let (sender, reciever) = channel();

        let mut watcher: RecommendedWatcher = raw_watcher(sender)?;
        watcher.watch(path, RecursiveMode::Recursive)?;

        loop {
            let event = reciever.recv()?;
            self.handle_event(event, file_map);
        }
    }

    fn handle_event(&self, event: RawEvent, file_map: &Map<PathBuf, PathBuf>) {
        let path = event.path.unwrap();
        if file_map.contains_key(&path.to_path_buf()) {
            debug!("Syncing {:?}", path);
            let remote = &file_map.get(&path).unwrap().to_path_buf();
            self.connection.sync(&path, remote).unwrap();
        }
    }
}

#[cfg(test)]
mod local_tests {
    use super::format_host_string;

    #[test]
    fn format_host_string_correctly() {
        let original = "random.host".to_owned();
        let formatted_result = format_host_string(&original, 22);
        assert_eq!(formatted_result, "random.host:22")
    }
}

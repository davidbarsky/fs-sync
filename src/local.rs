use errors::*;

use immutable::{List, Map};
use ignore::{DirEntry, Walk};
use notify::{raw_watcher, RawEvent, RecommendedWatcher, RecursiveMode, Watcher};

use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;

use remote::Connection;

pub fn read_env(env_var: &str) -> Result<String> {
    ::std::env::var(env_var).chain_err(|| ErrorKind::EnviromentRead(env_var.to_string()))
}

pub fn visit_dirs(dir: &Path) -> Result<List<DirEntry>> {
    let list: List<_> = Walk::new(dir)
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .collect();

    Ok(list)
}

pub fn read_file_to_string(path: &Path) -> Result<String> {
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
    pub fn watch(&self, path: &Path, file_map: Map<PathBuf, PathBuf>) -> Result<()> {
        let (sender, reciever) = channel();

        let mut watcher: RecommendedWatcher = raw_watcher(sender)?;
        watcher.watch(path, RecursiveMode::Recursive)?;

        loop {
            let event = reciever
                .recv()
                .chain_err(|| "Unable to receive file system event.");
            self.handle_event(event, file_map.clone());
        }
    }

    fn handle_event(&self, event: Result<RawEvent>, file_map: Map<PathBuf, PathBuf>) {
        match event {
            Ok(RawEvent {
                path: Some(path),
                op: Ok(op),
                cookie,
            }) => if file_map.contains_key(&path.to_path_buf()) {
                info!("Syncing {:?}", path);
                let remote = &file_map.get(&path).unwrap().to_path_buf();
                self.connection.sync(&path, remote).unwrap();
            },
            Ok(event) => error!("broken event: {:?}", event),
            Err(e) => error!("watch error: {:?}", e),
        }
    }
}

#[cfg(test)]
mod local_tests {
    use super::format_host_string;
    use std::path::{Path, PathBuf};
    use immutable::Map;

    #[test]
    fn format_host_string_correctly() {
        let original = "random.host".to_owned();
        let formatted_result = format_host_string(&original, 22);
        assert_eq!(formatted_result, "random.host:22")
    }
}

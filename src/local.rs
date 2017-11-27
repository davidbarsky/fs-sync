use failure::Error;
use ignore::{DirEntry, Walk};
use notify::{watcher, DebouncedEvent, RecursiveMode, Watcher};

use types::*;

use std::fs::File;
use std::io::prelude::*;
use std::path::{Component, Path};
use std::sync::mpsc::channel;
use std::time::Duration;

use paths::make_remote_path;
use remote::Connection;

pub fn read_env(env_var: &str) -> Result<String, Error> {
    match ::std::env::var(env_var) {
        Ok(env) => Ok(env),
        Err(_) => Err(format_err!("Could not read env var: {}", env_var)),
    }
}

pub fn visit_dirs(dir: &Path) -> Result<Vec<DirEntry>, Error> {
    let list: Vec<_> = Walk::new(dir)
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
    pub local_directory: LocalPathBuf,
    pub remote_directory: RemotePathBuf,
}

impl FileWatcher {
    pub fn new(connection: Connection, local_directory: &Path, remote_directory: &Path) -> Self {
        FileWatcher {
            connection: connection,
            local_directory: local_directory.to_path_buf(),
            remote_directory: remote_directory.to_path_buf(),
        }
    }

    pub fn watch(&self, path: &Path) -> Result<(), Error> {
        let (sender, receiver) = channel();

        let mut watcher = watcher(sender, Duration::from_millis(10))?;
        watcher.watch(path, RecursiveMode::Recursive)?;

        loop {
            let event = receiver.recv()?;
            self.handle_event(event)?;
        }
    }

    fn handle_event(&self, event: DebouncedEvent) -> Result<(), Error> {
        match event {
            DebouncedEvent::Create(ref path) if !self.any_match(&path, ".git") => {
                self.write(path.to_owned())
            }
            DebouncedEvent::Create(path) => Ok(()),
            DebouncedEvent::Write(ref path) if !self.any_match(&path, ".git") => {
                self.write(path.to_owned())
            }
            DebouncedEvent::Write(_) => Ok(()),
            DebouncedEvent::Remove(ref path) if !self.any_match(&path, ".git") => {
                self.remove(path.to_owned())
            }
            DebouncedEvent::Remove(_) => Ok(()),
            DebouncedEvent::Rename(old, new) => self.rename(old, new),
            DebouncedEvent::Rescan => Err(format_err!(
                "A serious error occured while watching this directory",
            )),
            DebouncedEvent::Error(e, path) => {
                Err(format_err!("At path {:?}, an error {:?}", e, path))
            }
            DebouncedEvent::NoticeRemove(_) |
            DebouncedEvent::NoticeWrite(_) |
            DebouncedEvent::Chmod(_) => Ok(()),
        }
    }

    fn write(&self, path: LocalPathBuf) -> Result<(), Error> {
        let remote_path = make_remote_path(&path, &self.local_directory, &self.remote_directory)?;
        self.connection.sync(&path, &remote_path)
    }

    fn remove(&self, path: LocalPathBuf) -> Result<(), Error> {
        let remote_path = make_remote_path(&path, &self.local_directory, &self.remote_directory)?;
        self.connection.remove(&remote_path)
    }

    fn rename(&self, old: LocalPathBuf, new: RemotePathBuf) -> Result<(), Error> {
        let old = make_remote_path(&old, &self.local_directory, &self.remote_directory)?;
        let new = make_remote_path(&new, &self.local_directory, &self.remote_directory)?;
        self.connection.rename(&old, &new)
    }

    fn full_sync(&self, file_map: &PathMap) -> Result<(), Error> {
        self.connection.initial_sync(file_map)
    }

    pub fn any_match(&self, path: &LocalPath, predicate: &'static str) -> bool {
        path.components()
            .any(|c: Component| c == Component::Normal(predicate.as_ref()))
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

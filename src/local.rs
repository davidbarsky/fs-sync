use failure::Error;
use ignore::{DirEntry, Walk};
use ignore::gitignore::{Gitignore, Glob};
use ignore::Match;
use notify::{watcher, DebouncedEvent, RecursiveMode, Watcher};

use types::*;

use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
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
    let mut files = vec![];
    for path in Walk::new(dir) {
        let path = path?;
        if path.path().is_file() {
            files.push(path);
        }
    }
    Ok(files)
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

#[derive(Debug)]
pub struct FileBlockList {
    blocklist: Gitignore,
}

impl FileBlockList {
    pub fn new(gitignore_root_path: &LocalPath) -> Result<Self, Error> {
        let ignore_file: LocalPathBuf = [gitignore_root_path, Path::new(".gitignore")]
            .iter()
            .collect();

        let (gitignore, err) = Gitignore::new(ignore_file);
        if let Some(err) = err {
            bail!("{}", err)
        }
        Ok(FileBlockList {
            blocklist: gitignore,
        })
    }

    pub fn any_match(&self, path: &Path) -> Match<&Glob> {
        self.blocklist.matched_path_or_any_parents(path, true)
    }

    pub fn len(&self) -> usize {
        self.blocklist.len()
    }

    pub fn is_empty(&self) -> bool {
        self.blocklist.is_empty()
    }
}

#[cfg(test)]
mod fileblocklist_tests {
    use std::path::Path;
    use super::FileBlockList;

    #[test]
    fn blocklist_accepts_files() {
        let blocklist = FileBlockList::new(Path::new("/Users/dbarsky/Developer/Rust/fs-sync/"))
            .expect("Failed to open .gitignore file");

        let arb_path = Path::new("/Users/dbarsky/Developer/Rust/fs-sync/Cargo.toml");
        let res = blocklist.any_match(arb_path);
        assert!(res.is_none())
    }

    #[test]
    fn blocklist_ignores_files() {
        let path = Path::new("./test-data/");
        let blocklist = FileBlockList::new(path).expect("Failed to open .gitignore file");

        let arb_path = Path::new("a/b/c/fs-sync.iml");
        let res = blocklist.any_match(arb_path);
        assert!(res.is_ignore());
    }

    #[test]
    fn assert_blocklist_has_contents() {
        let path = Path::new("./test-data");
        let blocklist = FileBlockList::new(path).expect("Failed to open .gitignore file");
        assert_eq!(blocklist.len(), 5);
    }
}

pub struct FileWatcher {
    pub connection: Connection,
    pub local_directory: LocalPathBuf,
    pub remote_directory: RemotePathBuf,
    blocklist: FileBlockList,
}

impl FileWatcher {
    pub fn new(
        connection: Connection,
        local_directory: &Path,
        remote_directory: &Path,
    ) -> Result<Self, Error> {
        let blocklist = FileBlockList::new(local_directory)?;
        println!("{:?}", blocklist);
        Ok(FileWatcher {
            connection: connection,
            local_directory: local_directory.to_path_buf(),
            remote_directory: remote_directory.to_path_buf(),
            blocklist: blocklist,
        })
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
            DebouncedEvent::Create(ref path) => match self.blocklist.any_match(&path) {
                Match::Whitelist(_) | Match::None => self.write(path.to_owned()),
                Match::Ignore(_) => Ok(()),
            },
            DebouncedEvent::Write(ref path) => match self.blocklist.any_match(&path) {
                Match::Whitelist(_) | Match::None => self.write(path.to_owned()),
                Match::Ignore(_) => Ok(()),
            },
            DebouncedEvent::Remove(ref path) => match self.blocklist.any_match(&path) {
                Match::Whitelist(_) | Match::None => self.remove(path.to_owned()),
                Match::Ignore(_) => Ok(()),
            },
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

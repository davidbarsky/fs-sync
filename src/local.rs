use errors::*;

use immutable::*;

use std::fs::File;
use std::io::prelude::*;
use std::path::{Component, Path};
use std::sync::mpsc::channel;
use ignore::{DirEntry, Walk};

use notify::{raw_watcher, RawEvent, RecommendedWatcher, RecursiveMode, Watcher};
use difference::{Changeset, Difference};

pub fn watch(path: &Path) -> Result<()> {
    let (sender, reciever) = channel();

    let mut watcher: RecommendedWatcher = raw_watcher(sender)?;
    watcher.watch(path, RecursiveMode::Recursive)?;

    loop {
        let event = reciever
            .recv()
            .chain_err(|| "Unable to receive file system event.");
        handle_event(event);
    }
}

pub fn read_env(env_var: &str) -> Result<String> {
    ::std::env::var(env_var).chain_err(|| ErrorKind::EnviromentRead(env_var.to_string()))
}

pub fn diff(a: &str, b: &str) -> Option<String> {
    let changeset = Changeset::new(a, b, " ").diffs;
    for change in changeset {
        if let Difference::Add(s) = change {
            return Some(s);
        }
    }
    None
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

fn handle_event(event: Result<RawEvent>) {
    match event {
        Ok(RawEvent {
            path: Some(path),
            op: Ok(op),
            cookie,
        }) => if !any_match(&path, ".git") {
            if !any_match(&path, "target") {
                info!("Operation: {:?} \n Path: {:?} \n ({:?})", op, path, cookie);
            }
        },
        Ok(event) => error!("broken event: {:?}", event),
        Err(e) => error!("watch error: {:?}", e),
    }
}

pub fn is_metadata_directory(path: &Path) -> bool {
    let path = if any_match(path, ".git/") {
        return true;
    } else {
        path
    };

    any_match(path, "target/")
}

pub fn any_match(path: &Path, predicate: &'static str) -> bool {
    path.components()
        .any(|c: Component| c == Component::Normal(predicate.as_ref()))
}

#[cfg(test)]
mod local_tests {
    use super::{any_match, diff, format_host_string};
    use std::path::{Path, PathBuf};
    use immutable::Map;

    #[test]
    fn format_host_string_correctly() {
        let original = "random.host".to_owned();
        let formatted_result = format_host_string(&original, 22);
        assert_eq!(formatted_result, "random.host:22")
    }

    #[test]
    fn diff_diffs() {
        let a = "/Users/dbarsky/Developer/Rust/fs-sync/";
        let b = "fs-sync/";
        assert_eq!(diff(a, b), Some("fs-sync/".to_string()))
    }

    #[test]
    fn git_directory_matches() {
        let path = Path::new("~/Developer/Rust/fs-sync/.git");
        assert_eq!(any_match(path, ".git"), true)
    }

    #[test]
    fn target_directory_matches() {
        let path = Path::new("~/Developer/Rust/fs-sync/");
        assert_eq!(any_match(path, "Rust"), true)
    }
}

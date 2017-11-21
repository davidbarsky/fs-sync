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
        let a = Path::new("/Users/dbarsky/Developer/Rust/fs-sync/cargo.toml").to_path_buf();
        let b = Path::new("/Users/dbarsky/Developer/Rust/fs-sync/src/main.rs").to_path_buf();
        let list = list![a, b];

        let watched_path = Path::new("/Users/dbarsky/Developer/Rust/fs-sync/").to_path_buf();
        let map_actual = zip_local_and_remote(
            list,
            watched_path,
            Path::new("/local/home/dbarsky/Desktop/test/").to_path_buf(),
        ).unwrap();

        let map_comp: Map<PathBuf, PathBuf> = map!{
            Path::new("/Users/dbarsky/Developer/Rust/fs-sync/cargo.toml").to_path_buf() =>
                Path::new("/local/home/dbarsky/Desktop/test/cargo.toml").to_path_buf(),
            Path::new("/Users/dbarsky/Developer/Rust/fs-sync/src/main.rs").to_path_buf() =>
                Path::new("/local/home/dbarsky/Desktop/test/src/main.rs").to_path_buf()
        };

        assert_eq!(map_actual, map_comp)
    }
}

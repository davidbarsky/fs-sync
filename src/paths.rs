use failure::Error;
use std::collections::HashMap;
use std::path::PathBuf;

use types::*;

pub fn zip_local_and_remote(
    files: Vec<LocalPathBuf>,
    directory: &LocalPath,
    remote: &RemotePath,
) -> Result<HashMap<PathBuf, PathBuf>, Error> {
    let mut map = HashMap::new();

    for f in files {
        let stripped_file = strip_prefix(&f, directory)?;
        let remote_path = generate_remote_path(stripped_file, remote);
        map.insert(f.to_path_buf(), remote_path);
    }
    Ok(map)
}

pub fn make_remote_path(
    file: &LocalPathBuf,
    directory: &LocalPathBuf,
    remote: &RemotePathBuf,
) -> Result<PathBuf, Error> {
    let relative_file = strip_prefix(&file, &directory)?;
    Ok(generate_remote_path(relative_file.to_owned(), &remote))
}

fn generate_remote_path(file: PathBuf, remote_directory: &RemotePath) -> RemotePathBuf {
    let mut path = PathBuf::new();

    path.push(remote_directory);
    path.push(file);

    path
}

fn strip_prefix(file: &LocalPath, directory: &LocalPath) -> Result<PathBuf, Error> {
    let path = file.strip_prefix(directory)?;
    Ok(path.to_path_buf())
}

#[cfg(test)]
mod path_tests {
    use paths::{generate_remote_path, make_remote_path, strip_prefix, zip_local_and_remote};
    use std::collections::HashMap;
    use std::path::Path;
    use types::*;

    #[test]
    fn generate_remote_path_works() {
        let file = Path::new("src/main.rs").to_path_buf();
        let directory = Path::new("/x/y/z").to_path_buf();

        assert_eq!(
            generate_remote_path(file, &directory),
            Path::new("/x/y/z/src/main.rs").to_path_buf()
        )
    }

    #[test]
    fn path_is_made_relative() {
        let file = Path::new("/a/b/c/fs-sync/src/main.rs");
        let directory = Path::new("/a/b/c/fs-sync/");
        assert_eq!(
            strip_prefix(file, directory).unwrap(),
            Path::new("src/main.rs").to_path_buf()
        )
    }

    #[test]
    fn make_remote_path_works() {
        let file: LocalPathBuf = Path::new("/a/b/c/fs-sync/src/main.rs").to_path_buf();
        let directory: LocalPathBuf = Path::new("/a/b/c/fs-sync").to_path_buf();
        let remote: RemotePathBuf = Path::new("/x/y/z").to_path_buf();

        let remote_path = make_remote_path(&file, &directory, &remote).unwrap();
        assert_eq!(remote_path, Path::new("/x/y/z/src/main.rs"))
    }

    #[test]
    fn zip_local_and_remote_works() {
        let a = Path::new("/a/b/c/fs-sync/cargo.toml").to_path_buf();
        let b = Path::new("/a/b/c/fs-sync/src/main.rs").to_path_buf();
        let list = vec![a, b];

        let watched_path = Path::new("/a/b/c/fs-sync/").to_path_buf();
        let map_actual =
            zip_local_and_remote(list, &watched_path, &Path::new("/x/y/z/").to_path_buf()).unwrap();

        let mut map_comp: HashMap<LocalPathBuf, RemotePathBuf> = HashMap::new();

        map_comp.insert(
            Path::new("/a/b/c/fs-sync/cargo.toml").to_path_buf(),
            Path::new("/x/y/z/cargo.toml").to_path_buf(),
        );
        map_comp.insert(
            Path::new("/a/b/c/fs-sync/src/main.rs").to_path_buf(),
            Path::new("/x/y/z/src/main.rs").to_path_buf(),
        );

        assert_eq!(map_actual, map_comp)
    }
}

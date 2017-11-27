use std::path::{Path, PathBuf};
use std::collections::HashMap;

pub type RemotePathBuf = PathBuf;
pub type RemotePath = Path;
pub type LocalPathBuf = PathBuf;
pub type LocalPath = Path;
pub type PathMap = HashMap<LocalPathBuf, RemotePathBuf>;

// #[derive(Eq, PartialEq, Debug, NewType, Hash)]
// pub struct LocalPathBuf(pub PathBuf);
// #[derive(Eq, PartialEq, Debug, NewType, Hash)]
// pub struct RemotePathBuf(pub PathBuf);

// #[derive(Eq, PartialEq, Debug, Hash)]
// pub struct RemotePathBuf2 {
//     pub path: PathBuf,
// }

// impl Ord for LocalPathBuf {
//     fn cmp(&self, other: &LocalPathBuf) -> Ordering {
//         self.0.cmp(&other.0)
//     }
// }

// impl PartialOrd for LocalPathBuf {
//     fn partial_cmp(&self, other: &LocalPathBuf) -> Option<Ordering> {
//         Some(self.cmp(other))
//     }
// }

// impl<'a> From<&'a Path> for LocalPathBuf {
//     fn from(path: &Path) -> Self {
//         LocalPathBuf(path.to_path_buf())
//     }
// }

// impl Ord for RemotePathBuf {
//     fn cmp(&self, other: &RemotePathBuf) -> Ordering {
//         self.0.cmp(&other.0)
//     }
// }

// impl PartialOrd for RemotePathBuf {
//     fn partial_cmp(&self, other: &RemotePathBuf) -> Option<Ordering> {
//         Some(self.cmp(other))
//     }
// }

// impl<'a> From<&'a Path> for RemotePathBuf {
//     fn from(path: &Path) -> Self {
//         RemotePathBuf(path.to_path_buf())
//     }
// }

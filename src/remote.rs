use std::io::prelude::*;
use std::sync::Arc;
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use ssh2::{FileStat, Session};
use std::fs;
use failure::Error;
use immutable::Map;

use local::read_file_to_string;

pub struct Connection {
    pub stream: TcpStream,
    pub session: Arc<Session>,
}

impl Connection {
    pub fn mkdir(&self, dir: &Path) -> Result<(), Error> {
        let ftp = self.session.sftp()?;
        if self.stat(dir).is_err() {
            ftp.mkdir(dir, 0o755)?;
        }
        Ok(())
    }

    pub fn stat(&self, dir: &Path) -> Result<FileStat, Error> {
        let ftp = self.session.sftp()?;
        match ftp.stat(dir) {
            Ok(stat) => Ok(stat),
            Err(e) => Err(format_err!("Encountered an error running stat: {}", e)),
        }
    }

    pub fn initial_sync(&self, file_map: Map<PathBuf, PathBuf>) -> Result<(), Error> {
        for (key, value) in file_map {
            let remote_path_parent = match value.as_path().parent() {
                None => return Err(format_err!("Path {:?} does not have a parent", value)),
                Some(rpp) => rpp,
            };

            self.mkdir(remote_path_parent)?;
            self.sync(&key, &value)?;
        }

        Ok(())
    }

    pub fn sync(&self, local_file: &Path, remote_destination: &Path) -> Result<(), Error> {
        let contents = read_file_to_string(local_file)?;
        let byte_contents = contents.as_bytes();
        let size: u64 = fs::metadata(&local_file)?.len();

        info!("Writing to {:?}", remote_destination);
        let mut remote_file = self.session
            .scp_send(remote_destination, 0o755, size, None)?;

        remote_file.write_all(byte_contents)?;
        Ok(())
    }
}

pub fn authenticate_with_agent(host: &str, user: &str) -> Result<Connection, Error> {
    let tcp = TcpStream::connect(host)?;
    // Session::new() returns an Option<Session>, so there is little
    // error propogation needed.
    let mut session = Session::new().unwrap();
    session.handshake(&tcp)?;

    session.userauth_agent(user)?;

    Ok(Connection {
        stream: tcp,
        session: Arc::new(session),
    })
}

use std::io::prelude::*;
use std::sync::Arc;
use std::net::TcpStream;
use ssh2::{FileStat, Session};
use std::fs;
use failure::Error;

use types::*;

use local::read_file_to_string;

pub struct Connection {
    pub stream: TcpStream,
    pub session: Arc<Session>,
}

impl Connection {
    pub fn new(host: &str, user: &str) -> Result<Self, Error> {
        let tcp = TcpStream::connect(host)?;

        let mut session = match Session::new() {
            None => return Err(format_err!("Session failed to initialized")),
            Some(s) => s,
        };

        session.handshake(&tcp)?;
        session.userauth_agent(user)?;

        Ok(Connection {
            stream: tcp,
            session: Arc::new(session),
        })
    }

    pub fn mkdir(&self, dir: &RemotePath) -> Result<(), Error> {
        let ftp = self.session.sftp()?;
        if self.stat(dir).is_err() {
            ftp.mkdir(dir, 0o755)?;
        }
        Ok(())
    }

    pub fn remove(&self, path: &RemotePath) -> Result<(), Error> {
        let ftp = self.session.sftp()?;
        match ftp.rmdir(path) {
            Ok(()) => Ok(()),
            Err(e) => Err(format_err!("{}", e)),
        }
    }

    pub fn rename(&self, old: &RemotePath, new: &RemotePath) -> Result<(), Error> {
        let ftp = self.session.sftp()?;
        match ftp.rename(old, new, None) {
            Ok(()) => Ok(()),
            Err(e) => Err(format_err!("{}", e)),
        }
    }

    pub fn stat(&self, dir: &RemotePath) -> Result<FileStat, Error> {
        let ftp = self.session.sftp()?;
        match ftp.stat(dir) {
            Ok(stat) => Ok(stat),
            Err(e) => Err(format_err!("Encountered an error running stat: {}", e)),
        }
    }

    pub fn initial_sync(&self, file_map: &PathMap) -> Result<(), Error> {
        for (key, value) in file_map {
            let remote_path_parent = match value.as_path().parent() {
                None => return Err(format_err!("Path {:?} does not have a parent", value)),
                Some(rpp) => rpp,
            };

            self.mkdir(remote_path_parent)?;
            self.sync(key, value)?;
        }

        Ok(())
    }

    pub fn sync(
        &self,
        local_file: &LocalPath,
        remote_destination: &RemotePath,
    ) -> Result<(), Error> {
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

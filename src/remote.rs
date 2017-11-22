use std::io::prelude::*;
use std::sync::Arc;
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use ssh2::{FileStat, Session};
use std::fs;
use errors::*;
use immutable::Map;

use local::read_file_to_string;

pub struct Connection {
    pub stream: TcpStream,
    pub session: Arc<Session>,
}

impl Connection {
    pub fn mkdir(&self, dir: &Path) -> Result<()> {
        let ftp = self.session.sftp()?;
        match self.stat(dir) {
            Err(_) => ftp.mkdir(dir, 0o755)
                .chain_err(|| ErrorKind::Mkdir(String::from(dir.to_string_lossy()))),
            _ => Err(ErrorKind::DirectoryExists(String::from(dir.to_string_lossy())).into()),
        }
    }
    pub fn stat(&self, dir: &Path) -> Result<FileStat> {
        let ftp = self.session.sftp()?;
        ftp.stat(dir)
            .chain_err(|| ErrorKind::LStat(String::from(dir.to_string_lossy())))
    }

    pub fn initial_sync(&self, file_map: Map<PathBuf, PathBuf>) -> Result<()> {
        for (key, value) in file_map {
            let remote_path_parent = match value.as_path().parent() {
                None => bail!("Remote path {:?} does not have a parent", value),
                Some(rpp) => rpp,
            };

            match self.mkdir(&remote_path_parent) {
                Err(_) => (),
                Ok(()) => (),
            };
            self.sync(&key, &value)?;
        }

        Ok(())
    }

    pub fn sync(&self, local_file: &Path, remote_destination: &Path) -> Result<()> {
        let contents = read_file_to_string(&local_file)?;
        let byte_contents = contents.as_bytes();
        let size: u64 = fs::metadata(&local_file)?.len();
        info!("Writing to {:?}", remote_destination);
        let mut remote_file = self.session
            .scp_send(&remote_destination, 0o755, size, None)?;

        remote_file.write_all(&byte_contents)?;
        Ok(())
    }
}

pub fn authenticate_with_agent(host: &str, user: &str) -> Result<Connection> {
    let tcp = TcpStream::connect(host).chain_err(|| ErrorKind::HostConnection(host.to_string()))?;
    // Session::new() returns an Option<Session>, so there is little
    // error propogation needed.
    let mut session = Session::new().unwrap();
    session.handshake(&tcp)?;

    session.userauth_agent(user).chain_err(|| {
        ErrorKind::UserAuthentication(user.to_string(), host.to_string())
    })?;

    Ok(Connection {
        stream: tcp,
        session: Arc::new(session),
    })
}

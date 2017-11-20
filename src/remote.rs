use std::net::TcpStream;
use std::path::Path;
use ssh2::{FileStat, Session};
use errors::*;

pub struct Connection {
    pub stream: TcpStream,
    pub session: Session,
}

impl Connection {
    pub fn mkdir(&self, dir: &str) -> Result<()> {
        let dir_name = Path::new(&dir);
        let ftp = self.session.sftp()?;
        match self.stat(dir) {
            Err(_) => ftp.mkdir(dir_name, 0o755).chain_err(|| {
                ErrorKind::Mkdir(String::from(dir_name.to_string_lossy()))
            }),
            _ => Err(ErrorKind::DirectoryExists(String::from(dir_name.to_string_lossy())).into()),
        }
    }

    pub fn stat(&self, dir: &str) -> Result<FileStat> {
        let dir_name = Path::new(&dir);
        let ftp = self.session.sftp()?;
        ftp.stat(dir_name).chain_err(|| {
            ErrorKind::LStat(String::from(dir_name.to_string_lossy()))
        })
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
        session: session,
    })
}

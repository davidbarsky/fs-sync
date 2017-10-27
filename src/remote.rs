extern crate ssh2;

use std::io::prelude::*;
use std::net::TcpStream;
use self::ssh2::Session;

struct Remote {
    session: Session,
}

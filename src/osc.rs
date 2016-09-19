
use std::net::SocketAddr;

use super::net::{UdpConnection, UdpConnectionError};

use rosc;
use rosc::decoder::decode;
use rosc::encoder::encode;

#[derive(Debug)]
pub enum OscError {
    Encoding(rosc::OscError),
    Connection(UdpConnectionError),
}

impl From<UdpConnectionError> for OscError {
    fn from(err: UdpConnectionError) -> Self {
        OscError::Connection(err)
    }
}

impl From<rosc::OscError> for OscError {
    fn from(err: rosc::OscError) -> Self {
        OscError::Encoding(err)
    }
}

pub struct OscConnection {
    udp_connection: UdpConnection,
}

impl OscConnection {
    pub fn new(udp_connection: UdpConnection) -> OscConnection {
        OscConnection { udp_connection: udp_connection }
    }

    pub fn read(&mut self) -> Result<Option<rosc::OscPacket>, OscError> {
        match try!(self.udp_connection.next_message()) {
            Some(msg) => {
                let packet = try!(decode(&msg));
                info!("OscConnection: <- {:?}", packet);
                Ok(Some(packet))
            }
            None => Ok(None),
        }
    }

    pub fn write(&mut self, packet: &rosc::OscPacket) -> Result<(), OscError> {
        info!("OscConnection: -> {:?}", packet);
        let bytes = try!(encode(&packet));
        try!(self.udp_connection.send_message(&bytes));
        Ok(())
    }

    pub fn local_addr(&self) -> Result<(String, i32), OscError> {
        let addr = try!(self.udp_connection.local_addr());
        match addr {
            SocketAddr::V4(v4) => Ok((v4.ip().to_string(), v4.port() as i32)),
            SocketAddr::V6(v6) => Ok((v6.ip().to_string(), v6.port() as i32)),
        }
    }
}


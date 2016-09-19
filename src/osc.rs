
use std::net::SocketAddr;

use super::net::{UdpConnection};
use super::errors::*;

use rosc;
use rosc::decoder::decode;
use rosc::encoder::encode;

pub struct OscConnection {
    udp_connection: UdpConnection,
}

impl OscConnection {
    pub fn new(udp_connection: UdpConnection) -> OscConnection {
        OscConnection { udp_connection: udp_connection }
    }

    pub fn read(&mut self) -> Result<Option<rosc::OscPacket>> {
        match try!(self.udp_connection.next_message()) {
            Some(msg) => {
                let packet = try!(decode(&msg).map_err(ErrorKind::Osc));
                info!("OscConnection: <- {:?}", packet);
                Ok(Some(packet))
            }
            None => Ok(None),
        }
    }

    pub fn write(&mut self, packet: &rosc::OscPacket) -> Result<()> {
        info!("OscConnection: -> {:?}", packet);
        let bytes: Vec<u8> = try!(encode(&packet).map_err(ErrorKind::Osc));
        try!(self.udp_connection.send_message(&bytes));
        Ok(())
    }

    pub fn local_addr(&self) -> Result<(String, i32)> {
        let addr = try!(self.udp_connection.local_addr());
        match addr {
            SocketAddr::V4(v4) => Ok((v4.ip().to_string(), v4.port() as i32)),
            SocketAddr::V6(v6) => Ok((v6.ip().to_string(), v6.port() as i32)),
        }
    }
}


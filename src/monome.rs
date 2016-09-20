
use std::net::SocketAddrV4;
use std::thread;
use std::time::{Duration, Instant};

use rosc::{OscPacket, OscMessage, OscType};

use super::errors::*;
use super::net::UdpConnection;
use super::osc::OscConnection;
use super::actions::{MonomeAction, MonomeEvent};
use super::constants::PREFIX;

pub struct Monome {
    osc_connection: OscConnection,
}

impl Monome {
    pub fn new() -> Result<Monome> {
        let device_port = try!(Self::fetch_device_port_from_serialosc()) as u16;
        let local_addr = try!("127.0.0.1".parse().chain_err(|| "failed parsing addr"));
        let socket_addr = SocketAddrV4::new(local_addr, device_port);
        let conn = try!(UdpConnection::new(socket_addr));
        let osc_conn = OscConnection::new(conn);

        let mut monome = Monome { osc_connection: osc_conn };
        try!(monome.set_host_port());
        Ok(monome)
    }

    fn fetch_device_port_from_serialosc() -> Result<i32> {
        let conn = try!(UdpConnection::new("127.0.0.1:12002"));
        let mut osc_conn = OscConnection::new(conn);
        let (addr, port) = try!(osc_conn.local_addr());

        let packet = message("/serialosc/list",
                             vec![OscType::String(addr), OscType::Int(port)]);

        try!(osc_conn.write(&packet));

        let packet = try!(Self::spin_until_read(&mut osc_conn));

        if let OscPacket::Message(msg) = packet {
            if msg.addr != "/serialosc/device" {
                return Err("received message with addr other than /serialosc/device".into());
            }

            if let Some(args) = msg.args {
                if args.len() != 3 {
                    return Err("/serialosc/device message has incorrect number of args".into());
                }
                if let OscType::Int(device_port) = args[2] {
                    info!("Monome: device port {}", device_port);
                    return Ok(device_port);
                }
            }
        }
        Err("error initialising Monome".into())
    }

    fn spin_until_read(osc_connection: &mut OscConnection) -> Result<OscPacket> {
        let expiration = Instant::now() + Duration::from_secs(3);
        loop {
            if let Some(packet) = try!(osc_connection.read()) {
                return Ok(packet);
            }
            if Instant::now() > expiration {
                return Err("timeout waiting for serialosc response/monome".into());
            }
            thread::sleep(Duration::from_millis(10));
        }
    }

    pub fn poll(&mut self) -> Result<Option<MonomeEvent>> {
        if let Some(packet) = try!(self.osc_connection.read()) {
            Ok(self.parse(packet))
        } else {
            Ok(None)
        }
    }

    pub fn send(&mut self, action: &MonomeAction) -> Result<()> {
        let packet = action.to_packet();
        try!(self.osc_connection.write(&packet));
        Ok(())
    }

    fn parse(&mut self, packet: OscPacket) -> Option<MonomeEvent> {
        if let OscPacket::Message(ref message) = packet {
            if message.addr.starts_with("/sys/") {
                info!("Monome: sys info received: {}", message.addr);
                return None;
            }
            if message.addr.starts_with(PREFIX) {
                return self.parse_prefixed(message);
            } else {
                warn!("Monome: received message with known prefix: {:?}", message);
            }
        }
        None
    }

    fn parse_prefixed(&mut self, message: &OscMessage) -> Option<MonomeEvent> {
        if message.addr[PREFIX.len()..].starts_with("/grid/key") {
            if let Some(ref args) = message.args {
                if let (&OscType::Int(x), &OscType::Int(y), &OscType::Int(s)) = (&args[0],
                                                                                 &args[1],
                                                                                 &args[2]) {
                    return Some(MonomeEvent::Key(x as u8, y as u8, s != 0));
                }
                error!("Monome: failed to parse /grid/key args: {:?}", message);
            } else {
                error!("Monome: received /grid/key with no args: {:?}", message);
            }
        } else {
            warn!("Monome: received unsupported non-/sys message: {:?}",
                  message);
        }
        None
    }

    pub fn info(&mut self) -> Result<()> {
        let (addr, port) = try!(self.osc_connection.local_addr());
        let packet = message("/sys/info", vec![OscType::String(addr), OscType::Int(port)]);
        try!(self.osc_connection.write(&packet));
        Ok(())
    }

    fn set_host_port(&mut self) -> Result<()> {
        let (addr, port) = try!(self.osc_connection.local_addr());
        let port_packet = message("/sys/port", vec![OscType::Int(port)]);
        let host_packet = message("/sys/host", vec![OscType::String(addr)]);
        let prefix_packet = message("/sys/prefix", vec![OscType::String(PREFIX.into())]);

        try!(self.osc_connection.write(&port_packet));
        try!(self.osc_connection.write(&host_packet));
        try!(self.osc_connection.write(&prefix_packet));
        Ok(())
    }
}

fn message(addr: &str, args: Vec<OscType>) -> OscPacket {
    let message = OscMessage {
        addr: addr.to_owned(),
        args: Some(args),
    };
    OscPacket::Message(message)
}

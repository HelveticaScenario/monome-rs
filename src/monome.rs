
use std::io;
use std::net::{SocketAddrV4, AddrParseError};
use std::thread;
use std::time::{Duration, Instant};

use rosc::{OscPacket, OscMessage, OscType};

use super::net::{UdpConnection, UdpConnectionError};
use super::osc::OscConnection;

#[derive(Debug)]
pub enum MonomeError {
    Init,
    IoError(io::Error),
    OscError(super::osc::OscError),
    Timeout,
    UdpConnection(UdpConnectionError),
    AddrParse(AddrParseError),
}

impl From<io::Error> for MonomeError {
    fn from(err: io::Error) -> Self {
        MonomeError::IoError(err)
    }
}

impl From<super::osc::OscError> for MonomeError {
    fn from(err: super::osc::OscError) -> Self {
        MonomeError::OscError(err)
    }
}

impl From<AddrParseError> for MonomeError {
    fn from(err: AddrParseError) -> Self {
        MonomeError::AddrParse(err)
    }
}

impl From<UdpConnectionError> for MonomeError {
    fn from(err: UdpConnectionError) -> Self {
        MonomeError::UdpConnection(err)
    }
}

#[allow(enum_variant_names)]
pub enum MonomeAction<'a> {
    // TODO: tighten types
    LedSet(u8, u8, bool),
    LedAll(bool),
    LedIntensity(u8),
    LedMap(u8, u8, &'a [u8; 8]),
    LedRow(u8, u8, u8),
    LedCol(u8, u8, u8),
}

#[derive(Debug, Copy, Clone)]
pub enum MonomeEvent {
    Key(u8, u8, bool),
}

const PREFIX: &'static str = "/64";

pub struct Monome {
    osc_connection: OscConnection,
}

impl Monome {
    pub fn new() -> Result<Monome, MonomeError> {
        let device_port = try!(Self::fetch_device_port_from_serialosc()) as u16;

        let conn = try!(UdpConnection::new(SocketAddrV4::new(try!("127.0.0.1".parse()),
                                                             device_port)));
        let osc_conn = OscConnection::new(conn);

        let mut monome = Monome { osc_connection: osc_conn };
        try!(monome.set_host_port());
        Ok(monome)
    }

    fn fetch_device_port_from_serialosc() -> Result<i32, MonomeError> {
        let conn = try!(UdpConnection::new("127.0.0.1:12002"));
        let mut osc_conn = OscConnection::new(conn);
        let (addr, port) = try!(osc_conn.local_addr());

        let packet = OscPacket::Message(OscMessage {
            addr: "/serialosc/list".into(),
            args: Some(vec![OscType::String(addr), OscType::Int(port)]),
        });

        try!(osc_conn.write(&packet));

        let packet = try!(Self::spin_until_read(&mut osc_conn));

        if let OscPacket::Message(msg) = packet {
            if msg.addr != "/serialosc/device" {
                return Err(MonomeError::Init);
            }

            if let Some(args) = msg.args {
                if args.len() != 3 {
                    return Err(MonomeError::Init);
                }
                if let OscType::Int(device_port) = args[2] {
                    info!("Monome: device port {}", device_port);
                    return Ok(device_port);
                }
            }
        }
        Err(MonomeError::Init)
    }

    fn spin_until_read(osc_connection: &mut OscConnection) -> Result<OscPacket, MonomeError> {
        let expiration = Instant::now() + Duration::from_secs(3);
        loop {
            if let Some(packet) = try!(osc_connection.read()) {
                return Ok(packet);
            }
            if Instant::now() > expiration {
                return Err(MonomeError::Timeout);
            }
            thread::sleep(Duration::from_millis(10));
        }
    }

    pub fn poll(&mut self) -> Result<Option<MonomeEvent>, MonomeError> {
        if let Some(packet) = try!(self.osc_connection.read()) {
            Ok(self.parse(packet))
        } else {
            Ok(None)
        }
    }

    pub fn send(&mut self, action: &MonomeAction) -> Result<(), MonomeError> {
        let packet = match *action {
            MonomeAction::LedSet(x, y, s) => {
                Self::message("/grid/led/set",
                              vec![OscType::Int(x as i32),
                                   OscType::Int(y as i32),
                                   OscType::Int(s as i32)])
            }
            MonomeAction::LedAll(s) => Self::message("/grid/led/all", vec![OscType::Int(s as i32)]),
            MonomeAction::LedIntensity(i) => {
                Self::message("/grid/led/intensity", vec![OscType::Int(i as i32)])
            }
            MonomeAction::LedMap(x_off, y_off, masks) => {
                let mut args = Vec::with_capacity(10);
                args.push(OscType::Int(x_off as i32));
                args.push(OscType::Int(y_off as i32));
                for m in masks.iter().map(|m| OscType::Int(*m as i32)) {
                    args.push(m);
                }
                Self::message("/grid/led/map", args)
            }
            MonomeAction::LedCol(x, y_off, mask) => {
                Self::message("/grid/led/col",
                              vec![OscType::Int(x as i32),
                                   OscType::Int(y_off as i32),
                                   OscType::Int(mask as i32)])
            }
            MonomeAction::LedRow(x_off, y, mask) => {
                Self::message("/grid/led/row",
                              vec![OscType::Int(x_off as i32),
                                   OscType::Int(y as i32),
                                   OscType::Int(mask as i32)])
            }
        };
        try!(self.osc_connection.write(&packet));
        Ok(())
    }

    fn message(addr: &str, args: Vec<OscType>) -> OscPacket {
        let mut final_addr = String::with_capacity(addr.len() + PREFIX.len());
        final_addr.push_str(PREFIX);
        final_addr.push_str(addr);
        let message = OscMessage {
            addr: final_addr,
            args: Some(args),
        };
        OscPacket::Message(message)
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

    pub fn info(&mut self) -> Result<(), MonomeError> {
        let (addr, port) = try!(self.osc_connection.local_addr());
        let packet = OscPacket::Message(OscMessage {
            addr: "/sys/info".into(),
            args: Some(vec![OscType::String(addr), OscType::Int(port)]),
        });
        try!(self.osc_connection.write(&packet));
        Ok(())
    }

    fn set_host_port(&mut self) -> Result<(), MonomeError> {
        let (addr, port) = try!(self.osc_connection.local_addr());
        let port_packet = OscPacket::Message(OscMessage {
            addr: "/sys/port".into(),
            args: Some(vec![OscType::Int(port)]),
        });
        let host_packet = OscPacket::Message(OscMessage {
            addr: "/sys/host".into(),
            args: Some(vec![OscType::String(addr)]),
        });
        let prefix_packet = OscPacket::Message(OscMessage {
            addr: "/sys/prefix".into(),
            args: Some(vec![OscType::String(PREFIX.into())]),
        });

        try!(self.osc_connection.write(&port_packet));
        try!(self.osc_connection.write(&host_packet));
        try!(self.osc_connection.write(&prefix_packet));
        Ok(())
    }
}

use rosc::{OscMessage, OscPacket, OscType};

use super::errors::*;
use super::constants::PREFIX;

// TODO: split out grid and add tilt & arc support
// TODO: support setting intensities in various actions
#[allow(enum_variant_names)]
pub enum MonomeAction<'a> {
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

impl<'a> MonomeAction<'a> {
    pub fn to_packet(&self) -> OscPacket {
        let mut addr = PREFIX.to_string();
        addr.push_str(&self.to_addr());
        OscPacket::Message(OscMessage {
            addr: addr,
            args: Some(self.to_args()),
        })
    }

    fn to_addr(&self) -> String {
        match *self {
                MonomeAction::LedSet(..) => "/grid/led/set",
                MonomeAction::LedAll(..) => "/grid/led/all",
                MonomeAction::LedIntensity(..) => "/grid/led/intensity",
                MonomeAction::LedMap(..) => "/grid/led/map",
                MonomeAction::LedCol(..) => "/grid/led/col",
                MonomeAction::LedRow(..) => "/grid/led/row",
            }
            .into()
    }

    fn to_args(&self) -> Vec<OscType> {
        match *self {
            MonomeAction::LedSet(x, y, s) => {
                vec![OscType::Int(x as i32), OscType::Int(y as i32), OscType::Int(s as i32)]
            }
            MonomeAction::LedAll(s) => vec![OscType::Int(s as i32)],
            MonomeAction::LedIntensity(i) => vec![OscType::Int(i as i32)],
            MonomeAction::LedMap(x_off, y_off, masks) => {
                let mut args = Vec::with_capacity(10);
                args.push(OscType::Int(x_off as i32));
                args.push(OscType::Int(y_off as i32));
                for m in masks.iter().map(|m| OscType::Int(*m as i32)) {
                    args.push(m);
                }
                args
            }
            MonomeAction::LedCol(x, y_off, mask) => {
                vec![OscType::Int(x as i32), OscType::Int(y_off as i32), OscType::Int(mask as i32)]
            }
            MonomeAction::LedRow(x_off, y, mask) => {
                vec![OscType::Int(x_off as i32), OscType::Int(y as i32), OscType::Int(mask as i32)]
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rosc::{OscPacket, OscMessage, OscType};

    #[test]
    fn led_set_to_packet() {
        let packet = MonomeAction::LedSet(3, 7, true).to_packet();
        assert_eq!(packet,
                   OscPacket::Message(OscMessage {
                       addr: "/64/grid/led/set".into(),
                       args: Some(vec![OscType::Int(3), OscType::Int(7), OscType::Int(1)]),
                   }));
    }

    #[test]
    fn led_all_to_packet() {
        let packet = MonomeAction::LedAll(true).to_packet();
        assert_eq!(packet,
                   OscPacket::Message(OscMessage {
                       addr: "/64/grid/led/all".into(),
                       args: Some(vec![OscType::Int(1)]),
                   }));
    }

    #[test]
    fn led_intensity_to_packet() {
        let packet = MonomeAction::LedIntensity(7).to_packet();
        assert_eq!(packet,
                   OscPacket::Message(OscMessage {
                       addr: "/64/grid/led/intensity".into(),
                       args: Some(vec![OscType::Int(7)]),
                   }));
    }

    #[test]
    fn led_map_to_packet() {
        let packet = MonomeAction::LedMap(3, 7, &[0, 1, 2, 3, 4, 5, 6, 7]).to_packet();
        assert_eq!(packet,
                   OscPacket::Message(OscMessage {
                       addr: "/64/grid/led/map".into(),
                       args: Some(vec![OscType::Int(3),
                                       OscType::Int(7),
                                       OscType::Int(0),
                                       OscType::Int(1),
                                       OscType::Int(2),
                                       OscType::Int(3),
                                       OscType::Int(4),
                                       OscType::Int(5),
                                       OscType::Int(6),
                                       OscType::Int(7)]),
                   }));
    }

    #[test]
    fn led_col_to_packet() {
        let packet = MonomeAction::LedCol(3, 7, 16).to_packet();
        assert_eq!(packet,
                   OscPacket::Message(OscMessage {
                       addr: "/64/grid/led/col".into(),
                       args: Some(vec![OscType::Int(3), OscType::Int(7), OscType::Int(16)]),
                   }));
    }

    #[test]
    fn led_row_to_packet() {
        let packet = MonomeAction::LedRow(3, 7, 17).to_packet();
        assert_eq!(packet,
                   OscPacket::Message(OscMessage {
                       addr: "/64/grid/led/row".into(),
                       args: Some(vec![OscType::Int(3), OscType::Int(7), OscType::Int(17)]),
                   }));
    }
}

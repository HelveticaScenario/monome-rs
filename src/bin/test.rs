#![feature(plugin)]
#![feature(conservative_impl_trait)]

#![plugin(clippy)]

use std::thread;
use std::time::Duration;

#[macro_use]
extern crate log;
extern crate env_logger;
extern crate rosc;
extern crate libmonome;
extern crate ndarray;

use libmonome::{Monome, MonomeEvent, MonomeAction};
use ndarray::{Array, ArrayBase, Ix};


fn main() {
    env_logger::init().unwrap();
    let mut monome = Monome::new().unwrap();

    monome.info().unwrap();

    monome.send(&MonomeAction::LedAll(false)).unwrap();

    let mut grid_state: Array<bool, (Ix, Ix)> = ArrayBase::from_elem((16, 8), false);
    let mut map: [u8; 8] = [0, 0, 0, 0, 0, 0, 0, 0];

    loop {
        if let Some(event) = monome.poll().unwrap() {
            match event {
                MonomeEvent::Key(x, y, s) => {
                    if s {
                        if let Some(state) = grid_state.get_mut((x as usize, y as usize)) {
                            *state = !*state;
                            map[y as usize] <<= 1;
                            if map[y as usize] == 0 {
                                map[y as usize] = 1;
                            }
                            monome.send(&MonomeAction::LedMap(0, 0, &map)).unwrap();
                        } else {
                            error!("state out of range: {}, {}", x, y);
                        }
                    }
                }
            }
            //            println!("monome poll: {:?}", event);
        }
        thread::sleep(Duration::from_millis(1));
    }
}

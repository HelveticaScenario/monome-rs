#![feature(plugin)]

#![plugin(clippy)]

#[macro_use]
extern crate log;
extern crate rosc;

pub mod net;
pub mod osc;
pub mod monome;

pub use monome::{Monome, MonomeError, MonomeEvent, MonomeAction};

#![recursion_limit = "1024"]
#![feature(plugin)]
#![plugin(clippy)]

#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate log;
extern crate rosc;

pub mod errors;
pub mod net;
pub mod osc;
pub mod monome;

pub use monome::{Monome, MonomeEvent, MonomeAction};

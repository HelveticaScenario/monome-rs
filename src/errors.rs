#![allow(redundant_closure)]

use rosc;
use std::sync::mpsc::TryRecvError;

error_chain! {
    foreign_links {
        TryRecvError, TryRecv;
    }

    errors {
        Osc(e: rosc::OscError) {
            description("osc encoding error")
            display("osc encoding error: '{:?}'", e)
        }
    }
}

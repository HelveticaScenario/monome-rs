use std::net::{ToSocketAddrs, UdpSocket, SocketAddr, SocketAddrV4};
use std::io;
use std::thread;
use std::sync::mpsc::channel;
use std::sync::mpsc::{Receiver, TryRecvError};

use super::errors::*;

// TODO: refactor to use tokio/futures, not threading
pub struct UdpConnection {
    socket: UdpSocket,
    message_receiver: Receiver<Vec<u8>>,
    thread_handle: thread::JoinHandle<io::Result<()>>,
}

impl UdpConnection {
    pub fn new<A: ToSocketAddrs>(addr: A) -> Result<Self> {
        info!("UdpConnection new");
        let socket = try!(Self::try_bind_free());
        try!(socket.connect(addr).chain_err(|| "unable to connect to udp socket"));
        let thread_builder = thread::Builder::new();
        let socket_clone = try!(socket.try_clone().chain_err(|| "unable to clone udp socket"));
        let (tx, rx) = channel();

        let spawn_result = thread_builder.name("udp_connection".into()).spawn(move || {
            // TODO: thread needs a timeout and self-kill sentinel
            info!("UdpConnection thread init");
            let mut recv_buf = [0u8; 64 * 1000];
            loop {
                let size = try!(socket_clone.recv(&mut recv_buf));
                let vec = recv_buf[..size].to_vec();
                debug!("UdpConnection data received: {:?}", vec);
                tx.send(vec).unwrap();
            }
        });
        let handle = try!(spawn_result.chain_err(|| "unable to spawn thread"));

        Ok(UdpConnection {
            socket: socket,
            message_receiver: rx,
            thread_handle: handle,
        })
    }

    fn try_bind_free() -> Result<UdpSocket> {
        let mut port = 10000;
        let mut counter = 0;
        let local_addr = try!("127.0.0.1".parse().chain_err(|| "unable to parse local addr"));

        loop {
            let bind_attempt = UdpSocket::bind(SocketAddrV4::new(local_addr, port));
            match bind_attempt {
                Ok(socket) => return Ok(socket),
                Err(err) => {
                    if err.kind() != io::ErrorKind::AddrInUse {
                        return Err(err)
                            .chain_err(|| "unknown error when attempting to bind udp socket");
                    }
                }
            }
            port += 1;
            counter += 1;
            if counter > 50000 {
                return Err(io::Error::last_os_error())
                    .chain_err(|| "unable to find free port to bind udp socket to");
            }
        }
    }

    pub fn next_message(&mut self) -> Result<Option<Vec<u8>>> {
        match self.message_receiver.try_recv() {
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(TryRecvError::Disconnected.into()),
            Ok(value) => Ok(Some(value)),
        }
    }

    pub fn send_message(&mut self, data: &[u8]) -> Result<usize> {
        self.socket.send(data).chain_err(|| "unable to send message to socket")
    }

    pub fn join(self) -> io::Result<()> {
        self.thread_handle.join().unwrap()
    }

    pub fn local_addr(&self) -> Result<SocketAddr> {
        Ok(try!(self.socket.local_addr().chain_err(|| "unable to fetch local addr from socket")))
    }
}

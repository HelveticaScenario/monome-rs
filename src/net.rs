use std::net::{AddrParseError, ToSocketAddrs, UdpSocket, SocketAddr, SocketAddrV4};
use std::io;
use std::thread;
use std::sync::mpsc::channel;
use std::sync::mpsc::{Receiver, TryRecvError};

#[derive(Debug)]
pub enum UdpConnectionError {
    Receive(TryRecvError),
    Io(io::Error),
    AddrParse(AddrParseError),
}

impl From<TryRecvError> for UdpConnectionError {
    fn from(err: TryRecvError) -> Self {
        UdpConnectionError::Receive(err)
    }
}

impl From<io::Error> for UdpConnectionError {
    fn from(err: io::Error) -> Self {
        UdpConnectionError::Io(err)
    }
}

impl From<AddrParseError> for UdpConnectionError {
    fn from(err: AddrParseError) -> Self {
        UdpConnectionError::AddrParse(err)
    }
}

pub struct UdpConnection {
    socket: UdpSocket,
    message_receiver: Receiver<Vec<u8>>,
    thread_handle: thread::JoinHandle<io::Result<()>>,
}

impl UdpConnection {
    pub fn new<A: ToSocketAddrs>(addr: A) -> Result<Self, UdpConnectionError> {
        info!("UdpConnection new");
        let socket = try!(Self::try_bind_free());
        try!(socket.connect(addr));
        let thread_builder = thread::Builder::new();
        let socket_clone = try!(socket.try_clone());
        let (tx, rx) = channel();

        let handle = try!(thread_builder.name("udp_connection".into()).spawn(move || {
            // TODO: thread needs a timeout and self-kill sentinel
            info!("UdpConnection thread init");
            let mut recv_buf = [0u8; 64 * 1000];
            loop {
                let size = try!(socket_clone.recv(&mut recv_buf));
                let vec = recv_buf[..size].to_vec();
                debug!("UdpConnection data received: {:?}", vec);
                tx.send(vec).unwrap();
            }
        }));

        Ok(UdpConnection {
            socket: socket,
            message_receiver: rx,
            thread_handle: handle,
        })
    }

    fn try_bind_free() -> Result<UdpSocket, UdpConnectionError> {
        let mut port = 10000;
        let mut counter = 0;
        let local_addr = try!("127.0.0.1".parse());

        loop {
            let bind_attempt = UdpSocket::bind(SocketAddrV4::new(local_addr, port));
            match bind_attempt {
                Ok(socket) => return Ok(socket),
                Err(err) => {
                    if err.kind() != io::ErrorKind::AddrInUse {
                        return Err(UdpConnectionError::Io(err));
                    }
                }
            }
            port += 1;
            counter += 1;
            if counter > 50000 {
                return Err(UdpConnectionError::Io(io::Error::last_os_error()));
            }
        }
    }

    pub fn next_message(&mut self) -> Result<Option<Vec<u8>>, UdpConnectionError> {
        match self.message_receiver.try_recv() {
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(TryRecvError::Disconnected.into()),
            Ok(value) => Ok(Some(value)),
        }
    }

    pub fn send_message(&mut self, data: &[u8]) -> Result<usize, UdpConnectionError> {
        self.socket.send(data).map_err(|e| e.into())
    }

    pub fn join(self) -> io::Result<()> {
        self.thread_handle.join().unwrap()
    }

    pub fn local_addr(&self) -> Result<SocketAddr, UdpConnectionError> {
        Ok(try!(self.socket.local_addr()))
    }
}

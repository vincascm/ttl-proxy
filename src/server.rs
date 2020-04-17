use std::{
    mem,
    net::{SocketAddr, SocketAddrV4},
    os::unix::io::AsRawFd,
};

use failure::{format_err, Fallible};
use socks5::connect_without_auth;
use tokio::{
    io::{copy, ErrorKind},
    net::{lookup_host, TcpListener, TcpStream},
};

pub struct Server {
    client: TcpStream,
    listen: SocketAddr,
    server: SocketAddr,
    default_target_addr: SocketAddr,
}

impl Server {
    pub fn new(
        listen: SocketAddr,
        client: TcpStream,
        server: SocketAddr,
        default_target_addr: SocketAddr,
    ) -> Self {
        Self {
            listen,
            client,
            server,
            default_target_addr,
        }
    }

    pub async fn run(
        listen: &str,
        socks5_server_addr: &str,
        default_target_addr: &str,
    ) -> Fallible<()> {
        let listen = lookup_host(listen).await?.next().ok_or_else(|| {
            let e: std::io::Error = ErrorKind::AddrNotAvailable.into();
            e
        })?;
        let socks5_server_addr =
            lookup_host(socks5_server_addr)
                .await?
                .next()
                .ok_or_else(|| {
                    let e: std::io::Error = ErrorKind::AddrNotAvailable.into();
                    e
                })?;
        let default_target_addr =
            lookup_host(default_target_addr)
                .await?
                .next()
                .ok_or_else(|| {
                    let e: std::io::Error = ErrorKind::AddrNotAvailable.into();
                    e
                })?;
        let mut listener = TcpListener::bind(&listen).await?;

        loop {
            let (stream, _) = listener.accept().await?;
            tokio::spawn(async move {
                let mut server =
                    Server::new(listen, stream, socks5_server_addr, default_target_addr);
                if let Err(e) = server.proxy().await {
                    println!("error: {}", e);
                }
            });
        }
    }

    async fn proxy(&mut self) -> Fallible<()> {
        let dest_addr = match self.get_dest_addr() {
            Ok(addr) => {
                if addr == self.listen {
                    self.default_target_addr
                } else {
                    addr
                }
            }
            Err(_) => self.default_target_addr,
        };
        let mut srv = connect_without_auth(self.server, dest_addr).await?;
        let (mut srv_r, mut srv_w) = srv.split();
        let (mut r, mut w) = self.client.split();
        futures::future::select(copy(&mut r, &mut srv_w), copy(&mut srv_r, &mut w)).await;
        Ok(())
    }

    fn get_dest_addr(&self) -> Fallible<SocketAddr> {
        use libc::{__errno_location, getsockopt, sockaddr_in, socklen_t, SOL_IP, SO_ORIGINAL_DST};
        use std::ffi::c_void;

        let fd = self.client.as_raw_fd();
        unsafe {
            let mut destaddr: sockaddr_in = mem::zeroed();
            let mut socklen = mem::size_of::<sockaddr_in>() as socklen_t;
            let r = getsockopt(
                fd,
                SOL_IP,
                SO_ORIGINAL_DST,
                &mut destaddr as *mut sockaddr_in as *mut c_void,
                &mut socklen as *mut socklen_t,
            );
            if r == -1 {
                let errno = *__errno_location() as i32;
                Err(format_err!("get original address error, errno: {}", errno))
            } else {
                let addr = SocketAddrV4::new(
                    u32::from_be(destaddr.sin_addr.s_addr).into(),
                    u16::from_be(destaddr.sin_port),
                );
                Ok(SocketAddr::V4(addr))
            }
        }
    }
}

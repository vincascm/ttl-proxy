use std::{
    mem,
    net::{SocketAddr, SocketAddrV4},
    net::{TcpListener, TcpStream},
    os::unix::io::AsRawFd,
};

use anyhow::{anyhow, Result};
use futures::io::{copy, AsyncReadExt};
use smol::{Async, Task};
use socks5::connect_without_auth;

pub struct Server {
    client: Async<TcpStream>,
    listen: SocketAddr,
    server: SocketAddr,
    default_target_addr: SocketAddr,
}

impl Server {
    pub fn new(
        listen: SocketAddr,
        client: Async<TcpStream>,
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

    pub fn run(
        listen: &str,
        socks5_server_addr: &str,
        default_target_addr: &str,
    ) -> Result<()> {
        let listen = listen.parse()?;
        let socks5_server_addr = socks5_server_addr.parse()?;
        let default_target_addr = default_target_addr.parse()?;
        smol::run(async {
            let listener = Async::<TcpListener>::bind(&listen)?;
            loop {
                let (stream, _) = listener.accept().await?;
                Task::spawn(async move {
                    let server = Server::new(listen, stream, socks5_server_addr, default_target_addr);
                    if let Err(e) = server.proxy().await {
                        println!("error: {}", e);
                    }
                }).detach();
            }
        })
    }

    async fn proxy(self) -> Result<()> {
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
        let srv = connect_without_auth(self.server, dest_addr).await?;
        let (mut srv_r, mut srv_w) = srv.split();
        let (mut r, mut w) = self.client.split();
        futures::future::select(copy(&mut r, &mut srv_w), copy(&mut srv_r, &mut w)).await;
        Ok(())
    }

    fn get_dest_addr(&self) -> Result<SocketAddr> {
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
                Err(anyhow!("get original address error, errno: {}", errno))
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

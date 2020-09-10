use std::{mem, os::unix::io::AsRawFd, time::Duration};

use anyhow::{anyhow, Error, Result};
use smol::{
    block_on,
    future::{race, FutureExt},
    io::copy,
    net::{resolve, SocketAddr, SocketAddrV4, TcpListener, TcpStream},
    spawn, Timer,
};
use socks5::connect_without_auth;

const TIMEOUT: Duration = Duration::from_secs(60);

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

    pub fn run(listen: &str, socks5_server_addr: &str, default_target_addr: &str) -> Result<()> {
        block_on(async {
            let listen = Self::resolve(listen, anyhow!("invalid listen address")).await?;
            let socks5_server_addr =
                Self::resolve(socks5_server_addr, anyhow!("invalid socks5 server address")).await?;
            let default_target_addr = Self::resolve(
                default_target_addr,
                anyhow!("invalid default target address"),
            )
            .await?;
            let listener = TcpListener::bind(listen).await?;
            loop {
                let (stream, _) = listener.accept().await?;
                spawn(async move {
                    let server =
                        Server::new(listen, stream, socks5_server_addr, default_target_addr);
                    if let Err(e) = server.proxy().await {
                        println!("error: {}", e);
                    }
                })
                .detach();
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
        let srv = connect_without_auth(self.server, dest_addr.into()).await?;
        srv.set_nodelay(true)?;
        let left = copy(&self.client, &srv).or(async { Self::timeout().await });
        let right = copy(&srv, &self.client).or(async { Self::timeout().await });
        race(left, right)
            .await
            .map(|_| ())
            .map_err(|_| anyhow!("io error"))
    }

    async fn resolve(addr: &str, err: Error) -> Result<SocketAddr> {
        Ok(*resolve(addr).await?.first().ok_or(err)?)
    }

    async fn timeout() -> Result<u64, std::io::Error> {
        Timer::after(TIMEOUT).await;
        Err(std::io::ErrorKind::TimedOut.into())
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

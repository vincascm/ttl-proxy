#[macro_use]
extern crate clap;

use clap::Arg;

mod server;

fn main() {
    let matches = clap::app_from_crate!()
        .arg(Arg::with_name("listen")
            .short("l")
            .long("listen")
            .takes_value(true)
            .help("listen address"))
        .arg(Arg::with_name("socks5")
            .short("s")
            .long("socks5")
            .takes_value(true)
            .help("socks5 server address"))
        .arg(Arg::with_name("default")
            .short("d")
            .long("default")
            .takes_value(true)
            .help("default target address"))
        .get_matches();
    let listen = matches.value_of("listen").unwrap_or("127.0.0.1:10800");
    let socks5 = matches.value_of("socks5").unwrap_or("127.0.0.1:1080");
    let default = matches.value_of("default").unwrap_or("127.0.0.1:53530");

    let mut rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => return println!("tokio runtime init error: {}", e),
    };
    if let Err(e) = rt.block_on(server::Server::run(&listen, &socks5, &default)) {
        println!("startup error: {}", e)
    }
}

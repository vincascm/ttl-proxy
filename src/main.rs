mod server;

macro_rules! help {
    () => {
        r#"
options:
    -h  show help
    -s <address> assgin a socks5 server address
    -l <address> assgin a listen address
    -V  show version
"#;
    };
}

const LISTEN_ADDR: &str = "127.0.0.1:10800";
const SOCKS5_ADDR: &str = "127.0.0.1:1080";

fn get_args(args: &mut std::env::Args) -> Result<(Option<String>, Option<String>), &'static str> {
    match args.next() {
        Some(opts) => match opts.as_str() {
            "-h" => Err(concat!(env!("CARGO_PKG_NAME"), "\n", help!())),
            "-s" => match args.next() {
                Some(srv) => Ok((None, Some(srv))),
                None => Err("invalid server argument, required a value."),
            },
            "-l" => match args.next() {
                Some(listen) => Ok((Some(listen), None)),
                None => Err("invalid listen argument, required a value."),
            },
            "-V" => Err(env!("CARGO_PKG_VERSION")),
            _ => Err(r#"invalid options, use "-h" to show help"#),
        },
        None => Ok((Some(LISTEN_ADDR.to_string()), Some(SOCKS5_ADDR.to_string()))),
    }
}

fn main() {
    let mut args = std::env::args();
    args.next(); // skip app's name
    let (listen, srv) = match get_args(&mut args) {
        Ok(x) => x,
        Err(e) => return println!("{}", e),
    };
    let (next_listen, next_srv) = match get_args(&mut args) {
        Ok(x) => x,
        Err(e) => return println!("{}", e),
    };
    let listen = listen.or(next_listen).unwrap_or_else(|| LISTEN_ADDR.to_string());
    let srv = srv.or(next_srv).unwrap_or_else(|| SOCKS5_ADDR.to_string());

    let mut rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => return println!("tokio runtime init error: {}", e),
    };
    if let Err(e) = rt.block_on(server::Server::run(&listen, &srv)) {
        println!("startup error: {}", e)
    }
}

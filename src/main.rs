use clap::Parser;

mod server;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// listen address
    #[arg(short, long, default_value = "127.0.0.1:10800")]
    listen: String,
    /// socks5 server address
    #[arg(short, long, default_value = "127.0.0.1:1080")]
    socks5: String,
    /// default target address
    #[arg(short, long, default_value = "1.1.1.1:53")]
    default: String,
}

fn main() {
    let args = Args::parse();
    if let Err(e) = server::Server::run(&args.listen, &args.socks5, &args.default) {
        println!("startup error: {}", e)
    }
}

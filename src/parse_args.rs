use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Bind socket to this host
    #[arg(long, env, default_value = "0.0.0.0")]
    pub host: String,
    /// Bind to this socket port
    #[arg(short, long, env, default_value_t = 53)]
    pub port: u32,
    /// Time to live in seconds (TTL)
    #[arg(short, long, env, default_value_t = 5 * 60)]
    pub ttl: u32,
}

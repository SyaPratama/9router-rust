use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "9router")]
#[command(about = "Native Rust AI Gateway with built-in Headroom Context Compression", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Start the 9router AI gateway server
    Start {
        #[arg(short, long, default_value_t = 20128)]
        port: u16,
        #[arg(long, default_value = "0.0.0.0")]
        host: String,
        #[arg(long)]
        daemon: bool,
    },
    /// Stop any running 9router background server
    Stop,
    /// View server status, uptime and memory metrics
    Status,
    /// Open the interactive terminal configuration menu
    Menu,
    /// Run performance benchmark comparison against 9Router (Next.js) & OmniRoute
    Bench {
        #[arg(short, long, default_value_t = 50)]
        concurrency: usize,
        #[arg(short, long, default_value_t = 1000)]
        requests: usize,
    },
}

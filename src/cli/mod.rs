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
        #[arg(short, long)]
        port: Option<u16>,
        #[arg(long)]
        host: Option<String>,
        #[arg(long)]
        daemon: bool,
    },
    /// Stop any running 9router background server
    Stop,
    /// View server status, uptime and memory metrics
    Status,
    /// Open the interactive terminal configuration menu
    Menu,
    /// Benchmark the local Rust health endpoint over TCP
    Bench {
        #[arg(short, long, default_value_t = 50)]
        concurrency: usize,
        #[arg(short, long, default_value_t = 1000)]
        requests: usize,
    },
}

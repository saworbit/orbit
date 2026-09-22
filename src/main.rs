use clap::Parser;
use orbit::cli::Cli;

fn main() {
    let _request = Cli::parse().into_request();
}

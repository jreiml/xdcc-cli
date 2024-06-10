use anyhow::{anyhow, Result};
use clap::Parser;

use crate::irc::command::xdcc::Xdcc;
use crate::irc::IrcDccClient;
use crate::package_downloader::PackageDownloader;

mod irc;
mod package_downloader;

/// Program to run an XDCC command in an IRC server
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    // XDCC command to run
    #[arg()]
    command: String,
    // IRC network to connect to
    #[arg(short, long, default_value="irc.rizon.net:6667")]
    server: String,
    // Channel to join upon connecting
    #[arg(short, long, default_value="#nibl")]
    channel: String,
    // Nickname for connecting to the IRC network
    #[arg(short, long, default_value="xdcc-cli")]
    nickname: String,
    // Seconds to wait for downloads before timing out
    #[arg(short, long, default_value="30")]
    timeout_seconds: u64,
}


#[tokio::main]
async fn main() -> Result<()> {
    // Set up logging
    env_logger::init();
    let args = Args::parse();
    let command = Xdcc::try_from(args.command.as_str())
        .map_err(|err| anyhow!("Failed to parse command: {}", err))?;

    let mut client = IrcDccClient::connect(&args.server).await?;
    client.login(args.nickname).await?;
    client.join(args.channel).await?;
    // cancel previous transfers before starting a new one
    client.send_dcc_request(Xdcc::Remove(command.recipient().to_string(), None))?;
    client.send_dcc_request(Xdcc::Cancel(command.recipient().to_string()))?;
    let downloader = PackageDownloader::new(client, command, args.timeout_seconds).await?;
    downloader.download_packages().await?;
    Ok(())
}


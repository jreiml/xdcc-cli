use std::net::Ipv4Addr;

use anyhow::Result;
use lazy_static::lazy_static;
use regex::Regex;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufWriter};
use tokio::net::TcpStream;

lazy_static! {
    static ref CTCP_DCC_SEND_REGEX: Regex =
        Regex::new(r#"\x01DCC SEND "?([^"]*)"? (\d+) (\d+) (\d+)\x01"#).unwrap();
}

#[derive(Debug)]
pub struct Dcc {
    pub(crate) sender: String,
    pub(crate) dcc_type: DccType,
}

#[derive(Debug)]
pub enum DccType {
    Send(Send)
}

#[derive(Debug)]
pub struct Send {
    pub filename: String,
    ip: Ipv4Addr,
    port: u16,
    file_size: usize,
}

impl<'a> TryFrom<&'a str> for DccType {
    type Error = &'a str;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        if let Some(captures) = CTCP_DCC_SEND_REGEX.captures(value) {
            let ip_number = captures[2].parse::<u32>().unwrap();
            let send = Send {
                filename: captures[1].to_string(),
                ip: Ipv4Addr::from(ip_number),
                port: captures[3].parse::<u16>().unwrap(),
                file_size: captures[4].parse::<usize>().unwrap(),
            };
            Ok(Self::Send(send))
        } else {
            Err(value)
        }
    }
}

impl Send {
    pub fn normalized_filename(&self) -> String {
        self.filename.replace(" ", "_")
    }

    pub async fn start_download(&self) -> Result<()> {
        let mut file = BufWriter::new(File::create(self.normalized_filename()).await?);
        let mut stream = TcpStream::connect((self.ip, self.port)).await?;
        let mut buffer = [0; 4096];
        let mut progress: usize = 0;
        while progress < self.file_size {
            let count = stream.read(&mut buffer[..]).await?;
            file.write_all(&mut buffer[..count]).await?;
            progress += count;
        }
        file.flush().await?;
        stream.shutdown().await?;
        Ok(())
    }
}

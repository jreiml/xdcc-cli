use std::time::Duration;
use anyhow::{Result, bail};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;
use tokio::time::timeout;
use crate::irc::command::ctcp::dcc::{Dcc, DccType};
use crate::irc::command::xdcc::Xdcc;
use crate::irc::IrcDccClient;


fn calculate_total_downloads(command: &Xdcc) -> Result<u32> {
    match command {
        Xdcc::Send(_, _) => Ok(1),
        Xdcc::Batch(_, packages) => Ok(packages.len() as u32),
        _ => bail!("Only SEND and BATCH are supported!"),
    }
}

pub struct PackageDownloader {
    client: IrcDccClient,
    downloads: Vec<JoinHandle<Result<()>>>,
    total_downloads: u32,
    handled_downloads: u32,
    quit_on_next_timeout: bool,
    timeout_duration: Duration,
    finished_sender: UnboundedSender<String>,
    print_handle: JoinHandle<()>,
}

impl PackageDownloader {
    pub async fn new(mut client: IrcDccClient, command: Xdcc, timeout_seconds: u64) -> Result<Self> {
        let total_downloads = calculate_total_downloads(&command)?;
        client.send_dcc_request(command.clone())?;

        let (finished_sender, finished_receiver) = tokio::sync::mpsc::unbounded_channel();
        let print_handle = tokio::spawn(PackageDownloader::print_finished_downloads(
            finished_receiver, total_downloads));

        Ok(Self {
            client,
            downloads: Vec::new(),
            total_downloads,
            handled_downloads: 0,
            quit_on_next_timeout: false,
            timeout_duration: Duration::from_secs(timeout_seconds / 2),
            finished_sender,
            print_handle,
        })
    }


    async fn print_finished_downloads(mut finished_receiver: UnboundedReceiver<String>,
                                      total_downloads: u32) {
        let mut handled_downloads = 0u32;
        loop {
            if handled_downloads == total_downloads {
                finished_receiver.close();
                return;
            }
            let filename = match finished_receiver.recv().await {
                Some(message) => message,
                None => return,
            };
            println!("{}", filename);
            handled_downloads += 1;
        }
    }

    async fn handle_timeout(&mut self) -> Result<()> {
        if self.downloads.iter().any(|f| !f.is_finished()) {
            Ok(())
        } else if self.quit_on_next_timeout {
            self.client.quit().await?;
            bail!("Timed out waiting for packages!")
        } else {
            self.quit_on_next_timeout = true;
            Ok(())
        }
    }

    async fn handle_download(&mut self, dcc: Dcc) -> Result<()> {
        let DccType::Send(send) = dcc.dcc_type;
        let normalized_filename = send.normalized_filename();
        log::info!("Accepting download of {} from {}.", normalized_filename, dcc.sender);
        let download_finished_sender = self.finished_sender.clone();
        let download = tokio::spawn(async move {
            send.start_download().await?;
            download_finished_sender.send(normalized_filename)?;
            Ok(())
        });
        self.downloads.push(download);
        Ok(())
    }

    async fn wait_for_download_completion(mut self) -> Result<()> {
        for download in self.downloads {
            let _ = download.await?;
        }
        self.client.quit().await?;
        self.print_handle.await?;
        Ok(())
    }

    async fn wait_for_dcc_with_timeout(&mut self) -> Result<Option<Dcc>> {
        let result = match timeout(self.timeout_duration, self.client.wait_for_dcc()).await {
            Ok(result) => result,
            Err(_) => {
                self.handle_timeout().await?;
                return Ok(None);
            }
        };

        let dcc = match result? {
            Some(dcc) => dcc,
            None => {
                self.client.quit().await?;
                bail!("Connection closed before all packages were downloaded!")
            }
        };

        self.quit_on_next_timeout = false;
        Ok(Some(dcc))
    }

    pub async fn download_packages(mut self) -> Result<()> {
        loop {
            if self.handled_downloads == self.total_downloads {
                self.wait_for_download_completion().await?;
                return Ok(())
            }

            if let Some(dcc) = self.wait_for_dcc_with_timeout().await? {
                self.handle_download(dcc).await?;
                self.handled_downloads += 1;
            }
        }
    }
}

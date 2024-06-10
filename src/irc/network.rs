use anyhow::Result;
use futures::StreamExt;
use tokio::io::AsyncWriteExt;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpStream;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio_util::codec::{FramedRead, LinesCodec};

use crate::irc::command::{ClientCommand, ServerCommand};

pub(crate) async fn connect(
    server: &str,
    server_command_sender: UnboundedSender<ServerCommand>,
    client_command_receiver: UnboundedReceiver<ClientCommand>,
) -> Result<()> {
    let stream = TcpStream::connect(server).await?;
    let (reader, writer) = stream.into_split();
    tokio::spawn(read_server_commands(reader, server_command_sender));
    tokio::spawn(write_client_commands(writer, client_command_receiver));
    Ok(())
}

async fn read_server_commands(
    reader: OwnedReadHalf,
    command_sender: UnboundedSender<ServerCommand>,
) {
    let mut reader = FramedRead::new(reader, LinesCodec::new());

    loop {
        let result = match reader.next().await {
            Some(result) => result,
            None => {
                return;
            }
        };
        let command_str = match result {
            Ok(command_str) => command_str,
            Err(e) => {
                log::error!("[Internal] Reader error: {}", e);
                return;
            }
        };
        log::trace!("[Server] {}", command_str);
        let command = command_str.as_str().into();
        if let Err(e) = command_sender.send(command) {
            log::error!("[Internal] Reader error: {}", e);
            return;
        }
    }
}

async fn shutdown_writer(
    mut writer: OwnedWriteHalf,
    mut command_receiver: UnboundedReceiver<ClientCommand>,
) {
    command_receiver.close();
    // receive remaining messages
    while let Some(_) = command_receiver.recv().await {}
    if let Err(e) = writer.shutdown().await {
        log::error!("[Internal] Failed to shutdown writer: {}", e);
    }
}

async fn write_client_commands(
    mut writer: OwnedWriteHalf,
    mut command_receiver: UnboundedReceiver<ClientCommand>,
) {
    loop {
        let command = match command_receiver.recv().await {
            Some(command) => command,
            None => {
                log::error!("[Internal] Writer error: channel closed");
                shutdown_writer(writer, command_receiver).await;
                return;
            }
        };

        let command_str: String = (&command).into();
        log::trace!("[Client] {}", command_str.trim());
        if let Err(e) = writer.write_all(&command_str.as_bytes()).await {
            log::error!("[Internal] Writer error: {}", e);
            shutdown_writer(writer, command_receiver).await;
            return;
        }

        // shutdown writer after quit
        if let ClientCommand::Quit(_) = command {
            shutdown_writer(writer, command_receiver).await;
            return;
        }
    }
}

use anyhow::bail;
use anyhow::Result;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::irc::command::{ClientCommand, ServerCommand};
use crate::irc::command::ClientCommand::{Notice, Pong};
use crate::irc::command::ctcp::{CtcpReply, CtcpRequest, CtcpRequestType};
use crate::irc::command::ctcp::dcc::Dcc;
use crate::irc::command::xdcc::Xdcc;
use crate::irc::network::connect;

mod network;
pub mod command;


pub struct IrcDccClient {
    client_command_sender: UnboundedSender<ClientCommand>,
    server_command_receiver: UnboundedReceiver<ServerCommand>,
}

impl IrcDccClient {
    pub async fn connect(server: &str) -> Result<Self> {
        let (client_command_sender, client_command_receiver) = tokio::sync::mpsc::unbounded_channel();
        let (server_command_sender, server_command_receiver) = tokio::sync::mpsc::unbounded_channel();
        connect(server, server_command_sender, client_command_receiver).await?;
        let client = Self {
            client_command_sender,
            server_command_receiver,
        };
        Ok(client)
    }

    async fn wait_for_ping(&mut self) -> Result<()> {
        loop {
            let message = match self.server_command_receiver.recv().await {
                Some(message) => message,
                None => bail!("Cannot receive PING command: channel closed"),
            };
            match message {
                ServerCommand::Ping(content) => {
                    self.client_command_sender.send(Pong(content))?;
                    return Ok(());
                }
                _ => {}
            }
        }
    }

    pub async fn login(&mut self, nickname: String) -> Result<()> {
        self.client_command_sender.send(ClientCommand::Nick(nickname.clone()))?;
        self.client_command_sender.send(ClientCommand::User(nickname))?;

        // Login is successful, once we get the first PING
        self.wait_for_ping().await
    }

    async fn wait_for_ctcp_version(&mut self) -> Result<()> {
        loop {
            let message = match self.server_command_receiver.recv().await {
                Some(message) => message,
                None => bail!("Cannot receive CTCP VERSION command: channel closed"),
            };
            if let ServerCommand::PrivMsg(request) = message {
                if let Some(ctcp_request) = CtcpRequest::try_from_request(request) {
                    match ctcp_request.request_type {
                        CtcpRequestType::Version => {
                            if let CtcpReply::Message(reply) = ctcp_request.generate_reply() {
                                self.client_command_sender.send(Notice(reply))?;
                                return Ok(());
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    pub async fn join(&mut self, channel: String) -> Result<()> {
        self.client_command_sender.send(ClientCommand::Join(channel))?;

        // Join is successful, once we get the first CTCP message (VERSION)
        self.wait_for_ctcp_version().await
    }

    pub fn send_dcc_request(&mut self, request: Xdcc) -> Result<()> {
        let command = request.into();
        self.client_command_sender.send(ClientCommand::PrivMsg(command))?;
        Ok(())
    }

    pub async fn quit(&mut self) -> Result<()> {
        self.client_command_sender.send(ClientCommand::Quit("Goodbye".to_string()))?;
        self.server_command_receiver.close();
        // receive remaining server commands
        while let Some(_) = self.server_command_receiver.recv().await {}
        Ok(())
    }

    pub async fn wait_for_dcc(&mut self) -> Result<Option<Dcc>> {
        loop {
            let message = match self.server_command_receiver.recv().await {
                Some(message) => message,
                None => return Ok(None),
            };
            match message {
                ServerCommand::Ping(content) => self.client_command_sender.send(Pong(content))?,
                ServerCommand::PrivMsg(message) => {
                    if let Some(ctcp_reply) = CtcpRequest::try_from_request(message)
                        .map(|request| request.generate_reply()) {
                        match ctcp_reply {
                            CtcpReply::Dcc(dcc) => return Ok(Some(dcc)),
                            CtcpReply::Message(reply) => {
                                self.client_command_sender.send(Notice(reply))?
                            }
                        }
                    }
                }
                _ => {}
            };
        }
    }
}

use irc_proto::types::{Command::*, Message, Source};
use tokio::sync::mpsc;
use std::collections::HashMap;

use crate::user::User;


pub type MessageReceiverMap = HashMap<String, mpsc::Sender<Message>>;

#[derive(Debug)]
pub enum OperMsg {
    AddChannel{name: String, channel: mpsc::Sender<Message>},
    DeleteChannel{name: String},
}

#[derive(Debug)]
pub struct CommMsg {
    pub user: User,
    pub msg: Message,
}

pub struct Bridge {
    pub handler_tx_map: MessageReceiverMap,
    pub channels_rx: mpsc::Receiver<OperMsg>,
    pub bridge_rx: mpsc::Receiver<CommMsg>,
}

impl Bridge {
    pub async fn run(&mut self) -> Result<(), ()> {
        loop {
            tokio::select! {
                bridge_msg_opt = self.channels_rx.recv() => {
                    if let Some(bridge_msg) = bridge_msg_opt {
                        match bridge_msg {
                            OperMsg::AddChannel{name, channel} => {
                                self.handler_tx_map.insert(name, channel);
                            },
                            OperMsg::DeleteChannel{name} => {
                                self.handler_tx_map.remove(&name);
                            },
                        }
                    }
                }

                message_opt = self.bridge_rx.recv() => {
                    if let Some(message) = message_opt {
                        match &message.msg.command {
                            PRIVMSG { targets, text } => {
                                for target in targets.split(',') {
                                    for (username, chan) in self.handler_tx_map.iter() {
                                        if username == target {
                                            let _ = chan.send(Message::new(
                                                None,
                                                Some(Source{name: message.user.nickname.clone(), user: None, host: None}),
                                                PRIVMSG {
                                                    targets: targets.clone(),
                                                    text: text.clone()
                                                }
                                            )).await;
                                        }
                                    }
                                }
                            },

                            _ => {},
                        }
                    }
                }
            }
        }
    }
}

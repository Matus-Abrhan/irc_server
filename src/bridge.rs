use irc_proto::{channel::Channel, types::{Command::{self, *}, Message, Source}};
use tokio::sync::mpsc;
use std::collections::HashMap;

use crate::user::User;


pub type HandlerTxMap = HashMap<String, mpsc::Sender<Message>>;
pub type ChannelMap = HashMap<String, Channel>;

#[derive(Debug)]
pub enum OperMsg {
    AddUser{name: String, channel: mpsc::Sender<Message>},
    DeleteUser{name: String},
    JoinChannel{nickname: String, channel_name: String}
}

#[derive(Debug)]
pub struct CommMsg {
    pub user: User,
    pub msg: Message,
}

pub struct Bridge {
    pub handler_tx_map: HandlerTxMap,
    pub channel_map: ChannelMap,
    pub oper_rx: mpsc::Receiver<OperMsg>,
    pub comm_rx: mpsc::Receiver<CommMsg>,
}

impl Bridge {
    pub fn new(oper_rx: mpsc::Receiver<OperMsg>, comm_rx: mpsc::Receiver<CommMsg>) -> Self {
        return Bridge { handler_tx_map: HashMap::new(), channel_map: HashMap::new(), oper_rx, comm_rx }
    }

    pub async fn run(&mut self) -> Result<(), ()> {
        loop {
            tokio::select! {
                bridge_msg_opt = self.oper_rx.recv() => {
                    if let Some(bridge_msg) = bridge_msg_opt {
                        match bridge_msg {
                            OperMsg::AddUser{name, channel} => {
                                self.handler_tx_map.insert(name, channel);
                            },
                            OperMsg::DeleteUser{name} => {
                                self.handler_tx_map.remove(&name);
                            },
                            OperMsg::JoinChannel{nickname, channel_name} => {
                                match self.channel_map.get_mut(&channel_name) {
                                    Some(channel) => {
                                        channel.members.push(nickname.clone());
                                    },
                                    None => {
                                        self.channel_map.insert(channel_name.clone(),
                                            Channel::new(channel_name.clone(), nickname.clone())
                                        );
                                    },
                                };

                                // TODO: MOTD
                                // TODO: List of users

                                match self.handler_tx_map.get(&nickname) {
                                    Some(handler_tx) => {
                                        let _ = handler_tx.send(Message::new(
                                            None,
                                            Some(Source{name: nickname, user: None, host: None}),
                                            Command::JOIN{
                                                channels: channel_name.clone(),
                                                keys: None,
                                            }
                                        )).await;
                                    },
                                    None => {},
                                };
                            },
                        }
                    }
                }

                message_opt = self.comm_rx.recv() => {
                    if let Some(message) = message_opt {
                        match &message.msg.command {
                            PRIVMSG { targets, text } => {
                                for target in targets.split(',') {
                                    match self.handler_tx_map.get(target) {
                                        Some(handler_tx) => {
                                            let _ = handler_tx.send(Message::new(
                                                None,
                                                Some(Source{name: message.user.nickname.clone(), user: None, host: None}),
                                                PRIVMSG{
                                                    targets: target.to_string(),
                                                    text: text.clone()
                                                }
                                            )).await;
                                        }
                                        None => {
                                            match self.channel_map.get(target) {
                                                Some(channel) => {
                                                    for member in channel.members.iter() {
                                                        match self.handler_tx_map.get(member) {
                                                            Some(handler_tx) => {
                                                                let _ = handler_tx.send(Message::new(
                                                                    None,
                                                                    Some(Source{name: message.user.nickname.clone(), user: None, host: None}),
                                                                    PRIVMSG{
                                                                        targets: target.to_string(),
                                                                        text: text.clone()
                                                                    }
                                                                )).await;
                                                            },
                                                            None => {},
                                                        }
                                                    }
                                                },
                                                None => {},
                                            }
                                        },
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

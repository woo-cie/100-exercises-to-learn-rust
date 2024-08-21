// TODO: Convert the implementation to use bounded channels.
use crate::data::{Ticket, TicketDraft};
use crate::store::{TicketId, TicketStore};
use std::sync::mpsc::{Receiver, RecvError, SendError, Sender, SyncSender};

pub mod data;
pub mod store;

#[derive(thiserror::Error, Debug)]
pub enum TicketStoreClientError {
    #[error("{0}")]
    SendError(#[from] SendError<Command>),
    #[error("{0}")]
    ResponseError(#[from] RecvError),
}

#[derive(Clone)]
pub struct TicketStoreClient {
    sender: SyncSender<Command>,
}

impl TicketStoreClient {
    pub fn insert(&self, draft: TicketDraft) -> Result<TicketId, TicketStoreClientError> {
        let (sender, receiver) = std::sync::mpsc::sync_channel(10);
        self.sender.send(Command::Insert {
            draft,
            response_channel: sender,
        })?;
        let id = receiver.recv()?;
        Ok(id)
    }

    pub fn get(&self, id: TicketId) -> Result<Option<Ticket>, TicketStoreClientError> {
        let (sender, receiver) = std::sync::mpsc::sync_channel(10);
        self.sender.send(Command::Get {
            id,
            response_channel: sender,
        })?;
        let ticket = receiver.recv()?;
        Ok(ticket)
    }
}

pub fn launch(capacity: usize) -> TicketStoreClient {
    let (sender, receiver) = std::sync::mpsc::sync_channel(10);
    std::thread::spawn(move || server(receiver));
    TicketStoreClient { sender }
}

enum Command {
    Insert {
        draft: TicketDraft,
        response_channel: SyncSender<TicketId>,
    },
    Get {
        id: TicketId,
        response_channel: SyncSender<Option<Ticket>>,
    },
}

pub fn server(receiver: Receiver<Command>) {
    let mut store = TicketStore::new();
    loop {
        match receiver.recv() {
            Ok(Command::Insert {
                draft,
                response_channel,
            }) => {
                let id = store.add_ticket(draft);
                let _ = response_channel.send(id);
            }
            Ok(Command::Get {
                id,
                response_channel,
            }) => {
                let ticket = store.get(id);
                let _ = response_channel.send(ticket.cloned());
            }
            Err(_) => {
                // There are no more senders, so we can safely break
                // and shut down the server.
                break;
            }
        }
    }
}

use std::collections::HashMap;
use std::marker::PhantomData;
use std::io::{Read, Write};

use sonr::{Token, Evented};
use sonr::prelude::*;
use tungstenite::handshake::client::ClientHandshake;
use tungstenite::handshake::{HandshakeError, MidHandshake};
use tungstenite::WebSocket;
use tungstenite::client; 

// -----------------------------------------------------------------------------
// 		- WIP -
// -----------------------------------------------------------------------------

pub struct WebsocketConnector<T, S> 
where
    T: AsRef<Stream<S>> + Read + Write,
    S: Read + Write + Evented,
{ 
    _p: PhantomData<S>,
    handshakes: HashMap<Token, ClientHandshake<T>>, 
}

impl<T, S> Reactor for WebsocketConnector<T, S> 
where
    T: AsRef<Stream<S>> + Read + Write,
    S: Read + Write + Evented,
{ 
    type Input = T;
    type Output = WebSocket<T>;

    fn react(&mut self, reaction: Reaction<Self::Input>) -> Reaction<Self::Output> {
        match reaction {
            Reaction::Value(stream) => {
                let token = stream.as_ref().token();
                let res = client(stream);

                match res {
                    Ok(websocket) => return Reaction::Value(websocket),
                    Err(HandshakeError::Interrupted(mid_shake)) => {
                        self.handshakes.insert(token, mid_shake);
                        Reaction::Continue
                    }
                    Err(HandshakeError::Failure(_e)) => {
                        return Reaction::Continue; /* Let the connections drop on error for now */
                    }
                }
            }
            Reaction::Event(event) => {
                if let Some(stream) = self.handshakes.remove(&event.token()) {
                    match stream.handshake() {
                        Ok(stream) => return Reaction::Value(stream),
                        Err(HandshakeError::Interrupted(mid_shake)) => {
                            self.handshakes.insert(event.token(), mid_shake);
                            return Reaction::Continue;
                        }
                        Err(_e) => return Reaction::Continue, /* Let the connections drop on error for now */
                    }
                } else {
                    Reaction::Event(event)
                }
            }
            Reaction::Continue => Reaction::Continue,
        }
    }
}

use std::fmt::Debug;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::marker::PhantomData;

use sonr::net::stream::StreamRef;
use sonr::prelude::*;
use sonr::{Evented, Token};
use tungstenite::client;
use tungstenite::handshake::client::ClientHandshake;
use tungstenite::handshake::{HandshakeError, MidHandshake};
use tungstenite::WebSocket;
use url::Url;

pub struct WebSocketConnector<T>
where
    T: StreamRef + Read + Write,
{
    handshakes: HashMap<Token, MidHandshake<ClientHandshake<T>>>,
}

impl<T> WebSocketConnector<T>
where
    T: StreamRef + Read + Write,
{
    pub fn new() -> Self {
        Self { 
            handshakes: HashMap::new(),
        }
    }
}

impl<T> Reactor for WebSocketConnector<T>
where
    T: StreamRef + Read + Write,
{
    type Input = (String, T); // String = Url
    type Output = WebSocket<T>;

    fn react(&mut self, reaction: Reaction<Self::Input>) -> Reaction<Self::Output> {
        use Reaction::*;
        match reaction {
            Value((url, stream)) => {
                let token = stream.token();
                let url = Url::parse(&url).unwrap();
                let res = client(url, stream);

                match res {
                    Ok((websocket, response)) => Value(websocket),
                    Err(HandshakeError::Interrupted(mid_shake)) => {
                        self.handshakes.insert(token, mid_shake);
                        Continue
                    }
                    Err(HandshakeError::Failure(_e)) => {
                        Continue /* Let the connections drop on error for now */
                    }
                }
            }
            Event(event) => {
                if let Some(stream) = self.handshakes.remove(&event.token()) {
                    let res = stream.handshake();
                    match res {
                        Ok((stream, response)) => Value(stream),
                        Err(HandshakeError::Interrupted(mid_shake)) => {
                            self.handshakes.insert(event.token(), mid_shake);
                            Continue
                        }
                        Err(_e) => Continue, /* Let the connections drop on error for now */
                    }
                } else {
                    event.into()
                }
            }
            Continue => Continue,
        }
    }
}

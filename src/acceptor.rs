use std::collections::HashMap;
use std::marker::PhantomData;
use std::io::{Read, Write};

use tungstenite::server::accept_hdr_with_config;
use tungstenite::handshake::server::{Callback, NoCallback, ServerHandshake};
use tungstenite::handshake::{HandshakeError, MidHandshake};
use tungstenite::WebSocket;

use sonr::net::stream::Stream;
use sonr::reactor::{Reaction, Reactor};
use sonr::{Evented, Token};

pub struct WebSocketAcceptor<T, S, C>
where
    T: AsRef<Stream<S>> + Read + Write,
    S: Read + Write + Evented,
    C: Callback,
{
    _p1: PhantomData<S>,
    _p2: PhantomData<C>,
    handshakes: HashMap<Token, MidHandshake<ServerHandshake<T, NoCallback>>>,
}

impl<T, S, C> WebSocketAcceptor<T, S, C>
where
    T: AsRef<Stream<S>> + Read + Write,
    S: Read + Write + Evented,
    C: Callback,
{
    pub fn new() -> Self {
        Self {
            _p1: PhantomData,
            _p2: PhantomData,
            handshakes: HashMap::new(),
        }
    }
}

impl<T, S, C> Reactor for WebSocketAcceptor<T, S, C>
where
    T: AsRef<Stream<S>> + Read + Write,
    S: Read + Write + Evented,
    C: Callback,
{
    type Input = T;
    type Output = WebSocket<T>;

    fn react(&mut self, reaction: Reaction<Self::Input>) -> Reaction<Self::Output> {
        match reaction {
            Reaction::Value(stream) => {
                let token = stream.as_ref().token();
                let res = accept_hdr_with_config(stream, NoCallback, None);

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


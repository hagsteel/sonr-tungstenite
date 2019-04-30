use std::collections::HashMap;
use std::env;
use std::io::{Read, Write};
use std::marker::PhantomData;

use sonr::net::tcp::ReactiveTcpListener;
use sonr::net::tcp::TcpStream;
use sonr::prelude::*;
use sonr::reactor::Reactor;
use sonr::*;
use sonr_tls::TlsAcceptor;
use sonr_tungstenite::WebSocketAcceptor;

use tungstenite::handshake::server::NoCallback;
use tungstenite::{Message, WebSocket};

struct Connections<T, S>
where
    T: AsRef<Stream<S>> + Read + Write,
    S: Evented + Read + Write,
{
    c: HashMap<Token, WebSocket<T>>,
    p: PhantomData<S>,
    m: Vec<String>,
}

impl<T, S> Connections<T, S>
where
    T: AsRef<Stream<S>> + Read + Write,
    S: Evented + Read + Write,
{
    pub fn new() -> Self {
        Self {
            c: HashMap::new(),
            p: PhantomData,
            m: Vec::new(),
        }
    }
}

impl<T, S> Reactor for Connections<T, S>
where
    T: AsRef<Stream<S>> + Read + Write,
    S: Evented + Read + Write,
{
    type Input = WebSocket<T>;
    type Output = ();

    fn react(&mut self, reaction: Reaction<Self::Input>) -> Reaction<()> {
        match reaction {
            Reaction::Value(websocket) => {
                self.c
                    .insert(websocket.get_ref().as_ref().token(), websocket);
                Reaction::Continue
            }
            Reaction::Event(ev) => {
                if let Some(socket) = self.c.get_mut(&ev.token()) {
                    match socket.read_message() {
                        Ok(msg) => {
                            if let Message::Text(m) = msg {
                                self.m.push(m);
                            }
                        }
                        Err(_) => {}
                    }

                    while let Some(m) = self.m.pop() {
                        socket.write_message(m.into());
                    }

                    Reaction::Continue
                } else {
                    ev.into()
                }
            }
            _ => Reaction::Continue,
        }
    }
}

fn main() {
    System::init();
    let listener = ReactiveTcpListener::bind("0.0.0.0:5555")
        .unwrap()
        .map(|(s, _)| Stream::new(s).unwrap());

    let path = env::var("SONR_PFX_PATH").unwrap();
    let pass = env::var("SONR_PFX_PASS").unwrap();
    let acceptor = TlsAcceptor::<TcpStream>::new(&path, &pass).unwrap();
    let connections = Connections::new();

    let ws_accept = WebSocketAcceptor::<_, _, NoCallback>::new().map(|s| {
        eprintln!("connected...");
        s
    });

    let ws = ws_accept.chain(connections);
    let server = listener.chain(acceptor.chain(ws));
    //let server = listener.chain(ws);
    System::start(server);
}

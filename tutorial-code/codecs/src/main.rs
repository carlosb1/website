use bytes::BytesMut;
use futures::sink::SinkExt;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::io;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::runtime::Runtime;
use tokio_util::codec::{Decoder, Encoder};
use tokio_util::codec::{FramedRead, FramedWrite};

/// JSON serialized message.
#[derive(Serialize, Deserialize)]
pub struct Message {
    text: String,
}

impl Message {
    /// Static factory for response messages.
    fn new_ok() -> Message {
        Message {
            text: "ok".to_string(),
        }
    }
}

/// Class for encoding and decoding messages.
pub struct MyBytesCodec;

impl Decoder for MyBytesCodec {
    type Item = Message;
    type Error = io::Error;

    /// Decode messages from buf to a Message type
    fn decode(&mut self, buf: &mut BytesMut) -> io::Result<Option<Self::Item>> {
        if buf.len() == 0 {
            return Ok(None);
        }
        let data = buf.clone().to_vec();
        let str_data = String::from_utf8(data).unwrap();
        if let Ok(message) = serde_json::from_str(str_data.as_str()) {
            buf.clear();
            Ok(message)
        } else {
            buf.clear();
            Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Message was  not well decoded",
            ))
        }
    }
}

impl Encoder<Message> for MyBytesCodec {
    type Error = io::Error;

    /// Encode a Message type and append in a BytesMut buffer.
    fn encode(&mut self, data: Message, buf: &mut BytesMut) -> io::Result<()> {
        //It can be used to_vec for a direct parser.
        if let Ok(vec_data) = serde_json::to_string(&data) {
            buf.extend(vec_data.as_bytes());
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Message was  not well Encoded",
            ))
        }
    }
}

/// Server example to listen messages and response
pub struct Server;

impl Server {
    /// Main function for listening new messages and reply.
    pub async fn run(self, address: &str) -> Result<(), Box<dyn Error>> {
        println!("Server: Starting to listen {}", address);
        let addr = address.parse::<SocketAddr>()?;

        let listener = TcpListener::bind(&addr).await?;
        loop {
            println!("Server: listening connection");
            let (mut socket, _) = listener.accept().await?;
            let (r, w) = socket.split();
            // Set up frameds to read and write messages.
            let mut framed_writer = FramedWrite::new(w, MyBytesCodec {});
            let mut framed_reader = FramedRead::new(r, MyBytesCodec {});
            if let Some(frame) = framed_reader.next().await {
                match frame {
                    Ok(response) => {
                        let str_msg = serde_json::to_string(&response)
                            .expect("This message was decoded but now it can be parser to string.");
                        println!(
                            "Server: it is a response message and replying response {}",
                            str_msg
                        );
                        framed_writer.send(Message::new_ok()).await?;
                    }
                    Err(e) => {
                        println!("Server: Error reading response  {}?", e);
                    }
                }
            } else {
                println!("Server: It was not possible to receive responses.");
            }
        }
    }
}

/// Function to send messages and wait response.
pub async fn send(address: &str, mesg: &str) -> Result<(), Box<dyn Error>> {
    let addr = address.parse::<SocketAddr>()?;
    let mut tcp = TcpStream::connect(&addr).await?;
    let (r, w) = tcp.split();

    let mut framed_writer = FramedWrite::new(w, MyBytesCodec {});
    let mut framed_reader = FramedRead::new(r, MyBytesCodec {});

    // Send a new message via json format
    framed_writer
        .send(Message {
            text: mesg.to_string(),
        })
        .await?;

    if let Some(frame) = framed_reader.next().await {
        match frame {
            Ok(response) => {
                let str_msg = serde_json::to_string(&response)
                    .expect("This message was decoded but now it can be parser to string.");
                println!("Sender: it is a response  message {}", str_msg);
            }
            Err(e) => {
                println!("Sender: Error reading response  {}?", e);
            }
        }
    } else {
        println!("Sender: It was not possible to receive responses.");
    }
    Ok(())
}

fn main() -> () {
    let rt = Runtime::new().unwrap();
    let server = Server {};
    rt.block_on(async {
        println!("Starting server...");
        server.run("127.0.0.1:12345").await.unwrap();
    });
}

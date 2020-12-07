use bytes::BytesMut;
use futures::sink::SinkExt;
use futures::StreamExt;
use std::error::Error;
use std::io;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::runtime::Runtime;
use tokio_util::codec::{Decoder, Encoder};
use tokio_util::codec::{FramedRead, FramedWrite};

pub struct MyBytesCodec;

impl Decoder for MyBytesCodec {
    type Item = Vec<u8>;
    type Error = io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> io::Result<Option<Self::Item>> {
        if buf.len() == 0 {
            return Ok(None);
        }
        let data = buf.clone().to_vec();
        buf.clear();
        Ok(Some(data))
    }
}

impl Encoder<Vec<u8>> for MyBytesCodec {
    type Error = io::Error;

    fn encode(&mut self, data: Vec<u8>, buf: &mut BytesMut) -> io::Result<()> {
        buf.extend(data);
        Ok(())
    }
}

pub struct Server;

impl Server {
    pub async fn run(self, address: &str) -> Result<(), Box<dyn Error>> {
        println!("Trying to connect {}", address);
        let addr = address.parse::<SocketAddr>()?;

        let listener = TcpListener::bind(&addr).await?;
        let (mut socket, _) = listener.accept().await?;
        loop {
            let (r, w) = socket.split();
            let mut framed_writer = FramedWrite::new(w, MyBytesCodec {});
            let mut framed_reader = FramedRead::new(r, MyBytesCodec {});
            if let Some(frame) = framed_reader.next().await {
                match frame {
                    Ok(response) => {
                        let mesg = "{'result': 'ok'}";
                        println!("it is a correct message");
                        framed_writer.send(mesg.as_bytes().to_vec()).await?;
                    }
                    Err(e) => {
                        println!("Error reading response  {}?", e);
                    }
                }
            } else {
                println!("It was not possible to receive responses.");
            }
        }
    }
}

pub async fn send(address: &str, mesg: &str) -> Result<(), Box<dyn Error>> {
    let addr = address.parse::<SocketAddr>()?;
    let mut tcp = TcpStream::connect(&addr).await?;
    let (r, w) = tcp.split();

    let mut framed_writer = FramedWrite::new(w, MyBytesCodec {});
    let mut framed_reader = FramedRead::new(r, MyBytesCodec {});

    // Extract message value
    let encoded: Vec<u8> = mesg.as_bytes().to_vec();
    framed_writer.send(encoded).await?;

    if let Some(frame) = framed_reader.next().await {
        match frame {
            Ok(response) => {
                println!("it is a correct message");
                //Ok(())
            }
            Err(e) => {
                println!("Error reading response  {}?", e);
            }
        }
    } else {
        println!("It was not possible to receive responses.");
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting server...");
    let rt = Runtime::new().unwrap();
    let server = Server {};
    rt.block_on(server.run("127.0.0.1:12345"))
}

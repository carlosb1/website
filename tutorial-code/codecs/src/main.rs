use bytes::BytesMut;
use std::error::Error;
use std::io;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio_util::codec::{BytesCodec, FramedRead, FramedWrite};
use tokio_util::codec::{Decoder, Encoder};

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
    pub async fn run(self, address: String) -> Result<(), Box<dyn Error>> {
        println!("Trying to connect {}", address);
        let addr = address.as_str().parse::<SocketAddr>()?;

        let mut listener = TcpListener::bind(&addr).await?;
        let (mut socket, _) = listener.accept().await?;
        loop {
            let (r, w) = socket.split();
            let mut framed_writer = FramedWrite::new(w, MyBytesCodec {});
            let mut framed_reader = FramedRead::new(r, MyBytesCodec {});
        }
    }
}

fn main() {
    println!("Hello world");
}

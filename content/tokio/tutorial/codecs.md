---
title: "Codecs"
---

We will get started by writing a simple JSON protocol with its server and client. The server will be listening
and repling `ok` messages for well-formed messages.


For the example, it is needed: 

- `Encoder` and `Decoder` traits translate messages to send or receive. 
- `FramedWrite` and `FramedRead` traits responsible to apply encoder and decoders in the transmitted data.
-  Serializable class that will be the message. In our case, we will use `serde-json` to create a json protocol.

# Serializable message

To serialize objects, in our case, we will use `serde` and `serde-json` to create a message. It permits an easy debugging.
 
```rust
use serde::{Deserialize, Serialize};

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
```

Now, we will implement the necessary traits to process and stream messages. For this, we need our `decoder` and `encoder`

# Encoder and decoder


The Decoder will reponsible to translate a set of bytes as `BytesMut` in a Serialized messages. It controls different situation
like a not correct decoding. 

```rust
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
```


For encoding, it is similar but it is used the `Encoder` trait. In this case, It parses a type `Message` to a buffer type `BytesMut`.


```rust
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
```
# FramedWrite and FramedRead

To operate with this encoders/decoders it can be used with framed that it will permit an easy way to write and read messages

```rust
        let mut framed_writer = FramedWrite::new(w, MyBytesCodec {});
        let mut framed_reader = FramedRead::new(r, MyBytesCodec {});
```


# Server example

Here, it is an example about how it could be a server that reads and writes messages.

```rust
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
```



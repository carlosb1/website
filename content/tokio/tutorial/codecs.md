---
title: "Codecs"
---

We will get started by writing a simple JSON protocol with its server and client. The server will be listening
and replying `ok` messages for well-formed messages.


For this example, it is needed: 

- `Encoder` and `Decoder` traits translate messages to send or receive one serializable object. 
- `FramedWrite` and `FramedRead` traits responsible to apply encoder and decoders in the transmitted data.
-  Serializable object that will be the message. In our case, we will use `serde-json` to create a json protocol.

# Serializable message

For serialized objects, in our case, we will use `serde` and `serde-json` to create a message that it permits an easy debugging. It can be something like this:
 
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

The job fo the decoder is to look into a buffer of data and determinate if the buffer contains enough data to produce a full item in the stream (in our case,  our item is `Message`).

If the buffer does not contain enough data to produce an item, it returns `Ok(None)` and leaves the buffer unmodified. In this case, the containing `FramedRead` will read more data from the underlying `AsyncRead`, append the bytes to the buffer, and call decode again later, hopefully with enough data to produce an item.


When the `decoder` returns `Ok(None)`, and you know how long the next item is going to be, it's a good idea to call reserve on the buffer such that the FramedRead will ask for that much data on the next call to the IO resource. That said, FramedRead calls the decoder every time there is more data available — it wont wait until the buffer is complete filled.

If the `decoder` returns an error, the stream is terminated.


In the case of the `encoder`, it is the opposite, its job is to encode a full item and fill the buffer, this item must be serialized and converted in bytes, then you can append it in the buffer. If it works well, you returns `Ok(())` in the other case, It should be returned an error to finish the encoding.


The `FramedWrite` will write this data from the `AsyncWrite` and it is called every time that we have a new item to encode.





# Encoder and decoder


The Decoder extracts a serialized item from the buffer `BytesMut`. Our example:

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
Where we try to deserialize via `serde-json` and it returns a correct  `Message` item, for any problem it will return an error. 


For the encoding step, the process is similar, It tries to serialize  the item via `serde-json` and converts to bytes. These bytes extend the buffer.


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

To operate with this encoders/decoders as it was explained above, it is necessary to use `FramedWrite` and `FrameRead` that they will use our codecs:

```rust
        let mut framed_writer = FramedWrite::new(w, MyBytesCodec {});
        let mut framed_reader = FramedRead::new(r, MyBytesCodec {});
```


# Server example

Getting back our example and with the  implementation of our encoder and decoder, the server could be something like this:

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


# Client example

In the case of the client, you need an function to send and wait a response.

```rust
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
```

It can be called like this:

```rust
fn main() -> () {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        println!("Sending message to the server");
        send("127.0.0.1:12345", "Hello my friend")
            .await
            .expect("Error sending message");
    });
}
```
# Reading messages from  StreamReader

In our example server, it is used the `TCPListener` to stream data but also `StreamReader` or `ReaderStream` can do this, they permit stream data in an easy way.

In this example, we are decoding messages that they come from `StreamReader`.


```rust
    use bytes::Bytes;
    use tokio_util::io::StreamReader;
    let stream = tokio::stream::iter(vec![tokio::io::Result::Ok(Bytes::from(
        "{\"text\":\"hello world\"}",
    ))]);
    let framed_stream = StreamReader::new(stream);
    let mut framed_reader = FramedRead::new(framed_stream, MyBytesCodec {});

    let rt = Runtime::new().unwrap();
    rt.block_on(async {
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
    });
```


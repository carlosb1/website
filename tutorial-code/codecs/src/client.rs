mod main;
use main::send;
use tokio::runtime::Runtime;

fn main() -> () {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        println!("Sending message to the server");
        send("127.0.0.1:12345", "Hello my friend")
            .await
            .expect("Error sending message");
    });
}

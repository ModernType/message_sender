use std::{collections::HashMap, io::Write, net::TcpStream};

#[test]
fn send_test_message() {
    // This constant holds an address to send message to `message_sender`
    const MESSAGE_SEND_ADDR: &str = "127.0.0.1:8000";

    let mut stream = TcpStream::connect(MESSAGE_SEND_ADDR).unwrap();
    let mut map = HashMap::new();
    map.insert("text", "Test message to send");
    let json = serde_json::to_string(&map).unwrap();
    stream.write_all(json.as_bytes()).unwrap();
}

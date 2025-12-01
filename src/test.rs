use reqwest::Method;

use crate::message::OperatorMessage;

#[test]
fn send_test_message() {
    // This constant holds an address to send message to `message_sender`
    const MESSAGE_SEND_ADDR: &str = "http://127.0.0.1:8000";
    const MESSAGE_BODY: &[u8] = include_bytes!("../test.json");

    let client = reqwest::blocking::Client::new();
    client
        .request(Method::POST, MESSAGE_SEND_ADDR)
        .body(MESSAGE_BODY)
        .send()
        .unwrap();
}

#[test]
fn deserialize_messages() {
    const MESSAGES: &str = include_str!("../test.json");

    let messages: Vec<OperatorMessage> = serde_json::from_str(MESSAGES).unwrap();
    for m in messages {
        println!("{m}");
    }
}

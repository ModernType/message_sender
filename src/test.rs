use reqwest::Method;

#[test]
fn send_test_message() {
    // This constant holds an address to send message to `message_sender`
    const MESSAGE_SEND_ADDR: &str = "http://127.0.0.1:8000";
    const MESSAGE_BODY: &[u8] = include_bytes!("../test.json");

    let client = reqwest::blocking::Client::new();
    client.request(Method::POST, MESSAGE_SEND_ADDR)
    .body(MESSAGE_BODY)
    .send()
    .unwrap();
}

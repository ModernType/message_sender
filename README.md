# Message Sender Rebirth

## Features
- [x] Link to main device with QR-code
- [x] Sync with secondary device
- [x] Edit and send messages to selected groups
- [x] Accept messages sent to the app (via ~~`TcpListener`~~ `axum`)
- [ ] Rich text support (Half baked)
- [ ] Full Op support
- [ ] WhatsApp?

## Installing
Run the executable and you are good to go 👍

## Building
For compilation you need [rust toolchain](https://rust-lang.org) to be installed.

Clone the project using `git`:

```git clone https://github.com/ModernType/message_sender.git```

Then in the project folder build it with `cargo`:

```cargo build --release```

Executable will be located in: `{project_folder}/target/release/message-sender`

Additionally you can compile the app with [skia renderer](https://skia.org/) using `skia` feature:

```cargo build --release --features skia```

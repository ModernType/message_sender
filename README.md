# Modern Sender

## Features
- [x] Link to main device with QR-code
- [x] Sync with secondary device
- [x] Edit and send messages to selected groups
- [x] Accept messages sent to the app
- [x] Rich text support (Markdown)
- [x] Sent messages history
- [x] WhatsApp
- [x] Send categories
- [ ] Send to contacts
- [ ] File sending

## Installing
Run the executable and you are good to go 👍

## Building
For compilation you need *nightly* [rust toolchain](https://rust-lang.org) to be installed.
Also you need to install [Protobuf compiler](https://uk.wikipedia.org/wiki/Protocol_Buffers) using `apt`:
```
sudo apt install protobuf-compiler
```
Debug build is set up to use [`cranelift`](https://cranelift.dev/) compiler backend. To install it use `rustup`:
```
rustup component add rustc-codegen-cranelift-preview --toolchain nightly
```

Clone the project using `git`:

```git clone https://github.com/ModernType/message_sender.git```

Then in the project folder build it with `cargo`:

```cargo release```

Executable will be located in: `{project_folder}/target/release/modern-sender`


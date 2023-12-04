# tokio-tungstenite-tls-example

Example for WebSocket server over TLS.

## Usage

You can do this with the command below.

The server then opens a WebSocket echo server listening on the 8080.

```bash
cargo build

./target/debug/tokio-tungstenite-tls-example -i identity_file.p12 -p password
```

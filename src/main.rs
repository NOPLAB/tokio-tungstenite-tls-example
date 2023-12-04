use anyhow::Context;
use clap::{Arg, Command};
use futures_util::{StreamExt, TryStreamExt};
use log::info;
use std::{
    fs::File,
    future,
    io::{Read, Write},
};
use tokio::net::{TcpListener, TcpStream};
use tokio_native_tls::{
    native_tls::{self, Identity},
    TlsStream,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Hello, world!");

    let app = Command::new("broadcast")
        .version("0.1.0")
        .author("nop")
        .about("WebRTC")
        .subcommand_negates_reqs(true)
        .arg(
            Arg::new("debug")
                .long("debug")
                .short('d')
                .help("Prints debug log")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("addr")
                .long("addr")
                .short('a')
                .help("Address to listen")
                .default_value("127.0.0.1:8080"),
        )
        .arg(
            Arg::new("identity")
                .required(true)
                .long("identity")
                .short('i')
                .help("Identity file"),
        )
        .arg(Arg::new("identity_password").long("identity-password").short('p').help(
            "Identity password. If not specified, it will be prompted to enter the password.",
        ));

    let matches = app.clone().get_matches();

    if matches.get_flag("debug") {
        println!("Debug mode");
        env_logger::Builder::new()
            .format(|buf, record| {
                writeln!(
                    buf,
                    "{}:{} [{}] {} - {}",
                    record.file().unwrap_or("unknown"),
                    record.line().unwrap_or(0),
                    record.level(),
                    chrono::Local::now().format("%H:%M:%S.%6f"),
                    record.args()
                )
            })
            .filter(None, log::LevelFilter::Trace)
            .init();
    }

    let addr = matches
        .get_one::<String>("addr")
        .context("Failed to get addr")?;

    let try_socket = TcpListener::bind(&addr).await;
    let listener = try_socket.expect("Failed to bind");
    info!("Listening on: {}", addr);

    let identity_file = matches
        .get_one::<String>("identity")
        .context("Failed to get identity file")?;

    let identity_password = match matches.get_one::<String>("identity_password") {
        Some(password) => password.clone(),
        None => rpassword::prompt_password("Enter identity password: ")
            .expect("Failed to read password"),
    };

    let mut file = File::open(identity_file).unwrap();
    let mut identity = vec![];
    file.read_to_end(&mut identity).unwrap();
    let identity = Identity::from_pkcs12(&identity, &identity_password).unwrap();
    let tls_acceptor =
        tokio_native_tls::TlsAcceptor::from(native_tls::TlsAcceptor::builder(identity).build()?);

    while let Ok((stream, _)) = listener.accept().await {
        let tls_acceptor = tls_acceptor.clone();

        tokio::spawn(async move {
            let stream = tls_acceptor.accept(stream).await.expect("tls accept error");
            accept_connection(stream).await;
        });
    }

    Ok(())
}

async fn accept_connection(stream: TlsStream<TcpStream>) {
    let ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .expect("Error during the websocket handshake occurred");

    info!("New WebSocket connection");

    let (write, read) = ws_stream.split();
    // We should not forward messages other than text or binary.
    read.try_filter(|msg| future::ready(msg.is_text() || msg.is_binary()))
        .forward(write)
        .await
        .expect("Failed to forward messages")
}

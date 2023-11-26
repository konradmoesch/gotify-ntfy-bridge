use ntfy::{Auth, Dispatcher, Payload, Priority};
use tokio::{io::BufStream, net::TcpListener};
use tokio::io::AsyncWriteExt;
use tracing::info;

mod req;

static DEFAULT_PORT: &str = "8080";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize the default tracing subscriber.
    tracing_subscriber::fmt::init();

    let port: u16 = std::env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_PORT.to_string())
        .parse()?;

    let listener = TcpListener::bind(format!("0.0.0.0:{port}")).await.unwrap();

    info!("listening on: {}", listener.local_addr()?);

    loop {
        let (stream, addr) = listener.accept().await?;
        let mut stream = BufStream::new(stream);

        let dispatcher = Dispatcher::builder("https://ntfy.kmoes.ch")
            .credentials(Auth::new("bridge", "test"))
            .build()?;

        // do not block the main thread, spawn a new task
        tokio::spawn(async move {
            info!(?addr, "new connection");

            match req::parse_request(&mut stream).await {
                Ok(req) => {
                    info!(?req, "incoming request");
                    let token = req.headers.get("Authorization").unwrap().split_whitespace().last().unwrap();
                    info!(?token, "token");

                    let raw_body = req.body.unwrap();
                    let body:serde_json::Value = serde_json::from_str(raw_body.as_str()).unwrap();
                    let message = body["message"].as_str().unwrap();
                    let title = body["title"].as_str().unwrap();

                    dbg!(body.clone());

                    let payload = Payload::new("pve")
                        .message(message)
                        .title(title);
                    dispatcher.send(&payload).await.unwrap();

                    let mut response = "HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n";
                    let res_status = stream.write(response.as_bytes()).await.unwrap();
                    dbg!(res_status);
                },
                Err(e) => {
                    info!(?e, "failed to parse request");
                }
            }
        });
    }
}


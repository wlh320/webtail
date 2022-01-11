use askama::Template;
use axum::{
    body::{self, Full},
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::Extension,
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::get,
    AddExtensionLayer, Router,
};
use clap::Parser;
use linemux::MuxedLines;
use std::{net::SocketAddr, sync::Arc};
use tokio::io::{self, AsyncReadExt};
use tokio::{fs::File, io::AsyncSeekExt};
use tower_http::auth::RequireAuthorizationLayer;

struct State {
    filepath: String,
}

/// Make "tail -f" as a web service
#[derive(Parser, Debug)]
#[clap(about, version, author)]
struct Args {
    /// Path of log file
    #[clap(short, long)]
    filepath: String,

    /// Username of basic auth
    #[clap(long, default_value_t = String::from("webtail"))]
    username: String,

    /// Password of basic auth
    #[clap(long, default_value_t = String::from("webtail"))]
    passwd: String,

    /// TCP port to bind
    #[clap(long, default_value_t = 3000)]
    port: u16,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let (filepath, port, username, passwd) = (args.filepath, args.port, args.username, args.passwd);
    let shared_state = Arc::new(State { filepath });
    // build axum app
    let app = Router::new()
        .route("/", get(index_handler))
        .route("/ws", get(ws_handler))
        .layer(AddExtensionLayer::new(shared_state))
        .layer(RequireAuthorizationLayer::basic(&username, &passwd));

    // run it with hyper
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    filename: String,
}

struct HtmlTemplate<T>(T);

impl<T> IntoResponse for HtmlTemplate<T>
where
    T: Template,
{
    fn into_response(self) -> Response {
        match self.0.render() {
            Ok(html) => Html(html).into_response(),
            Err(err) => Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(body::boxed(Full::from(format!(
                    "Failed to render template. Error: {}",
                    err
                ))))
                .unwrap(),
        }
    }
}

async fn index_handler(Extension(state): Extension<Arc<State>>) -> impl IntoResponse {
    let filename = state.filepath.to_string();
    let template = IndexTemplate { filename };
    HtmlTemplate(template)
}

async fn ws_handler(ws: WebSocketUpgrade, state: Extension<Arc<State>>) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, Extension(state): Extension<Arc<State>>) {
    if let Some(Ok(msg)) = socket.recv().await {
        println!("Client says: {:?}", msg);
    } else {
        println!("Client disconnected");
    }
    // read last few lines of log file
    if let Ok(s) = read_last_few_lines(&state.filepath).await {
        if socket.send(Message::Text(s)).await.is_err() {
            println!("send message failed");
        }
    } else {
        println!("read file failed");
        return;
    }
    // read upcoming new lines continuously
    let mut lines = MuxedLines::new().unwrap();
    if let Err(e) = lines.add_file(&state.filepath).await {
        println!("{}", e);
        return;
    }
    while let Ok(Some(line)) = lines.next_line().await {
        socket
            .send(Message::Text(line.line().to_string()))
            .await
            .map_err(|e| {
                println!("send message failed!, error: {:?}", e);
                e
            })
            .ok();
    }
}

async fn read_last_few_lines(filepath: &str) -> io::Result<String> {
    let mut f = File::open(filepath).await?;
    let chunksize = 1024u64;
    let pos = f.metadata().await?.len();
    let pos = if pos > chunksize { pos - chunksize } else { 0 };
    let mut s = String::new();
    f.seek(io::SeekFrom::Start(pos)).await?;
    f.read_to_string(&mut s).await?;
    let res = match s.split_once('\n') {
        None => s,
        Some((_s1, s2)) => s2.to_owned(),
    };
    Ok(res)
}

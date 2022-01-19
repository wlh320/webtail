use axum::{routing::get, AddExtensionLayer, Router};
use clap::Parser;
use std::{net::SocketAddr, sync::Arc};
use tower_http::auth::RequireAuthorizationLayer;

mod index;
mod ws;

pub struct State {
    pub filepath: String,
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
        .route("/", get(index::index_handler))
        .route("/ws", get(ws::ws_handler))
        .layer(AddExtensionLayer::new(shared_state))
        .layer(RequireAuthorizationLayer::basic(&username, &passwd));

    // run it with hyper
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

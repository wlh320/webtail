use crate::State;
use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::Extension,
    response::IntoResponse,
};
use linemux::MuxedLines;
use serde::Serialize;
use std::time::UNIX_EPOCH;
use std::{sync::Arc, time::SystemTime};
use tokio::io::{self, AsyncReadExt};
use tokio::{fs::File, io::AsyncSeekExt};

pub async fn ws_handler(ws: WebSocketUpgrade, state: Extension<Arc<State>>) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

#[derive(Serialize)]
struct TailMsg {
    time: SystemTime,
    text: String,
}

async fn handle_socket(mut socket: WebSocket, Extension(state): Extension<Arc<State>>) {
    if let Some(Ok(msg)) = socket.recv().await {
        println!("Client says: {:?}", msg);
    } else {
        println!("Client disconnected");
    }
    // read last few lines of log file
    if let Ok(msg) = read_last_few_lines(&state.filepath).await {
        let s = serde_json::to_string(&msg).unwrap_or_default();
        if socket.send(Message::Text(s)).await.is_err() {
            println!("send message failed");
        }
    } else {
        eprintln!("read file failed");
        return;
    }
    // read upcoming new lines continuously
    let mut lines = MuxedLines::new().unwrap();
    if let Err(e) = lines.add_file(&state.filepath).await {
        eprintln!("{}", e);
        return;
    }
    while let Ok(Some(line)) = lines.next_line().await {
        let msg = TailMsg {
            time: read_mod_time(&state.filepath).await.unwrap_or(UNIX_EPOCH),
            text: line.line().to_owned(),
        };
        let s = serde_json::to_string(&msg).unwrap_or_default();
        if let Err(e) = socket.send(Message::Text(s)).await {
            eprintln!("send message failed!, error: {:?}", e);
            break;
        }
    }
    // TODO: close websocket correctly
}

async fn read_mod_time(filepath: &str) -> io::Result<SystemTime> {
    File::open(filepath).await?.metadata().await?.modified()
}

async fn read_last_few_lines(filepath: &str) -> io::Result<TailMsg> {
    let mut f = File::open(filepath).await?;
    let chunksize = 1024u64;
    let pos = f.metadata().await?.len();
    let pos = if pos > chunksize { pos - chunksize } else { 0 };
    let mut s = String::new();
    f.seek(io::SeekFrom::Start(pos)).await?;
    f.read_to_string(&mut s).await?;
    let mod_time = f.metadata().await?.modified()?;
    let res = match s.split_once('\n') {
        None => s,
        Some((_s1, s2)) => s2.to_owned(),
    };
    Ok(TailMsg {
        time: mod_time,
        text: res,
    })
}

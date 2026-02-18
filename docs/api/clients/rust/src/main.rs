use anyhow::{Context, Result};
use base64::Engine;
use futures_util::StreamExt;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use tokio_tungstenite::tungstenite::Message;
use url::Url;

#[tokio::main]
async fn main() -> Result<()> {
    let mut ws_url = None;
    let mut token = None;
    let mut session = "active".to_string();
    let mut encoding = "binary".to_string();

    for arg in env::args().skip(1) {
        if let Some(value) = arg.strip_prefix("--ws=") {
            ws_url = Some(value.to_string());
        } else if let Some(value) = arg.strip_prefix("--token=") {
            token = Some(value.to_string());
        } else if let Some(value) = arg.strip_prefix("--session=") {
            session = value.to_string();
        } else if let Some(value) = arg.strip_prefix("--encoding=") {
            encoding = value.to_string();
        }
    }

    let state = read_ws_state()?;
    let ws_url = ws_url.unwrap_or(state.ws_url);
    let mut url = Url::parse(&ws_url).context("invalid ws url")?;
    url.query_pairs_mut()
        .append_pair("session", &session)
        .append_pair("encoding", &encoding);
    if token.is_none() {
        token = state.token;
    }
    if let Some(token) = token.as_deref() {
        url.query_pairs_mut().append_pair("token", token);
    }

    let (mut socket, _) = tokio_tungstenite::connect_async(url.clone())
        .await
        .context("failed to connect")?;
    eprintln!("connected {url}");

    let mut decoder = encoding_rs::UTF_8.new_decoder();

    while let Some(msg) = socket.next().await {
        let msg = msg.context("ws error")?;
        match msg {
            Message::Text(text) => handle_text(&text)?,
            Message::Binary(bytes) => handle_binary(&bytes, &mut decoder)?,
            Message::Close(_) => break,
            _ => {}
        }
    }

    Ok(())
}

struct WsState {
    ws_url: String,
    token: Option<String>,
}

fn read_ws_state() -> Result<WsState> {
    let path = env::var("AGENT_TUI_WS_STATE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            PathBuf::from(home).join(".agent-tui/api.json")
        });

    let contents = fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let value: serde_json::Value = serde_json::from_str(&contents)?;
    Ok(WsState {
        ws_url: value
            .get("ws_url")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        token: value
            .get("token")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
    })
}

fn handle_text(text: &str) -> Result<()> {
    let value: serde_json::Value = serde_json::from_str(text)?;
    let event = value.get("event").and_then(|v| v.as_str()).unwrap_or("");
    match event {
        "init" => {
            if let Some(init) = value.get("init").and_then(|v| v.as_str()) {
                write_out(init.as_bytes())?;
            }
        }
        "output" => {
            if let Some(data) = value.get("data_b64").and_then(|v| v.as_str()) {
                let decoded = base64::engine::general_purpose::STANDARD
                    .decode(data)
                    .context("invalid base64 output")?;
                write_out(&decoded)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn handle_binary(bytes: &[u8], decoder: &mut encoding_rs::Decoder) -> Result<()> {
    if bytes.is_empty() || bytes[0] != 0x01 {
        return Ok(());
    }
    let mut output = String::new();
    let (_, _, _) = decoder.decode_to_string(&bytes[1..], &mut output, false);
    write_out(output.as_bytes())
}

fn write_out(bytes: &[u8]) -> Result<()> {
    let mut stdout = io::stdout();
    stdout.write_all(bytes)?;
    stdout.flush()?;
    Ok(())
}

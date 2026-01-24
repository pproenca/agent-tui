//! Adapters for recording output format conversion.

use serde_json::{Value, json};
use tracing::error;

use crate::session::RecordingFrame;

/// Build asciicast v2 format from recording frames.
///
/// Asciicast is the format used by asciinema for terminal recordings.
/// See: https://github.com/asciinema/asciinema/blob/develop/doc/asciicast-v2.md
pub fn build_asciicast(session_id: &str, cols: u16, rows: u16, frames: &[RecordingFrame]) -> Value {
    let mut output = Vec::new();

    let duration = frames
        .last()
        .map(|f| f.timestamp_ms as f64 / 1000.0)
        .unwrap_or(0.0);

    let header = json!({
        "version": 2,
        "width": cols,
        "height": rows,
        "timestamp": chrono::Utc::now().timestamp(),
        "duration": duration,
        "title": format!("agent-tui recording - {}", session_id),
        "env": {
            "TERM": "xterm-256color",
            "SHELL": std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string())
        }
    });

    match serde_json::to_string(&header) {
        Ok(s) => output.push(s),
        Err(e) => {
            error!(error = %e, "Failed to serialize asciicast header");
            return json!({
                "format": "asciicast",
                "version": 2,
                "error": format!("Failed to serialize recording header: {}", e)
            });
        }
    }

    let mut prev_screen = String::new();
    for frame in frames {
        let time_secs = frame.timestamp_ms as f64 / 1000.0;
        if frame.screen != prev_screen {
            let screen_data = if prev_screen.is_empty() {
                frame.screen.clone()
            } else {
                format!("\x1b[2J\x1b[H{}", frame.screen)
            };
            let event = json!([time_secs, "o", screen_data]);
            match serde_json::to_string(&event) {
                Ok(s) => output.push(s),
                Err(e) => {
                    error!(error = %e, "Failed to serialize asciicast frame");
                }
            }
            prev_screen = frame.screen.clone();
        }
    }

    json!({
        "format": "asciicast",
        "version": 2,
        "data": output.join("\n")
    })
}

/// Build raw frames format from recording frames.
pub fn build_raw_frames(frames: &[RecordingFrame]) -> Value {
    let frame_data: Vec<_> = frames
        .iter()
        .map(|f| {
            json!({
                "timestamp_ms": f.timestamp_ms,
                "screen": f.screen
            })
        })
        .collect();
    json!({ "frames": frame_data, "frame_count": frames.len() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_asciicast_empty_frames() {
        let result = build_asciicast("test-session", 80, 24, &[]);
        assert_eq!(result["format"], "asciicast");
        assert_eq!(result["version"], 2);
        assert!(result["data"].as_str().is_some());
    }

    #[test]
    fn test_build_asciicast_with_frames() {
        let frames = vec![
            RecordingFrame {
                timestamp_ms: 0,
                screen: "Hello".to_string(),
            },
            RecordingFrame {
                timestamp_ms: 1000,
                screen: "World".to_string(),
            },
        ];
        let result = build_asciicast("test-session", 80, 24, &frames);
        assert_eq!(result["format"], "asciicast");
        assert_eq!(result["version"], 2);
        let data = result["data"].as_str().unwrap();
        assert!(data.contains("Hello"));
        assert!(data.contains("World"));
    }

    #[test]
    fn test_build_raw_frames() {
        let frames = vec![RecordingFrame {
            timestamp_ms: 100,
            screen: "test".to_string(),
        }];
        let result = build_raw_frames(&frames);
        assert_eq!(result["frame_count"], 1);
        assert!(result["frames"].is_array());
    }
}

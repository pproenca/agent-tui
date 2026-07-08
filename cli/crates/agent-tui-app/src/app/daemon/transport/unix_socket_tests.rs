use super::*;
use std::io::Cursor;

#[test]
fn test_size_limited_reader_within_limit() {
    let data = "hello\nworld\n";
    let cursor = Cursor::new(data);
    let buf_reader = BufReader::new(cursor);
    let mut reader = SizeLimitedReader::new(buf_reader, 100);

    assert_eq!(
        reader.read_line().expect("first line should read"),
        Some("hello".to_string())
    );
    assert_eq!(
        reader.read_line().expect("second line should read"),
        Some("world".to_string())
    );
    assert_eq!(reader.read_line().expect("reader should reach EOF"), None);
}

#[test]
fn test_size_limited_reader_exceeds_limit() {
    let data = "this is a long line that exceeds the limit\n";
    let cursor = Cursor::new(data);
    let buf_reader = BufReader::new(cursor);
    let mut reader = SizeLimitedReader::new(buf_reader, 10);

    let result = reader.read_line();
    assert!(matches!(result, Err(TransportError::SizeLimit { .. })));
}

#[test]
fn test_size_limited_reader_strips_newlines() {
    let data = "line with crlf\r\n";
    let cursor = Cursor::new(data);
    let buf_reader = BufReader::new(cursor);
    let mut reader = SizeLimitedReader::new(buf_reader, 100);

    assert_eq!(
        reader
            .read_line()
            .expect("CRLF-terminated line should read"),
        Some("line with crlf".to_string())
    );
}

#[test]
fn test_transport_error_display() {
    let io_err = TransportError::Io(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "test error",
    ));
    assert!(io_err.to_string().contains("I/O error"));

    let parse_err = TransportError::Parse {
        source: serde_json::from_str::<serde_json::Value>("invalid json")
            .expect_err("invalid JSON should fail"),
        request_id: None,
    };
    assert!(parse_err.to_string().contains("Parse error"));

    let size_err = TransportError::SizeLimit { max_bytes: 1024 };
    assert!(size_err.to_string().contains("1024"));

    let timeout_err = TransportError::Timeout;
    assert_eq!(timeout_err.to_string(), "Connection timeout");

    let closed_err = TransportError::ConnectionClosed;
    assert_eq!(closed_err.to_string(), "Connection closed");
}

#[test]
fn test_transport_error_from_io() {
    let timeout = std::io::Error::new(std::io::ErrorKind::TimedOut, "timed out");
    assert!(matches!(
        TransportError::from(timeout),
        TransportError::Timeout
    ));

    let would_block = std::io::Error::new(std::io::ErrorKind::WouldBlock, "would block");
    assert!(matches!(
        TransportError::from(would_block),
        TransportError::Timeout
    ));

    let eof = std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "eof");
    assert!(matches!(
        TransportError::from(eof),
        TransportError::ConnectionClosed
    ));

    let broken_pipe = std::io::Error::new(std::io::ErrorKind::BrokenPipe, "broken");
    assert!(matches!(
        TransportError::from(broken_pipe),
        TransportError::ConnectionClosed
    ));

    let other = std::io::Error::other("other");
    assert!(matches!(TransportError::from(other), TransportError::Io(_)));
}

#[test]
fn test_unix_socket_roundtrip() {
    use std::os::unix::net::UnixStream;
    use std::thread;

    let (client_stream, server_stream) =
        UnixStream::pair().expect("unix stream pair should be created");

    let server_handle = thread::spawn(move || {
        let mut conn =
            UnixSocketConnection::new(server_stream).expect("server connection should wrap");
        let request = conn.read_request().expect("server request should read");
        assert_eq!(request.method, "test_method");

        let payload =
            serde_json::from_str(r#"{"ok":true}"#).expect("response payload should parse");
        let response = RpcResponse::success(request.id, payload);
        conn.write_response(&response)
            .expect("server response should write");
    });

    let mut client_stream_writer = client_stream
        .try_clone()
        .expect("client stream should clone");
    let mut client_conn =
        UnixSocketConnection::new(client_stream).expect("client connection should wrap");

    let request_json = r#"{"jsonrpc":"2.0","id":1,"method":"test_method"}"#;
    writeln!(client_stream_writer, "{request_json}").expect("client request should write");

    let response = client_conn.read_request();
    assert!(
        response.is_ok()
            || matches!(
                response,
                Err(TransportError::Parse {
                    source: _,
                    request_id: _
                })
            )
    );

    server_handle.join().expect("server thread should join");
}

#[test]
fn test_unix_socket_parse_error_recovers_request_id() {
    use crate::adapters::rpc::RpcId;
    use std::os::unix::net::UnixStream;

    let (mut client_stream, server_stream) =
        UnixStream::pair().expect("unix stream pair should be created");
    let mut conn = UnixSocketConnection::new(server_stream).expect("server connection should wrap");

    writeln!(
        client_stream,
        "{{\"jsonrpc\":\"2.0\",\"id\":\"request-7\",\"method\":7}}"
    )
    .expect("invalid request should write");

    let result = conn.read_request();
    assert!(matches!(
        result,
        Err(TransportError::Parse {
            request_id: Some(id),
            ..
        }) if id == RpcId::from("request-7")
    ));
}

#[test]
fn test_unix_socket_connection_closed() {
    use std::os::unix::net::UnixStream;

    let (client_stream, server_stream) =
        UnixStream::pair().expect("unix stream pair should be created");
    drop(server_stream);

    let mut conn = UnixSocketConnection::new(client_stream).expect("client connection should wrap");
    let result = conn.read_request();
    assert!(matches!(result, Err(TransportError::ConnectionClosed)));
}

#[test]
fn test_size_limited_reader_applies_limit_per_request_line() {
    let data = "aaa\nbbb\nccc\n";
    let cursor = Cursor::new(data);
    let buf_reader = BufReader::new(cursor);
    let mut reader = SizeLimitedReader::new(buf_reader, 8);

    assert_eq!(
        reader.read_line().expect("first line should read"),
        Some("aaa".to_string())
    );
    assert_eq!(
        reader.read_line().expect("second line should read"),
        Some("bbb".to_string())
    );
    assert_eq!(
        reader.read_line().expect("third line should read"),
        Some("ccc".to_string())
    );
    assert_eq!(reader.read_line().expect("reader should reach EOF"), None);
}

#[test]
fn test_unix_socket_connection_reads_multiple_requests_with_per_request_limit() {
    use std::os::unix::net::UnixStream;

    let (mut client_stream, server_stream) =
        UnixStream::pair().expect("unix stream pair should be created");
    let mut conn = UnixSocketConnection::new_with_max(server_stream, 64)
        .expect("server connection should wrap");

    writeln!(
        client_stream,
        "{{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"one\"}}"
    )
    .expect("first request should write");
    writeln!(
        client_stream,
        "{{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"two\"}}"
    )
    .expect("second request should write");

    let first = conn.read_request().expect("first request should read");
    let second = conn.read_request().expect("second request should read");

    assert_eq!(first.method, "one");
    assert_eq!(second.method, "two");
}

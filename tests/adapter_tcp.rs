use gpui_tetris::adapter::{AdapterConfig, SocketAdapter};
use gpui_tetris::game::state::{GameConfig, GameState};
use serde_json::Value;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::thread;
use std::time::{Duration, Instant};

fn test_adapter() -> (SocketAdapter, GameState, String) {
    test_adapter_with_config(AdapterConfig {
        host: "127.0.0.1".to_string(),
        port: 0,
        idle_timeout_ms: None,
        log_path: None,
        ..AdapterConfig::default()
    })
}

fn test_adapter_with_config(config: AdapterConfig) -> (SocketAdapter, GameState, String) {
    let adapter = SocketAdapter::bind(config).expect("adapter bind");
    let addr = adapter.local_addr().expect("local addr");
    let state = GameState::new(1, GameConfig::default());
    (adapter, state, addr)
}

fn connect(addr: &str) -> TcpStream {
    let stream = TcpStream::connect(addr).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_millis(20)))
        .expect("set timeout");
    stream
}

fn send_json_line(stream: &mut TcpStream, value: Value) {
    let mut bytes = serde_json::to_vec(&value).expect("encode");
    bytes.push(b'\n');
    stream.write_all(&bytes).expect("write line");
}

fn send_raw_line(stream: &mut TcpStream, bytes: &[u8]) {
    stream.write_all(bytes).expect("write bytes");
    stream.write_all(b"\n").expect("write newline");
}

fn try_read_json_line(stream: &mut TcpStream) -> Option<Value> {
    let mut buf = Vec::new();
    loop {
        let mut byte = [0_u8; 1];
        match stream.read(&mut byte) {
            Ok(0) => return None,
            Ok(_) => {
                if byte[0] == b'\n' {
                    break;
                }
                buf.push(byte[0]);
            }
            Err(err)
                if err.kind() == std::io::ErrorKind::WouldBlock
                    || err.kind() == std::io::ErrorKind::TimedOut =>
            {
                return None;
            }
            Err(_) => return None,
        }
    }
    serde_json::from_slice(&buf).ok()
}

fn wait_for_type(
    stream: &mut TcpStream,
    adapter: &mut SocketAdapter,
    state: &mut GameState,
    expected: &str,
) -> Value {
    let deadline = Instant::now() + Duration::from_millis(700);
    while Instant::now() < deadline {
        adapter.poll_and_apply(state);
        if let Some(message) = try_read_json_line(stream)
            && message.get("type").and_then(Value::as_str) == Some(expected)
        {
            return message;
        }
        thread::sleep(Duration::from_millis(2));
    }
    panic!("timed out waiting for {expected}");
}

fn wait_for_observation(
    stream: &mut TcpStream,
    adapter: &mut SocketAdapter,
    state: &mut GameState,
) -> Value {
    let deadline = Instant::now() + Duration::from_millis(900);
    while Instant::now() < deadline {
        adapter.poll_and_apply(state);
        adapter.emit_observation(state);
        if let Some(message) = try_read_json_line(stream)
            && message.get("type").and_then(Value::as_str) == Some("observation")
        {
            return message;
        }
        thread::sleep(Duration::from_millis(2));
    }
    panic!("timed out waiting for observation");
}

fn hello(seq: u64) -> Value {
    hello_with_mode(seq, "action")
}

fn hello_with_mode(seq: u64, mode: &str) -> Value {
    serde_json::json!({
        "type":"hello",
        "seq":seq,
        "ts":1,
        "client":{"name":"test","version":"0.1.0"},
        "protocol_version":"2.0.0",
        "formats":["json"],
        "requested":{"stream_observations":true,"command_mode":mode}
    })
}

#[test]
fn hello_receives_welcome() {
    let (mut adapter, mut state, addr) = test_adapter();
    let mut client = connect(&addr);

    send_json_line(&mut client, hello(1));
    let welcome = wait_for_type(&mut client, &mut adapter, &mut state, "welcome");

    assert_eq!(welcome["seq"], 1);
    assert_eq!(welcome["protocol_version"], "2.0.0");
}

#[test]
fn command_before_hello_receives_handshake_required() {
    let (mut adapter, mut state, addr) = test_adapter();
    let mut client = connect(&addr);

    send_json_line(
        &mut client,
        serde_json::json!({
            "type":"command",
            "seq":1,
            "ts":1,
            "mode":"action",
            "actions":["moveLeft"]
        }),
    );

    let error = wait_for_type(&mut client, &mut adapter, &mut state, "error");
    assert_eq!(error["seq"], 1);
    assert_eq!(error["code"], "handshake_required");
}

#[test]
fn control_before_hello_receives_handshake_required() {
    let (mut adapter, mut state, addr) = test_adapter();
    let mut client = connect(&addr);

    send_json_line(
        &mut client,
        serde_json::json!({"type":"control","seq":1,"ts":1,"action":"claim"}),
    );

    let error = wait_for_type(&mut client, &mut adapter, &mut state, "error");
    assert_eq!(error["seq"], 1);
    assert_eq!(error["code"], "handshake_required");
}

#[test]
fn invalid_json_receives_invalid_command() {
    let (mut adapter, mut state, addr) = test_adapter();
    let mut client = connect(&addr);

    send_raw_line(&mut client, br#"{"type":"hello""#);

    let error = wait_for_type(&mut client, &mut adapter, &mut state, "error");
    assert_eq!(error["seq"], 0);
    assert_eq!(error["code"], "invalid_command");
}

#[test]
fn welcome_includes_required_capabilities() {
    let (mut adapter, mut state, addr) = test_adapter();
    let mut client = connect(&addr);

    send_json_line(&mut client, hello(1));
    let welcome = wait_for_type(&mut client, &mut adapter, &mut state, "welcome");

    assert_eq!(welcome["type"], "welcome");
    assert!(welcome.get("ts").is_some());
    assert_eq!(welcome.get("game_id").and_then(Value::as_str), Some("gpui-tetris"));
    let formats = welcome["capabilities"]["formats"].as_array().expect("formats array");
    assert!(formats.iter().any(|entry| entry.as_str() == Some("json")));
    let modes = welcome["capabilities"]["command_modes"]
        .as_array()
        .expect("command_modes array");
    assert!(modes.iter().any(|entry| entry.as_str() == Some("place")));
}

#[test]
fn first_observation_is_full_snapshot() {
    let (mut adapter, mut state, addr) = test_adapter();
    let mut client = connect(&addr);

    send_json_line(&mut client, hello(1));
    let _ = wait_for_type(&mut client, &mut adapter, &mut state, "welcome");
    let obs = wait_for_observation(&mut client, &mut adapter, &mut state);

    assert_eq!(obs["type"], "observation");
    for key in ["seq", "ts", "playable", "paused", "game_over", "score", "level", "lines"] {
        assert!(obs.get(key).is_some(), "missing {key}");
    }

    assert_eq!(obs["board"]["width"], 10);
    assert_eq!(obs["board"]["height"], 20);
    let cells = obs["board"]["cells"].as_array().expect("cells array");
    assert_eq!(cells.len(), 20);
    for row in cells {
        let row = row.as_array().expect("cell row array");
        assert_eq!(row.len(), 10);
    }

    for key in ["kind", "rotation", "x", "y"] {
        assert!(obs["active"].get(key).is_some(), "missing active.{key}");
    }

    for key in ["drop_ms", "lock_ms", "line_clear_ms"] {
        assert!(obs["timers"].get(key).is_some(), "missing timers.{key}");
    }
}

#[test]
fn pause_toggles_and_playable_reflects_paused() {
    let (mut adapter, mut state, addr) = test_adapter();
    let mut client = connect(&addr);

    send_json_line(&mut client, hello(1));
    let _ = wait_for_type(&mut client, &mut adapter, &mut state, "welcome");

    send_json_line(
        &mut client,
        serde_json::json!({
            "type":"command",
            "seq":2,
            "ts":2,
            "mode":"action",
            "actions":["pause"]
        }),
    );
    let ack = wait_for_type(&mut client, &mut adapter, &mut state, "ack");
    assert_eq!(ack["seq"], 2);

    let paused_obs = wait_for_observation(&mut client, &mut adapter, &mut state);
    assert_eq!(paused_obs["paused"], true);
    assert_eq!(paused_obs["playable"], false);

    send_json_line(
        &mut client,
        serde_json::json!({
            "type":"command",
            "seq":3,
            "ts":3,
            "mode":"action",
            "actions":["pause"]
        }),
    );
    let ack = wait_for_type(&mut client, &mut adapter, &mut state, "ack");
    assert_eq!(ack["seq"], 3);

    let unpaused_obs = wait_for_observation(&mut client, &mut adapter, &mut state);
    assert_eq!(unpaused_obs["paused"], false);
    assert_eq!(unpaused_obs["game_over"], false);
    assert_eq!(unpaused_obs["playable"], true);
}

#[test]
fn closed_loop_stability_reconnect_smoke() {
    let rounds: u32 = std::env::var("TETRIS_AI_STABILITY_ROUNDS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(30);
    let max_pieces_per_round: u32 = std::env::var("TETRIS_AI_STABILITY_MAX_PIECES")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(10_000);

    let (mut adapter, mut state, addr) = test_adapter();

    let mut client = connect(&addr);
    let mut next_seq: u64 = 1;
    let send = |stream: &mut TcpStream, value: Value, seq: &mut u64| {
        let mut value = value;
        value["seq"] = (*seq).into();
        value["ts"] = 1.into();
        *seq += 1;
        send_json_line(stream, value);
    };

    send(&mut client, hello_with_mode(0, "place"), &mut next_seq);
    let _ = wait_for_type(&mut client, &mut adapter, &mut state, "welcome");

    let mut completed_rounds = 0_u32;
    let mut pieces_this_round = 0_u32;
    let mut last_activity = Instant::now();

    while completed_rounds < rounds {
        adapter.poll_and_apply(&mut state);
        adapter.emit_observation(&state);

        if let Some(message) = try_read_json_line(&mut client) {
            last_activity = Instant::now();
            match message.get("type").and_then(Value::as_str) {
                Some("observation") => {
                    let game_over = message.get("game_over").and_then(Value::as_bool) == Some(true);
                    if game_over {
                        send(
                            &mut client,
                            serde_json::json!({
                                "type":"command",
                                "mode":"action",
                                "actions":["restart"]
                            }),
                            &mut next_seq,
                        );
                        completed_rounds += 1;
                        pieces_this_round = 0;
                        if completed_rounds == (rounds / 2).max(1) {
                            drop(client);
                            adapter.poll_and_apply(&mut state);
                            client = connect(&addr);
                            next_seq = 1;
                            send(&mut client, hello_with_mode(0, "place"), &mut next_seq);
                            let _ = wait_for_type(&mut client, &mut adapter, &mut state, "welcome");
                        }
                        continue;
                    }

                    let active = &message["active"];
                    let x = active["x"].as_i64().unwrap_or(0) as i32;
                    let rotation = active["rotation"].as_str().unwrap_or("north");
                    send(
                        &mut client,
                        serde_json::json!({
                            "type":"command",
                            "mode":"place",
                            "place":{"x":x,"rotation":rotation,"useHold":false}
                        }),
                        &mut next_seq,
                    );
                    pieces_this_round += 1;
                    assert!(
                        pieces_this_round <= max_pieces_per_round,
                        "round did not terminate within piece limit"
                    );
                }
                _ => {}
            }
        } else {
            thread::sleep(Duration::from_millis(1));
        }

        assert!(
            Instant::now().duration_since(last_activity) < Duration::from_secs(3),
            "no adapter activity; possible hang"
        );
    }
}

#[test]
fn mixed_command_modes_are_accepted() {
    let (mut adapter, mut state, addr) = test_adapter();
    let mut client = connect(&addr);

    send_json_line(&mut client, hello_with_mode(1, "action"));
    let _ = wait_for_type(&mut client, &mut adapter, &mut state, "welcome");

    send_json_line(
        &mut client,
        serde_json::json!({
            "type":"command",
            "seq":2,
            "ts":2,
            "mode":"place",
            "place":{"x":3,"rotation":"north","useHold":false}
        }),
    );

    let ack = wait_for_type(&mut client, &mut adapter, &mut state, "ack");
    assert_eq!(ack["seq"], 2);
    assert_eq!(ack["status"], "ok");
}

#[test]
fn observer_command_receives_not_controller() {
    let (mut adapter, mut state, addr) = test_adapter();
    let mut controller = connect(&addr);
    let mut observer = connect(&addr);

    send_json_line(&mut controller, hello(1));
    let _ = wait_for_type(&mut controller, &mut adapter, &mut state, "welcome");
    send_json_line(&mut observer, hello(2));
    let _ = wait_for_type(&mut observer, &mut adapter, &mut state, "welcome");

    send_json_line(
        &mut observer,
        serde_json::json!({
            "type":"command",
            "seq":3,
            "ts":2,
            "mode":"action",
            "actions":["moveLeft"]
        }),
    );

    let error = wait_for_type(&mut observer, &mut adapter, &mut state, "error");
    assert_eq!(error["seq"], 3);
    assert_eq!(error["code"], "not_controller");
}

#[test]
fn controller_command_receives_ack_and_applies_action() {
    let (mut adapter, mut state, addr) = test_adapter();
    let mut client = connect(&addr);

    send_json_line(&mut client, hello(1));
    let _ = wait_for_type(&mut client, &mut adapter, &mut state, "welcome");
    let start_x = state.active.x;

    send_json_line(
        &mut client,
        serde_json::json!({
            "type":"command",
            "seq":2,
            "ts":2,
            "mode":"action",
            "actions":["moveLeft"]
        }),
    );
    let ack = wait_for_type(&mut client, &mut adapter, &mut state, "ack");
    assert_eq!(ack["seq"], 2);
    assert_eq!(ack["status"], "ok");
    assert!(state.active.x <= start_x);
}

#[test]
fn release_promotes_observer_to_controller() {
    let (mut adapter, mut state, addr) = test_adapter();
    let mut client1 = connect(&addr);
    let mut client2 = connect(&addr);

    send_json_line(&mut client1, hello(1));
    let _ = wait_for_type(&mut client1, &mut adapter, &mut state, "welcome");
    send_json_line(&mut client2, hello(2));
    let _ = wait_for_type(&mut client2, &mut adapter, &mut state, "welcome");

    send_json_line(
        &mut client1,
        serde_json::json!({"type":"control","seq":3,"ts":2,"action":"release"}),
    );
    let release_ack = wait_for_type(&mut client1, &mut adapter, &mut state, "ack");
    assert_eq!(release_ack["seq"], 3);

    send_json_line(
        &mut client2,
        serde_json::json!({
            "type":"command",
            "seq":4,
            "ts":2,
            "mode":"action",
            "actions":["moveRight"]
        }),
    );
    let ack = wait_for_type(&mut client2, &mut adapter, &mut state, "ack");
    assert_eq!(ack["seq"], 4);
    assert_eq!(ack["status"], "ok");
}

#[test]
fn hello_with_mismatched_major_receives_protocol_mismatch() {
    let (mut adapter, mut state, addr) = test_adapter();
    let mut client = connect(&addr);

    send_json_line(
        &mut client,
        serde_json::json!({
            "type":"hello",
            "seq":1,
            "ts":1,
            "client":{"name":"test","version":"0.1.0"},
            "protocol_version":"3.0.0",
            "formats":["json"],
            "requested":{"stream_observations":true,"command_mode":"action"}
        }),
    );

    let error = wait_for_type(&mut client, &mut adapter, &mut state, "error");
    assert_eq!(error["seq"], 1);
    assert_eq!(error["code"], "protocol_mismatch");
}

#[test]
fn command_queue_overflow_receives_backpressure() {
    let (mut adapter, mut state, addr) = test_adapter_with_config(AdapterConfig {
        host: "127.0.0.1".to_string(),
        port: 0,
        idle_timeout_ms: None,
        max_pending_commands: 0,
        log_path: None,
        ..AdapterConfig::default()
    });
    let mut client = connect(&addr);

    send_json_line(&mut client, hello(1));
    let _ = wait_for_type(&mut client, &mut adapter, &mut state, "welcome");

    send_json_line(
        &mut client,
        serde_json::json!({
            "type":"command",
            "seq":2,
            "ts":2,
            "mode":"action",
            "actions":["moveLeft"]
        }),
    );

    let error = wait_for_type(&mut client, &mut adapter, &mut state, "error");
    assert_eq!(error["seq"], 2);
    assert_eq!(error["code"], "backpressure");
}

#[test]
fn restart_command_transitions_to_playable_and_unpaused() {
    let (mut adapter, mut state, addr) = test_adapter();
    let mut client = connect(&addr);

    state.paused = true;
    state.game_over = false;

    send_json_line(&mut client, hello(1));
    let _ = wait_for_type(&mut client, &mut adapter, &mut state, "welcome");

    send_json_line(
        &mut client,
        serde_json::json!({
            "type":"command",
            "seq":2,
            "ts":2,
            "mode":"action",
            "actions":["restart"]
        }),
    );

    let ack = wait_for_type(&mut client, &mut adapter, &mut state, "ack");
    assert_eq!(ack["seq"], 2);
    assert_eq!(ack["status"], "ok");

    let observation = wait_for_observation(&mut client, &mut adapter, &mut state);
    assert_eq!(observation["playable"], true);
    assert_eq!(observation["paused"], false);
    assert_eq!(observation["game_over"], false);
}

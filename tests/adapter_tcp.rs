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
    // Do not consume partial frames: a timeout must not lose a prefix.
    let mut bytes = [0_u8; 65_537];
    let n = stream.peek(&mut bytes).ok()?;
    let end = bytes[..n].iter().position(|byte| *byte == b'\n')?;
    let mut frame = vec![0; end + 1];
    stream.read_exact(&mut frame).ok()?;
    serde_json::from_slice(&frame[..end]).ok()
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
    hello_with_role(seq, mode, "auto")
}

fn hello_with_role(seq: u64, mode: &str, role: &str) -> Value {
    serde_json::json!({
        "type":"hello",
        "seq":seq,
        "ts":1,
        "client":{"name":"test","version":"0.1.0"},
        "protocol_version":"3.0.0",
        "formats":["json"],
        "requested":{"stream_observations":true,"command_mode":mode,"role":role}
    })
}

#[test]
fn hello_receives_welcome() {
    let (mut adapter, mut state, addr) = test_adapter();
    let mut client = connect(&addr);

    send_json_line(&mut client, hello(1));
    let welcome = wait_for_type(&mut client, &mut adapter, &mut state, "welcome");

    assert_eq!(welcome["seq"], 1);
    assert_eq!(welcome["protocol_version"], "3.0.0");
    assert_eq!(welcome["client_id"], 1);
    assert_eq!(welcome["role"], "controller");
    assert_eq!(welcome["controller_id"], 1);
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
    assert_eq!(
        welcome.get("game_id").and_then(Value::as_str),
        Some("gpui-tetris")
    );
    let formats = welcome["capabilities"]["formats"]
        .as_array()
        .expect("formats array");
    assert!(formats.iter().any(|entry| entry.as_str() == Some("json")));
    let modes = welcome["capabilities"]["command_modes"]
        .as_array()
        .expect("command_modes array");
    assert!(modes.iter().any(|entry| entry.as_str() == Some("place")));
    let features = welcome["capabilities"]["features"]
        .as_array()
        .expect("features array");
    for required in ["board_id", "events", "logical_step", "state_hash"] {
        assert!(
            features
                .iter()
                .any(|entry| entry.as_str() == Some(required)),
            "missing capability {required}"
        );
    }
    assert_eq!(
        welcome["capabilities"]["control_policy"]["promotion_order"],
        "lowest_client_id"
    );
}

#[test]
fn first_observation_is_full_snapshot() {
    let (mut adapter, mut state, addr) = test_adapter();
    let mut client = connect(&addr);

    send_json_line(&mut client, hello(1));
    let _ = wait_for_type(&mut client, &mut adapter, &mut state, "welcome");
    let obs = wait_for_observation(&mut client, &mut adapter, &mut state);

    assert_eq!(obs["type"], "observation");
    for key in [
        "seq",
        "ts",
        "logical_step",
        "playable",
        "paused",
        "game_over",
        "episode_id",
        "seed",
        "piece_id",
        "step_in_piece",
        "board_id",
        "events",
        "state_hash",
        "score",
        "level",
        "lines",
    ] {
        assert!(obs.get(key).is_some(), "missing {key}");
    }

    assert_eq!(obs["board"]["width"], 10);
    assert_eq!(obs["board"]["height"], 20);
    let cells = obs["board"]["cells"].as_array().expect("cells array");
    assert_eq!(cells.len(), 20);
    for row in cells {
        let row = row.as_array().expect("cell row array");
        assert_eq!(row.len(), 10);
        assert!(
            row.iter()
                .all(|cell| cell.as_u64().is_some_and(|value| value <= 7))
        );
    }

    for key in ["kind", "rotation", "x", "y"] {
        assert!(obs["active"].get(key).is_some(), "missing active.{key}");
    }

    for key in ["drop_ms", "lock_ms", "line_clear_ms"] {
        assert!(obs["timers"].get(key).is_some(), "missing timers.{key}");
    }
    assert_eq!(obs["next_queue"].as_array().map(Vec::len), Some(5));
    assert_eq!(obs["next"], obs["next_queue"][0]);
    assert!(obs["events"].is_array());
    assert_eq!(obs["state_hash"].as_str().map(str::len), Some(16));
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
    assert_eq!(ack["correlation_seq"], 2);
    assert!(ack["applied_step"].is_u64());
    assert_eq!(ack["state_hash"].as_str().map(str::len), Some(16));

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
        adapter.emit_observation(&mut state);

        if let Some(message) = try_read_json_line(&mut client) {
            last_activity = Instant::now();
            if message.get("type").and_then(Value::as_str) == Some("observation") {
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
    send_json_line(&mut observer, hello(1));
    let _ = wait_for_type(&mut observer, &mut adapter, &mut state, "welcome");

    send_json_line(
        &mut observer,
        serde_json::json!({
            "type":"command",
            "seq":2,
            "ts":2,
            "mode":"action",
            "actions":["moveLeft"]
        }),
    );

    let error = wait_for_type(&mut observer, &mut adapter, &mut state, "error");
    assert_eq!(error["seq"], 2);
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
    let observation = wait_for_observation(&mut client, &mut adapter, &mut state);
    assert_eq!(ack["state_hash"], observation["state_hash"]);
    assert_eq!(ack["applied_step"], observation["logical_step"]);
}

#[test]
fn release_clears_controller_until_observer_claims() {
    let (mut adapter, mut state, addr) = test_adapter();
    let mut client1 = connect(&addr);
    let mut client2 = connect(&addr);

    send_json_line(&mut client1, hello(1));
    let _ = wait_for_type(&mut client1, &mut adapter, &mut state, "welcome");
    send_json_line(&mut client2, hello(1));
    let _ = wait_for_type(&mut client2, &mut adapter, &mut state, "welcome");

    send_json_line(
        &mut client1,
        serde_json::json!({"type":"control","seq":2,"ts":2,"action":"release"}),
    );
    let release_ack = wait_for_type(&mut client1, &mut adapter, &mut state, "ack");
    assert_eq!(release_ack["seq"], 2);
    assert_eq!(release_ack["correlation_seq"], 2);
    assert!(release_ack.get("applied_step").is_none());
    assert!(release_ack.get("state_hash").is_none());

    send_json_line(
        &mut client2,
        serde_json::json!({"type":"control","seq":2,"ts":2,"action":"claim"}),
    );
    let claim_ack = wait_for_type(&mut client2, &mut adapter, &mut state, "ack");
    assert_eq!(claim_ack["seq"], 2);
    assert_eq!(claim_ack["correlation_seq"], 2);
    assert!(claim_ack.get("applied_step").is_none());
    assert!(claim_ack.get("state_hash").is_none());

    send_json_line(
        &mut client2,
        serde_json::json!({
            "type":"command",
            "seq":3,
            "ts":2,
            "mode":"action",
            "actions":["moveRight"]
        }),
    );
    let ack = wait_for_type(&mut client2, &mut adapter, &mut state, "ack");
    assert_eq!(ack["seq"], 3);
    assert_eq!(ack["status"], "ok");
}

#[test]
fn duplicate_or_decreasing_sequence_is_rejected() {
    let (mut adapter, mut state, addr) = test_adapter();
    let mut client = connect(&addr);
    send_json_line(&mut client, hello(1));
    let _ = wait_for_type(&mut client, &mut adapter, &mut state, "welcome");

    let command = serde_json::json!({
        "type":"command","seq":2,"ts":2,"mode":"action","actions":[]
    });
    send_json_line(&mut client, command.clone());
    let _ = wait_for_type(&mut client, &mut adapter, &mut state, "ack");
    let logical_step = state.logical_step;

    send_json_line(&mut client, command);
    let error = wait_for_type(&mut client, &mut adapter, &mut state, "error");
    assert_eq!(error["seq"], 2);
    assert_eq!(error["code"], "invalid_command");
    assert_eq!(state.logical_step, logical_step);
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
            "protocol_version":"2.1.1",
            "formats":["json"],
            "requested":{"stream_observations":true,"command_mode":"action","role":"auto"}
        }),
    );

    let error = wait_for_type(&mut client, &mut adapter, &mut state, "error");
    assert_eq!(error["seq"], 1);
    assert_eq!(error["code"], "protocol_mismatch");
}

#[test]
fn malformed_semver_receives_protocol_mismatch() {
    let (mut adapter, mut state, addr) = test_adapter();
    let mut client = connect(&addr);
    let mut message = hello(1);
    message["protocol_version"] = "3".into();
    send_json_line(&mut client, message);

    let error = wait_for_type(&mut client, &mut adapter, &mut state, "error");
    assert_eq!(error["code"], "protocol_mismatch");
}

#[test]
fn hello_requires_sequence_one_and_json_format() {
    let (mut adapter, mut state, addr) = test_adapter();
    let mut wrong_seq = connect(&addr);
    send_json_line(&mut wrong_seq, hello(2));
    let error = wait_for_type(&mut wrong_seq, &mut adapter, &mut state, "error");
    assert_eq!(error["code"], "invalid_command");

    let mut missing_json = connect(&addr);
    let mut message = hello(1);
    message["formats"] = serde_json::json!(["msgpack"]);
    send_json_line(&mut missing_json, message);
    let error = wait_for_type(&mut missing_json, &mut adapter, &mut state, "error");
    assert_eq!(error["code"], "invalid_command");
}

#[test]
fn hello_requires_client_identity() {
    let (mut adapter, mut state, addr) = test_adapter();
    let mut client = connect(&addr);
    let mut message = hello(1);
    message
        .as_object_mut()
        .expect("hello object")
        .remove("client");
    send_json_line(&mut client, message);

    let error = wait_for_type(&mut client, &mut adapter, &mut state, "error");
    assert_eq!(error["code"], "invalid_command");
}

#[test]
fn observer_request_is_never_auto_promoted() {
    let (mut adapter, mut state, addr) = test_adapter();
    let mut controller = connect(&addr);
    let mut observer = connect(&addr);
    send_json_line(&mut controller, hello_with_role(1, "action", "auto"));
    let _ = wait_for_type(&mut controller, &mut adapter, &mut state, "welcome");
    send_json_line(&mut observer, hello_with_role(1, "action", "observer"));
    let welcome = wait_for_type(&mut observer, &mut adapter, &mut state, "welcome");
    assert_eq!(welcome["role"], "observer");

    drop(controller);
    for _ in 0..10 {
        adapter.poll_and_apply(&mut state);
        thread::sleep(Duration::from_millis(2));
    }
    send_json_line(
        &mut observer,
        serde_json::json!({
            "type":"command","seq":2,"ts":2,"mode":"action","actions":[]
        }),
    );
    let error = wait_for_type(&mut observer, &mut adapter, &mut state, "error");
    assert_eq!(error["code"], "not_controller");
}

#[test]
fn seeded_restart_is_deterministic_and_starts_new_episode() {
    let (mut adapter, mut state, addr) = test_adapter();
    let mut client = connect(&addr);
    send_json_line(&mut client, hello(1));
    let _ = wait_for_type(&mut client, &mut adapter, &mut state, "welcome");

    let restart = |seq| {
        serde_json::json!({
            "type":"command","seq":seq,"ts":2,"mode":"action",
            "actions":["restart"],"restart":{"seed":42}
        })
    };
    send_json_line(&mut client, restart(2));
    let _ = wait_for_type(&mut client, &mut adapter, &mut state, "ack");
    let first = wait_for_observation(&mut client, &mut adapter, &mut state);
    send_json_line(&mut client, restart(3));
    let _ = wait_for_type(&mut client, &mut adapter, &mut state, "ack");
    let second = wait_for_observation(&mut client, &mut adapter, &mut state);

    assert_eq!(first["seed"], 42);
    assert_eq!(second["seed"], 42);
    assert_eq!(first["active"], second["active"]);
    assert_eq!(first["next_queue"], second["next_queue"]);
    assert!(second["episode_id"].as_u64() > first["episode_id"].as_u64());
}

#[test]
fn piece_id_changes_only_when_active_piece_changes() {
    let (mut adapter, mut state, addr) = test_adapter();
    let mut client = connect(&addr);
    send_json_line(&mut client, hello(1));
    let _ = wait_for_type(&mut client, &mut adapter, &mut state, "welcome");
    let before = wait_for_observation(&mut client, &mut adapter, &mut state);

    send_json_line(
        &mut client,
        serde_json::json!({
            "type":"command","seq":2,"ts":2,"mode":"action","actions":["moveLeft"]
        }),
    );
    let _ = wait_for_type(&mut client, &mut adapter, &mut state, "ack");
    let moved = wait_for_observation(&mut client, &mut adapter, &mut state);
    assert_eq!(moved["piece_id"], before["piece_id"]);
    assert!(moved["step_in_piece"].as_u64() > before["step_in_piece"].as_u64());

    send_json_line(
        &mut client,
        serde_json::json!({
            "type":"command","seq":3,"ts":3,"mode":"action","actions":["hardDrop"]
        }),
    );
    let _ = wait_for_type(&mut client, &mut adapter, &mut state, "ack");
    let spawned = wait_for_observation(&mut client, &mut adapter, &mut state);
    assert!(spawned["piece_id"].as_u64() > moved["piece_id"].as_u64());
}

#[test]
fn lock_event_reports_actual_clear_and_score() {
    let (mut adapter, mut state, addr) = test_adapter();
    let mut client = connect(&addr);
    send_json_line(&mut client, hello(1));
    let _ = wait_for_type(&mut client, &mut adapter, &mut state, "welcome");

    use gpui_tetris::game::board::{BOARD_HEIGHT, BOARD_WIDTH};
    use gpui_tetris::game::pieces::{Tetromino, TetrominoType};
    for x in 0..BOARD_WIDTH {
        if !(3..=6).contains(&x) {
            state.board.cells[BOARD_HEIGHT - 1][x].kind =
                Some(gpui_tetris::game::pieces::TetrominoType::I);
            state.board.cells[BOARD_HEIGHT - 1][x].kind = Some(TetrominoType::O);
        }
    }
    state.active = Tetromino::new(TetrominoType::I, 3, BOARD_HEIGHT as i32 - 2);
    send_json_line(
        &mut client,
        serde_json::json!({
            "type":"command","seq":2,"ts":2,"mode":"action","actions":["hardDrop"]
        }),
    );
    let _ = wait_for_type(&mut client, &mut adapter, &mut state, "ack");
    let observation = wait_for_observation(&mut client, &mut adapter, &mut state);
    let event = &observation["events"][0];
    assert_eq!(event["locked"], true);
    assert_eq!(event["lines_cleared"], 1);
    assert!(
        event["line_clear_score"]
            .as_u64()
            .is_some_and(|score| score > 0)
    );
}

#[test]
fn events_belong_only_to_the_observed_transition() {
    use gpui_tetris::game::input::GameAction;

    let (mut adapter, mut state, addr) = test_adapter();
    let mut client = connect(&addr);
    send_json_line(&mut client, hello(1));
    let _ = wait_for_type(&mut client, &mut adapter, &mut state, "welcome");
    let _ = wait_for_observation(&mut client, &mut adapter, &mut state);

    for _ in 0..5 {
        state.apply_action(GameAction::HardDrop);
    }
    let observation = wait_for_observation(&mut client, &mut adapter, &mut state);
    let events = observation["events"].as_array().expect("events array");
    assert_eq!(events.len(), 1);
    assert_eq!(observation["logical_step"], state.logical_step);
    assert!(events.iter().all(|event| event["locked"] == true));
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
    assert!(
        error["retry_after_ms"]
            .as_u64()
            .is_some_and(|value| value > 0)
    );
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

#[test]
fn action_count_and_restart_payload_are_validated() {
    let (mut adapter, mut state, addr) = test_adapter();
    let mut client = connect(&addr);
    send_json_line(&mut client, hello(1));
    let _ = wait_for_type(&mut client, &mut adapter, &mut state, "welcome");

    send_json_line(
        &mut client,
        serde_json::json!({
            "type":"command","seq":2,"ts":2,"mode":"action",
            "actions":vec!["moveLeft"; 33]
        }),
    );
    let error = wait_for_type(&mut client, &mut adapter, &mut state, "error");
    assert_eq!(error["code"], "invalid_command");

    send_json_line(
        &mut client,
        serde_json::json!({
            "type":"command","seq":3,"ts":3,"mode":"action",
            "actions":[],"restart":{"seed":42}
        }),
    );
    let error = wait_for_type(&mut client, &mut adapter, &mut state, "error");
    assert_eq!(error["code"], "invalid_command");
}

#[test]
fn invalid_place_is_atomic() {
    let (mut adapter, mut state, addr) = test_adapter();
    let mut client = connect(&addr);
    send_json_line(&mut client, hello_with_mode(1, "place"));
    let _ = wait_for_type(&mut client, &mut adapter, &mut state, "welcome");
    let active = state.active;
    let hold = state.hold;
    let queue = state.next_queue.clone();
    let score = state.score;
    let logical_step = state.logical_step;
    let board_revision = state.board_revision();

    send_json_line(
        &mut client,
        serde_json::json!({
            "type":"command","seq":2,"ts":2,"mode":"place",
            "place":{"x":127,"rotation":"north","useHold":true}
        }),
    );
    let error = wait_for_type(&mut client, &mut adapter, &mut state, "error");
    assert_eq!(error["code"], "invalid_place");
    assert_eq!(state.active, active);
    assert_eq!(state.hold, hold);
    assert_eq!(state.next_queue, queue);
    assert_eq!(state.score, score);
    assert_eq!(state.logical_step, logical_step);
    assert_eq!(state.board_revision(), board_revision);
}

#[test]
fn place_origin_outside_schema_range_is_invalid_command() {
    let (mut adapter, mut state, addr) = test_adapter();
    let mut client = connect(&addr);
    send_json_line(&mut client, hello_with_mode(1, "place"));
    let _ = wait_for_type(&mut client, &mut adapter, &mut state, "welcome");

    send_json_line(
        &mut client,
        serde_json::json!({
            "type":"command","seq":2,"ts":2,"mode":"place",
            "place":{"x":128,"rotation":"north","useHold":false}
        }),
    );
    let error = wait_for_type(&mut client, &mut adapter, &mut state, "error");
    assert_eq!(error["code"], "invalid_command");
}

#[test]
fn observer_is_not_disconnected_by_inbound_idle_timeout() {
    let (mut adapter, mut state, addr) = test_adapter_with_config(AdapterConfig {
        host: "127.0.0.1".to_string(),
        port: 0,
        idle_timeout_ms: Some(10_000),
        log_path: None,
        ..AdapterConfig::default()
    });
    let mut controller = connect(&addr);
    send_json_line(&mut controller, hello(1));
    let _ = wait_for_type(&mut controller, &mut adapter, &mut state, "welcome");
    let mut observer = connect(&addr);
    send_json_line(&mut observer, hello_with_role(1, "action", "observer"));
    let _ = wait_for_type(&mut observer, &mut adapter, &mut state, "welcome");
    adapter.poll_and_apply_at(&mut state, Instant::now() + Duration::from_secs(11));

    send_json_line(
        &mut observer,
        serde_json::json!({"type":"control","seq":2,"ts":2,"action":"claim"}),
    );
    let ack = wait_for_type(&mut observer, &mut adapter, &mut state, "ack");
    assert_eq!(ack["correlation_seq"], 2);
}

#[test]
fn eligible_observer_is_promoted_after_controller_disconnect() {
    let (mut adapter, mut state, addr) = test_adapter();
    let mut controller = connect(&addr);
    let mut observer = connect(&addr);
    send_json_line(&mut controller, hello(1));
    let _ = wait_for_type(&mut controller, &mut adapter, &mut state, "welcome");
    send_json_line(&mut observer, hello(1));
    let _ = wait_for_type(&mut observer, &mut adapter, &mut state, "welcome");
    drop(controller);
    for _ in 0..10 {
        adapter.poll_and_apply(&mut state);
        thread::sleep(Duration::from_millis(2));
    }

    send_json_line(
        &mut observer,
        serde_json::json!({
            "type":"command","seq":2,"ts":2,"mode":"action","actions":[]
        }),
    );
    let ack = wait_for_type(&mut observer, &mut adapter, &mut state, "ack");
    assert_eq!(ack["correlation_seq"], 2);
}

#[test]
fn unterminated_oversized_frame_closes_only_that_client() {
    let (mut adapter, mut state, addr) = test_adapter();
    let mut oversized = connect(&addr);
    let mut healthy = connect(&addr);
    oversized
        .write_all(&vec![b'x'; 65_537])
        .expect("write oversized frame");
    for _ in 0..25 {
        adapter.poll_and_apply(&mut state);
        thread::sleep(Duration::from_millis(2));
    }

    send_json_line(&mut healthy, hello(1));
    let welcome = wait_for_type(&mut healthy, &mut adapter, &mut state, "welcome");
    assert_eq!(welcome["protocol_version"], "3.0.0");

    let mut byte = [0_u8; 1];
    assert_eq!(oversized.read(&mut byte).ok(), Some(0));
}

#[test]
fn frame_at_exact_payload_limit_is_accepted() {
    let (mut adapter, mut state, addr) = test_adapter();
    let mut client = connect(&addr);
    let mut payload = serde_json::to_vec(&hello(1)).expect("encode hello");
    payload.resize(65_536, b' ');
    send_raw_line(&mut client, &payload);

    let welcome = wait_for_type(&mut client, &mut adapter, &mut state, "welcome");
    assert_eq!(welcome["protocol_version"], "3.0.0");
}

#[test]
fn board_id_changes_only_when_locked_board_changes() {
    let (mut adapter, mut state, addr) = test_adapter();
    let mut client = connect(&addr);
    send_json_line(&mut client, hello(1));
    let _ = wait_for_type(&mut client, &mut adapter, &mut state, "welcome");
    let initial = wait_for_observation(&mut client, &mut adapter, &mut state);

    send_json_line(
        &mut client,
        serde_json::json!({
            "type":"command","seq":2,"ts":2,"mode":"action","actions":["moveLeft"]
        }),
    );
    let _ = wait_for_type(&mut client, &mut adapter, &mut state, "ack");
    let moved = wait_for_observation(&mut client, &mut adapter, &mut state);
    assert_eq!(moved["board_id"], initial["board_id"]);

    send_json_line(
        &mut client,
        serde_json::json!({
            "type":"command","seq":3,"ts":3,"mode":"action","actions":["hardDrop"]
        }),
    );
    let _ = wait_for_type(&mut client, &mut adapter, &mut state, "ack");
    let locked = wait_for_observation(&mut client, &mut adapter, &mut state);
    assert!(locked["board_id"].as_u64() > moved["board_id"].as_u64());
}

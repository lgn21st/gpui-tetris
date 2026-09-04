use gpui_tetris::{
    adapter::{AdapterConfig, SocketAdapter},
    game::state::GameState,
    runtime::Runtime,
};
use std::{
    io::{Read, Write},
    net::TcpStream,
    time::{Duration, Instant},
};

#[test]
fn headless_runtime_services_handshake_restart_and_simulation() {
    let adapter = SocketAdapter::bind(AdapterConfig {
        port: 0,
        idle_timeout_ms: None,
        ..Default::default()
    })
    .unwrap();
    let mut client = TcpStream::connect(adapter.local_addr().unwrap()).unwrap();
    client.set_nonblocking(true).unwrap();
    let mut runtime = Runtime::new(GameState::new(1, Default::default()), Some(adapter));
    client.write_all(b"{\"type\":\"hello\",\"seq\":1,\"ts\":0,\"client\":{\"name\":\"test\",\"version\":\"1\"},\"protocol_version\":\"3.0.0\",\"formats\":[\"json\"],\"requested\":{\"stream_observations\":true,\"command_mode\":\"action\"}}\n").unwrap();
    let mut received = Vec::new();
    let start = Instant::now();
    let mut messages = Vec::new();
    let mut restart_sent = false;
    let mut acknowledged = false;
    let mut observed_step = 0;
    for tick in 0..100 {
        runtime.pump(start + Duration::from_millis(tick * 16), |_, _, _| {});
        let mut bytes = [0; 8192];
        loop {
            match client.read(&mut bytes) {
                Ok(0) => panic!("connection closed"),
                Ok(n) => received.extend_from_slice(&bytes[..n]),
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) => panic!("{e}"),
            }
        }
        while let Some(end) = received.iter().position(|byte| *byte == b'\n') {
            let frame: Vec<_> = received.drain(..=end).collect();
            messages.push(serde_json::from_slice::<serde_json::Value>(&frame).unwrap());
        }
        for msg in messages.drain(..) {
            if msg["type"] == "welcome" && !restart_sent {
                client.write_all(b"{\"type\":\"command\",\"seq\":2,\"ts\":1,\"mode\":\"action\",\"actions\":[\"restart\"],\"restart\":{\"seed\":42}}\n").unwrap();
                restart_sent = true;
            }
            if msg["type"] == "ack" {
                assert_eq!(msg["correlation_seq"], 2);
                acknowledged = true;
            }
            if msg["type"] == "observation" && msg["seed"] == 42 {
                observed_step = msg["logical_step"].as_u64().unwrap();
            }
        }
        if acknowledged && observed_step > 5 {
            break;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(acknowledged);
    assert!(observed_step > 5);
    assert!(runtime.playable());
}

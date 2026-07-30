use super::*;
use semver::Version;
use std::collections::{HashMap, VecDeque};
use std::env;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_PORT: u16 = 7777;
const DEFAULT_PROTOCOL_VERSION: &str = "3.0.0";
const DEFAULT_GAME_ID: &str = "gpui-tetris";
const MAX_FRAME_PAYLOAD_BYTES: usize = 65_536;
const MAX_RESPONSE_BUFFER_BYTES: usize = 262_144;
const MAX_CLIENTS: usize = 16;
const BACKPRESSURE_RETRY_AFTER_MS: u64 = 16;

#[derive(Clone, Debug)]
pub struct AdapterConfig {
    pub host: String,
    pub port: u16,
    pub protocol_version: String,
    pub game_id: String,
    pub idle_timeout_ms: Option<u64>,
    pub max_pending_commands: usize,
    pub observation_interval_ms: Option<u64>,
    pub log_path: Option<String>,
}

impl Default for AdapterConfig {
    fn default() -> Self {
        Self {
            host: DEFAULT_HOST.to_string(),
            port: DEFAULT_PORT,
            protocol_version: DEFAULT_PROTOCOL_VERSION.to_string(),
            game_id: DEFAULT_GAME_ID.to_string(),
            idle_timeout_ms: Some(2_000),
            max_pending_commands: 64,
            observation_interval_ms: None,
            log_path: Some("auto".to_string()),
        }
    }
}

impl AdapterConfig {
    pub fn from_env() -> Option<Self> {
        if env::var("TETRIS_AI_DISABLED")
            .ok()
            .is_some_and(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        {
            return None;
        }

        let mut config = Self::default();
        if let Ok(host) = env::var("TETRIS_AI_HOST")
            && !host.trim().is_empty()
        {
            config.host = host;
        }
        if let Ok(port) = env::var("TETRIS_AI_PORT")
            && let Ok(parsed) = port.parse::<u16>()
        {
            config.port = parsed;
        }
        if let Ok(value) = env::var("TETRIS_AI_IDLE_TIMEOUT_MS")
            && let Ok(parsed) = value.parse::<i64>()
        {
            config.idle_timeout_ms = if parsed > 0 {
                Some(parsed as u64)
            } else {
                None
            };
        }
        if let Ok(value) = env::var("TETRIS_AI_MAX_PENDING")
            && let Ok(parsed) = value.parse::<usize>()
            && parsed > 0
        {
            config.max_pending_commands = parsed;
        }
        if let Ok(value) = env::var("TETRIS_AI_OBSERVATION_MS")
            && let Ok(parsed) = value.parse::<i64>()
        {
            config.observation_interval_ms = if parsed >= 0 {
                Some(parsed as u64)
            } else {
                None
            };
        }
        if let Ok(path) = env::var("TETRIS_AI_LOG_PATH") {
            if path.trim().is_empty() {
                config.log_path = None;
            } else {
                config.log_path = Some(path);
            }
        }

        Some(config)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ClientRole {
    Controller,
    Observer,
}

impl ClientRole {
    fn as_str(self) -> &'static str {
        match self {
            Self::Controller => "controller",
            Self::Observer => "observer",
        }
    }
}

#[derive(Debug)]
struct ClientState {
    stream: TcpStream,
    read_buf: Vec<u8>,
    write_buf: Vec<u8>,
    handshake_complete: bool,
    stream_observations: bool,
    requested_mode: Option<CommandMode>,
    requested_role: RequestedRole,
    role: ClientRole,
    last_seq: Option<u64>,
    last_read_at_ms: u64,
    needs_snapshot: bool,
    observation_buf: Option<Vec<u8>>,
    observation_offset: usize,
    next_observation: Option<Vec<u8>>,
    disconnect_requested: bool,
}

impl ClientState {
    fn new(stream: TcpStream, now_ms: u64) -> Self {
        Self {
            stream,
            read_buf: Vec::with_capacity(4096),
            write_buf: Vec::with_capacity(1024),
            handshake_complete: false,
            stream_observations: false,
            requested_mode: None,
            requested_role: RequestedRole::Auto,
            role: ClientRole::Observer,
            last_seq: None,
            last_read_at_ms: now_ms,
            needs_snapshot: false,
            observation_buf: None,
            observation_offset: 0,
            next_observation: None,
            disconnect_requested: false,
        }
    }
}

#[derive(Debug)]
struct PendingCommand {
    connection_id: usize,
    seq: u64,
    command: ProtocolCommand,
}

pub struct SocketAdapter {
    config: AdapterConfig,
    listener: TcpListener,
    clients: HashMap<usize, ClientState>,
    controller_id: Option<usize>,
    next_connection_id: usize,
    out_seq: u64,
    pending_commands: VecDeque<PendingCommand>,
    last_observation_ts_ms: Option<u64>,
    log_file: Option<File>,
}

impl SocketAdapter {
    pub fn from_env() -> Result<Option<Self>, String> {
        let Some(config) = AdapterConfig::from_env() else {
            return Ok(None);
        };
        Self::bind(config).map(Some)
    }

    pub fn bind(config: AdapterConfig) -> Result<Self, String> {
        let bind_addr = format!("{}:{}", config.host, config.port);
        let listener = TcpListener::bind(&bind_addr)
            .map_err(|err| format!("adapter bind failed on {bind_addr}: {err}"))?;
        listener
            .set_nonblocking(true)
            .map_err(|err| format!("adapter nonblocking listener failed: {err}"))?;
        let log_file = open_log_file(config.log_path.as_deref());
        Ok(Self {
            config,
            listener,
            clients: HashMap::new(),
            controller_id: None,
            next_connection_id: 1,
            out_seq: 0,
            pending_commands: VecDeque::new(),
            last_observation_ts_ms: None,
            log_file,
        })
    }

    pub fn local_addr(&self) -> Option<String> {
        self.listener.local_addr().ok().map(|addr| addr.to_string())
    }

    pub fn poll_and_apply(&mut self, state: &mut GameState) -> bool {
        let now_ms = now_unix_ms();
        self.accept_clients(now_ms);
        self.read_from_clients(now_ms);

        let mut changed = false;
        while let Some(pending) = self.pending_commands.pop_front() {
            let result = apply_protocol_command(&pending.command, state)
                .map(|()| changed = true)
                .map_err(|err| err.to_error());

            match result {
                Ok(()) => {
                    self.send_game_ack(pending.connection_id, pending.seq, state);
                }
                Err((code, message)) => {
                    self.send_error(pending.connection_id, pending.seq, code, message);
                }
            }
        }

        self.flush_writes();
        changed
    }

    pub fn emit_observation(&mut self, state: &mut GameState) {
        if self.clients.is_empty() {
            return;
        }
        let targets: Vec<usize> = self
            .clients
            .iter()
            .filter_map(|(id, client)| {
                if client.handshake_complete && client.stream_observations {
                    Some(*id)
                } else {
                    None
                }
            })
            .collect();
        if targets.is_empty() {
            return;
        }
        let now_ms = now_unix_ms();
        let snapshot_required = targets.iter().any(|id| {
            self.clients
                .get(id)
                .is_some_and(|client| client.needs_snapshot)
        });
        if let Some(interval) = self.config.observation_interval_ms
            && let Some(last) = self.last_observation_ts_ms
            && now_ms.saturating_sub(last) < interval
            && !snapshot_required
        {
            return;
        }

        self.out_seq = self.out_seq.saturating_add(1);
        let events = state.take_protocol_events();
        let message =
            OutMessage::Observation(Observation::from_state(self.out_seq, now_ms, state, events));
        let Some(encoded) = encode_message(&message) else {
            return;
        };

        for id in targets {
            self.send_observation(id, &encoded);
            if let Some(client) = self.clients.get_mut(&id) {
                client.needs_snapshot = false;
            }
        }
        self.last_observation_ts_ms = Some(now_ms);
        self.flush_writes();
    }

    fn accept_clients(&mut self, now_ms: u64) {
        loop {
            let accepted = self.listener.accept();
            let (stream, _) = match accepted {
                Ok(pair) => pair,
                Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(_) => break,
            };
            if stream.set_nonblocking(true).is_err() {
                continue;
            }
            if self.clients.len() >= MAX_CLIENTS {
                continue;
            }
            let id = self.next_connection_id;
            self.next_connection_id = self.next_connection_id.saturating_add(1);
            self.clients.insert(id, ClientState::new(stream, now_ms));
        }
    }

    fn read_from_clients(&mut self, now_ms: u64) {
        let ids: Vec<usize> = self.clients.keys().copied().collect();
        let mut to_drop = Vec::new();
        let mut frames: Vec<(usize, Vec<u8>)> = Vec::new();

        for id in ids {
            let Some(client) = self.clients.get_mut(&id) else {
                continue;
            };
            let mut temp = [0_u8; 4096];
            loop {
                match client.stream.read(&mut temp) {
                    Ok(0) => {
                        to_drop.push(id);
                        break;
                    }
                    Ok(n) => {
                        client.last_read_at_ms = now_ms;
                        client.read_buf.extend_from_slice(&temp[..n]);
                        if client.read_buf.len() > MAX_FRAME_PAYLOAD_BYTES + 1 {
                            break;
                        }
                    }
                    Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => break,
                    Err(_) => {
                        to_drop.push(id);
                        break;
                    }
                }
            }

            let drained = drain_lines(&mut client.read_buf);
            for line in drained {
                if line.len() > MAX_FRAME_PAYLOAD_BYTES {
                    to_drop.push(id);
                } else {
                    frames.push((id, line));
                }
            }
            if client.read_buf.len() > MAX_FRAME_PAYLOAD_BYTES {
                to_drop.push(id);
            }
        }

        for (id, line) in frames {
            self.log_wire("recv", id, &line);
            self.handle_line(id, &line);
        }

        if let Some(idle_timeout_ms) = self.config.idle_timeout_ms {
            let ids_for_timeout: Vec<usize> = self
                .clients
                .iter()
                .filter_map(|(id, client)| {
                    let timeout_applies =
                        !client.handshake_complete || client.role == ClientRole::Controller;
                    if timeout_applies
                        && now_ms.saturating_sub(client.last_read_at_ms) >= idle_timeout_ms
                    {
                        Some(*id)
                    } else {
                        None
                    }
                })
                .collect();
            to_drop.extend(ids_for_timeout);
        }

        to_drop.sort_unstable();
        to_drop.dedup();
        for id in to_drop {
            self.drop_client(id);
        }
    }

    fn flush_writes(&mut self) {
        let ids: Vec<usize> = self.clients.keys().copied().collect();
        let mut to_drop = Vec::new();
        for id in ids {
            let Some(client) = self.clients.get_mut(&id) else {
                continue;
            };
            if client.disconnect_requested {
                to_drop.push(id);
                continue;
            }
            while !client.write_buf.is_empty() {
                match client.stream.write(&client.write_buf) {
                    Ok(0) => {
                        to_drop.push(id);
                        break;
                    }
                    Ok(written) => {
                        client.write_buf.drain(..written);
                    }
                    Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => break,
                    Err(_) => {
                        to_drop.push(id);
                        break;
                    }
                }
            }
            if client.write_buf.is_empty() {
                flush_observation(client, &mut to_drop, id);
            }
        }
        to_drop.sort_unstable();
        to_drop.dedup();
        for id in to_drop {
            self.drop_client(id);
        }
    }

    fn handle_line(&mut self, connection_id: usize, line: &[u8]) {
        if line.is_empty() {
            self.send_error(connection_id, 0, "invalid_command", "Empty frame.");
            return;
        }
        let Some(msg) = decode_incoming(line) else {
            self.send_error(connection_id, 0, "invalid_command", "Failed to parse JSON.");
            return;
        };
        let seq = msg.seq();
        if !self.is_handshake_complete(connection_id) {
            match msg {
                InMessage::Hello(hello) => {
                    self.handle_hello(connection_id, hello);
                }
                _ => self.send_error(
                    connection_id,
                    seq,
                    "handshake_required",
                    "Send hello before commands.",
                ),
            }
            return;
        }
        if matches!(msg, InMessage::Hello(_)) {
            self.send_error(
                connection_id,
                seq,
                "invalid_command",
                "hello is only valid as the first message.",
            );
            return;
        }
        if !self.validate_monotonic_seq(connection_id, seq) {
            self.send_error(
                connection_id,
                seq,
                "invalid_command",
                "seq must increase monotonically.",
            );
            return;
        }
        match msg {
            InMessage::Hello(_) => {}
            InMessage::Command(command) => self.handle_command(connection_id, command),
            InMessage::Control(control) => self.handle_control(connection_id, control),
        }
    }

    fn validate_monotonic_seq(&mut self, connection_id: usize, seq: u64) -> bool {
        let Some(client) = self.clients.get_mut(&connection_id) else {
            return false;
        };
        if let Some(last) = client.last_seq
            && seq <= last
        {
            return false;
        }
        client.last_seq = Some(seq);
        true
    }

    fn handle_hello(&mut self, connection_id: usize, hello: HelloMessage) {
        if hello.seq != 1 || !hello.formats.iter().any(|format| format == "json") {
            self.send_error(
                connection_id,
                hello.seq,
                "invalid_command",
                "hello requires seq=1 and json format support.",
            );
            return;
        }
        if !compatible_protocol_version(&self.config.protocol_version, &hello.protocol_version) {
            self.send_error(
                connection_id,
                hello.seq,
                "protocol_mismatch",
                "Incompatible protocol version.",
            );
            return;
        }
        let assigned_role =
            if hello.requested.role != RequestedRole::Observer && self.controller_id.is_none() {
                self.controller_id = Some(connection_id);
                ClientRole::Controller
            } else {
                ClientRole::Observer
            };
        if let Some(client) = self.clients.get_mut(&connection_id) {
            client.handshake_complete = true;
            client.stream_observations = hello.requested.stream_observations;
            client.requested_mode = Some(hello.requested.command_mode);
            client.requested_role = hello.requested.role;
            client.role = assigned_role;
            client.last_seq = Some(hello.seq);
            client.needs_snapshot = hello.requested.stream_observations;
        }
        let welcome = OutMessage::Welcome(WelcomeMessage {
            r#type: "welcome",
            seq: hello.seq,
            ts: now_unix_ms(),
            protocol_version: self.config.protocol_version.clone(),
            client_id: connection_id,
            role: assigned_role.as_str(),
            controller_id: self.controller_id,
            game_id: self.config.game_id.clone(),
            capabilities: Capabilities {
                formats: vec!["json"],
                command_modes: vec!["action", "place"],
                features: vec![
                    "hold",
                    "next",
                    "next_queue",
                    "can_hold",
                    "ghost_y",
                    "board_id",
                    "events",
                    "logical_step",
                    "state_hash",
                    "score",
                    "timers",
                ],
                features_always: vec![
                    "next",
                    "next_queue",
                    "can_hold",
                    "board_id",
                    "events",
                    "logical_step",
                    "state_hash",
                    "score",
                    "timers",
                ],
                features_optional: vec!["hold", "ghost_y"],
                control_policy: ControlPolicy {
                    auto_promote_on_disconnect: true,
                    promotion_order: "lowest_client_id",
                },
            },
        });
        if let Some(data) = encode_message(&welcome) {
            self.send_line(connection_id, &data);
        }
    }

    fn handle_command(&mut self, connection_id: usize, command: CommandMessage) {
        let seq = command.seq;
        if !self.is_handshake_complete(connection_id) {
            self.send_error(
                connection_id,
                seq,
                "handshake_required",
                "Send hello before commands.",
            );
            return;
        }
        if self.controller_id != Some(connection_id) {
            self.send_error(
                connection_id,
                seq,
                "not_controller",
                "Only controller may send commands.",
            );
            return;
        }
        if self.pending_commands.len() >= self.config.max_pending_commands {
            self.send_error(connection_id, seq, "backpressure", "Command queue full.");
            return;
        }
        let parsed = match parse_protocol_command(command) {
            Ok(parsed) => parsed,
            Err((code, message)) => {
                self.send_error(connection_id, seq, code, message);
                return;
            }
        };
        self.pending_commands.push_back(PendingCommand {
            connection_id,
            seq,
            command: parsed,
        });
    }

    fn handle_control(&mut self, connection_id: usize, control: ControlMessage) {
        if !self.is_handshake_complete(connection_id) {
            self.send_error(
                connection_id,
                control.seq,
                "handshake_required",
                "Send hello before control.",
            );
            return;
        }

        match control.action {
            ControlAction::Claim => {
                let ok = if self.controller_id.is_none() {
                    self.controller_id = Some(connection_id);
                    if let Some(client) = self.clients.get_mut(&connection_id) {
                        client.role = ClientRole::Controller;
                    }
                    true
                } else {
                    self.controller_id == Some(connection_id)
                };
                if ok {
                    self.send_control_ack(connection_id, control.seq);
                } else {
                    self.send_error(
                        connection_id,
                        control.seq,
                        "controller_active",
                        "Controller already assigned.",
                    );
                }
            }
            ControlAction::Release => {
                if self.controller_id != Some(connection_id) {
                    self.send_error(
                        connection_id,
                        control.seq,
                        "not_controller",
                        "Only controller may release control.",
                    );
                    return;
                }
                self.controller_id = None;
                if let Some(client) = self.clients.get_mut(&connection_id) {
                    client.role = ClientRole::Observer;
                }
                self.send_control_ack(connection_id, control.seq);
            }
        }
    }

    fn is_handshake_complete(&self, connection_id: usize) -> bool {
        self.clients
            .get(&connection_id)
            .map(|client| client.handshake_complete)
            .unwrap_or(false)
    }

    fn send_control_ack(&mut self, connection_id: usize, seq: u64) {
        let msg = OutMessage::Ack(AckMessage {
            r#type: "ack",
            seq,
            ts: now_unix_ms(),
            status: "ok",
            correlation_seq: seq,
            applied_step: None,
            state_hash: None,
        });
        if let Some(data) = encode_message(&msg) {
            self.send_line(connection_id, &data);
        }
    }

    fn send_game_ack(&mut self, connection_id: usize, seq: u64, state: &GameState) {
        let hash = state_hash(state);
        let msg = OutMessage::Ack(AckMessage {
            r#type: "ack",
            seq,
            ts: now_unix_ms(),
            status: "ok",
            correlation_seq: seq,
            applied_step: Some(state.logical_step),
            state_hash: Some(&hash),
        });
        if let Some(data) = encode_message(&msg) {
            self.send_line(connection_id, &data);
        }
    }

    fn send_error(&mut self, connection_id: usize, seq: u64, code: &str, message: &str) {
        let msg = OutMessage::Error(ErrorMessage {
            r#type: "error",
            seq,
            ts: now_unix_ms(),
            code,
            message,
            retry_after_ms: (code == "backpressure").then_some(BACKPRESSURE_RETRY_AFTER_MS),
        });
        if let Some(data) = encode_message(&msg) {
            self.send_line(connection_id, &data);
        }
    }

    fn send_line(&mut self, connection_id: usize, payload: &[u8]) {
        let Some(client) = self.clients.get_mut(&connection_id) else {
            return;
        };
        if client.write_buf.len().saturating_add(payload.len() + 1) > MAX_RESPONSE_BUFFER_BYTES {
            client.disconnect_requested = true;
            return;
        }
        client.write_buf.extend_from_slice(payload);
        client.write_buf.push(b'\n');
        self.log_wire("send", connection_id, payload);
    }

    fn send_observation(&mut self, connection_id: usize, payload: &[u8]) {
        let Some(client) = self.clients.get_mut(&connection_id) else {
            return;
        };
        let mut frame = Vec::with_capacity(payload.len() + 1);
        frame.extend_from_slice(payload);
        frame.push(b'\n');
        if client.observation_buf.is_none() {
            client.observation_buf = Some(frame);
            client.observation_offset = 0;
        } else {
            client.next_observation = Some(frame);
        }
        self.log_wire("send", connection_id, payload);
    }

    fn drop_client(&mut self, connection_id: usize) {
        let was_controller = self.controller_id == Some(connection_id);
        self.clients.remove(&connection_id);
        if was_controller {
            self.controller_id = None;
            self.promote_observer(None);
        }
        self.pending_commands
            .retain(|pending| pending.connection_id != connection_id);
    }

    fn promote_observer(&mut self, excluding: Option<usize>) {
        if self.controller_id.is_some() {
            return;
        }
        let mut observers: Vec<usize> = self
            .clients
            .iter()
            .filter_map(|(id, client)| {
                if client.role == ClientRole::Observer
                    && client.requested_role != RequestedRole::Observer
                    && client.handshake_complete
                    && Some(*id) != excluding
                {
                    Some(*id)
                } else {
                    None
                }
            })
            .collect();
        observers.sort_unstable();
        let Some(controller_id) = observers.first().copied() else {
            return;
        };
        self.controller_id = Some(controller_id);
        if let Some(client) = self.clients.get_mut(&controller_id) {
            client.role = ClientRole::Controller;
        }
    }

    fn log_wire(&mut self, direction: &str, connection_id: usize, line: &[u8]) {
        let Some(file) = self.log_file.as_mut() else {
            return;
        };
        let mut line_text = String::from_utf8_lossy(line).to_string();
        if line_text.ends_with('\n') {
            line_text.pop();
        }
        let record = serde_json::json!({
            "ts_ms": now_unix_ms(),
            "direction": direction,
            "connection_id": connection_id,
            "line": line_text
        });
        if let Ok(data) = serde_json::to_vec(&record) {
            let _ = file.write_all(&data);
            let _ = file.write_all(b"\n");
        }
    }
}

fn compatible_protocol_version(server: &str, client: &str) -> bool {
    matches!(
        (Version::parse(server), Version::parse(client)),
        (Ok(server), Ok(client)) if server.major == client.major
    )
}

fn drain_lines(buffer: &mut Vec<u8>) -> Vec<Vec<u8>> {
    let mut lines = Vec::new();
    while let Some(pos) = buffer.iter().position(|byte| *byte == b'\n') {
        let mut line = buffer.drain(..=pos).collect::<Vec<u8>>();
        if line.last() == Some(&b'\n') {
            line.pop();
        }
        if line.last() == Some(&b'\r') {
            line.pop();
        }
        lines.push(line);
    }
    lines
}

fn flush_observation(client: &mut ClientState, to_drop: &mut Vec<usize>, id: usize) {
    loop {
        if client.observation_buf.is_none() {
            client.observation_buf = client.next_observation.take();
            client.observation_offset = 0;
        }
        let Some(frame) = client.observation_buf.as_ref() else {
            return;
        };
        match client.stream.write(&frame[client.observation_offset..]) {
            Ok(0) => {
                to_drop.push(id);
                return;
            }
            Ok(written) => {
                client.observation_offset += written;
                if client.observation_offset == frame.len() {
                    client.observation_buf = None;
                    client.observation_offset = 0;
                    continue;
                }
                return;
            }
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => return,
            Err(_) => {
                to_drop.push(id);
                return;
            }
        }
    }
}

fn now_unix_ms() -> u64 {
    let now = SystemTime::now();
    let duration = now
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_millis(0));
    duration.as_millis() as u64
}

fn open_log_file(path: Option<&str>) -> Option<File> {
    let path = path?;
    let resolved = if path == "auto" {
        format!("/tmp/tetris-ai-adapter-{}.jsonl", now_unix_ms())
    } else {
        path.to_string()
    };
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(resolved)
        .ok()
}

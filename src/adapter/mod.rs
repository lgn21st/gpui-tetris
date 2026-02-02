use crate::game::board::{BOARD_HEIGHT, BOARD_WIDTH};
use crate::game::input::GameAction;
use crate::game::pieces::{Rotation, Tetromino, TetrominoType};
use crate::game::state::{GameState, TSpinKind};
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, VecDeque};
use std::env;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_PORT: u16 = 7777;
const DEFAULT_PROTOCOL_VERSION: &str = "2.0.0";
const DEFAULT_GAME_ID: &str = "gpui-tetris";

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
        if env::var("TETRIS_AI_DISABLED").ok().as_deref() == Some("1") {
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

#[derive(Debug)]
struct ClientState {
    stream: TcpStream,
    read_buf: Vec<u8>,
    write_buf: Vec<u8>,
    handshake_complete: bool,
    stream_observations: bool,
    requested_mode: Option<CommandMode>,
    role: ClientRole,
    join_order: u64,
    last_seq: Option<u64>,
    last_read_at_ms: u64,
}

impl ClientState {
    fn new(stream: TcpStream, join_order: u64, now_ms: u64) -> Self {
        Self {
            stream,
            read_buf: Vec::with_capacity(4096),
            write_buf: Vec::with_capacity(1024),
            handshake_complete: false,
            stream_observations: false,
            requested_mode: None,
            role: ClientRole::Observer,
            join_order,
            last_seq: None,
            last_read_at_ms: now_ms,
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
    next_join_order: u64,
    out_seq: u64,
    pending_commands: VecDeque<PendingCommand>,
    last_observation_ts_ms: Option<u64>,
    log_file: Option<File>,
    episode_id: u64,
    piece_id: u64,
    step_in_piece: u64,
    last_active: Option<Tetromino>,
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
            next_join_order: 0,
            out_seq: 0,
            pending_commands: VecDeque::new(),
            last_observation_ts_ms: None,
            log_file,
            episode_id: 0,
            piece_id: 0,
            step_in_piece: 0,
            last_active: None,
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
            let result = map_command_to_actions(&pending.command, state)
                .map(|actions| {
                    for action in actions {
                        state.apply_action(action);
                        changed = true;
                    }
                })
                .map_err(|err| err.to_error());

            match result {
                Ok(()) => {
                    self.send_ack(pending.connection_id, pending.seq, "ok");
                }
                Err((code, message)) => {
                    self.send_error(pending.connection_id, pending.seq, code, message);
                }
            }
        }

        self.flush_writes();
        changed
    }

    pub fn emit_observation(&mut self, state: &GameState) {
        if self.clients.is_empty() {
            return;
        }
        let now_ms = now_unix_ms();
        if let Some(interval) = self.config.observation_interval_ms
            && let Some(last) = self.last_observation_ts_ms
            && now_ms.saturating_sub(last) < interval
        {
            return;
        }

        self.update_episode_and_piece_tracking(state);
        self.out_seq = self.out_seq.saturating_add(1);
        let message = OutMessage::Observation(Observation::from_state(
            self.out_seq,
            now_ms,
            self.episode_id,
            self.piece_id,
            self.step_in_piece,
            state,
        ));
        let Some(encoded) = encode_message(&message) else {
            return;
        };

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
        for id in targets {
            self.send_line(id, &encoded);
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
            let id = self.next_connection_id;
            self.next_connection_id = self.next_connection_id.saturating_add(1);
            let join_order = self.next_join_order;
            self.next_join_order = self.next_join_order.saturating_add(1);
            self.clients
                .insert(id, ClientState::new(stream, join_order, now_ms));
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
                frames.push((id, line));
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
                    if now_ms.saturating_sub(client.last_read_at_ms) >= idle_timeout_ms {
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
            InMessage::Hello(hello) => self.handle_hello(connection_id, hello),
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
        if !compatible_major_version(&self.config.protocol_version, &hello.protocol_version) {
            self.send_error(
                connection_id,
                hello.seq,
                "protocol_mismatch",
                "Incompatible protocol version.",
            );
            return;
        }
        let assigned_role = if self.controller_id.is_none() {
            self.controller_id = Some(connection_id);
            ClientRole::Controller
        } else {
            ClientRole::Observer
        };
        if let Some(client) = self.clients.get_mut(&connection_id) {
            client.handshake_complete = true;
            client.stream_observations = hello.requested.stream_observations;
            client.requested_mode = Some(hello.requested.command_mode);
            client.role = assigned_role;
        }
        let welcome = OutMessage::Welcome(WelcomeMessage {
            r#type: "welcome",
            seq: hello.seq,
            ts: now_unix_ms(),
            protocol_version: self.config.protocol_version.clone(),
            game_id: self.config.game_id.clone(),
            capabilities: Capabilities {
                formats: vec!["json"],
                command_modes: vec!["action", "place"],
                features: vec![
                    "hold",
                    "next",
                    "next_queue",
                    "can_hold",
                    "last_event",
                    "state_hash",
                    "score",
                    "timers",
                ],
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
        if let Some(client) = self.clients.get(&connection_id)
            && let Some(mode) = client.requested_mode
            && command.mode != mode
        {
            self.send_error(
                connection_id,
                seq,
                "invalid_command",
                "Unsupported command mode.",
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
                    self.send_ack(connection_id, control.seq, "ok");
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
                self.promote_observer(Some(connection_id));
                self.send_ack(connection_id, control.seq, "ok");
            }
        }
    }

    fn is_handshake_complete(&self, connection_id: usize) -> bool {
        self.clients
            .get(&connection_id)
            .map(|client| client.handshake_complete)
            .unwrap_or(false)
    }

    fn send_ack(&mut self, connection_id: usize, seq: u64, status: &str) {
        let msg = OutMessage::Ack(AckMessage {
            r#type: "ack",
            seq,
            ts: now_unix_ms(),
            status,
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
        });
        if let Some(data) = encode_message(&msg) {
            self.send_line(connection_id, &data);
        }
    }

    fn send_line(&mut self, connection_id: usize, payload: &[u8]) {
        let Some(client) = self.clients.get_mut(&connection_id) else {
            return;
        };
        client.write_buf.extend_from_slice(payload);
        client.write_buf.push(b'\n');
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
        let mut observers: Vec<(usize, u64)> = self
            .clients
            .iter()
            .filter_map(|(id, client)| {
                if client.role == ClientRole::Observer && Some(*id) != excluding {
                    Some((*id, client.join_order))
                } else {
                    None
                }
            })
            .collect();
        observers.sort_by_key(|entry| entry.1);
        let Some((controller_id, _)) = observers.first().copied() else {
            return;
        };
        self.controller_id = Some(controller_id);
        if let Some(client) = self.clients.get_mut(&controller_id) {
            client.role = ClientRole::Controller;
        }
    }

    fn update_episode_and_piece_tracking(&mut self, state: &GameState) {
        match self.last_active {
            None => {
                self.piece_id = 0;
                self.step_in_piece = 0;
                self.last_active = Some(state.active);
            }
            Some(last) => {
                if state.active == last {
                    self.step_in_piece = self.step_in_piece.saturating_add(1);
                } else {
                    self.piece_id = self.piece_id.saturating_add(1);
                    self.step_in_piece = 0;
                    self.last_active = Some(state.active);
                }
            }
        }
        if state.score == 0
            && state.lines == 0
            && state.level == 0
            && self.step_in_piece == 0
            && self.piece_id > 0
        {
            self.episode_id = self.episode_id.saturating_add(1);
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

#[derive(Clone, Debug)]
enum ProtocolCommand {
    Action(Vec<GameAction>),
    Place {
        x: i32,
        rotation: Rotation,
        use_hold: bool,
    },
}

#[derive(Debug)]
enum CommandMapError {
    HoldUnavailable,
    InvalidPlace,
}

impl CommandMapError {
    fn to_error(&self) -> (&'static str, &'static str) {
        match self {
            CommandMapError::HoldUnavailable => ("hold_unavailable", "Hold unavailable."),
            CommandMapError::InvalidPlace => ("invalid_place", "Invalid place command."),
        }
    }
}

fn map_command_to_actions(
    command: &ProtocolCommand,
    state: &GameState,
) -> Result<Vec<GameAction>, CommandMapError> {
    match command {
        ProtocolCommand::Action(actions) => Ok(actions.clone()),
        ProtocolCommand::Place {
            x,
            rotation,
            use_hold,
        } => {
            let mut working = state.clone();
            let mut actions = Vec::new();
            if *use_hold {
                if !working.can_hold {
                    return Err(CommandMapError::HoldUnavailable);
                }
                actions.push(GameAction::Hold);
                working.apply_action(GameAction::Hold);
            }
            let Some(mut plan) = plan_place_actions(&working, *x, *rotation) else {
                return Err(CommandMapError::InvalidPlace);
            };
            actions.append(&mut plan);
            actions.push(GameAction::HardDrop);
            Ok(actions)
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct PlanState {
    x: i32,
    y: i32,
    rotation: Rotation,
}

fn plan_place_actions(
    state: &GameState,
    target_x: i32,
    target_rotation: Rotation,
) -> Option<Vec<GameAction>> {
    let board = &state.board;
    let kind = state.active.kind;
    let start = PlanState {
        x: state.active.x,
        y: state.active.y,
        rotation: state.active.rotation,
    };
    if !can_place_plan(kind, board, start) {
        return None;
    }

    let mut frontier: BinaryHeap<(Reverse<u32>, PlanState)> = BinaryHeap::new();
    let mut cost: HashMap<PlanState, u32> = HashMap::new();
    let mut prev: HashMap<PlanState, (PlanState, GameAction)> = HashMap::new();
    frontier.push((Reverse(0), start));
    cost.insert(start, 0);

    while let Some((Reverse(curr_cost), curr)) = frontier.pop() {
        if curr.x == target_x && curr.rotation == target_rotation {
            return Some(reconstruct_actions(start, curr, &prev));
        }
        if let Some(known) = cost.get(&curr)
            && curr_cost > *known
        {
            continue;
        }
        for (action, next, step_cost) in neighbors(kind, board, curr) {
            let next_cost = curr_cost.saturating_add(step_cost);
            let best = cost.get(&next).copied().unwrap_or(u32::MAX);
            if next_cost < best {
                cost.insert(next, next_cost);
                prev.insert(next, (curr, action));
                frontier.push((Reverse(next_cost), next));
            }
        }
    }
    None
}

fn neighbors(
    kind: TetrominoType,
    board: &crate::game::board::Board,
    state: PlanState,
) -> Vec<(GameAction, PlanState, u32)> {
    let mut next = Vec::with_capacity(5);

    let left = PlanState {
        x: state.x - 1,
        y: state.y,
        rotation: state.rotation,
    };
    if can_place_plan(kind, board, left) {
        next.push((GameAction::MoveLeft, left, 1));
    }
    let right = PlanState {
        x: state.x + 1,
        y: state.y,
        rotation: state.rotation,
    };
    if can_place_plan(kind, board, right) {
        next.push((GameAction::MoveRight, right, 1));
    }
    let down = PlanState {
        x: state.x,
        y: state.y + 1,
        rotation: state.rotation,
    };
    if can_place_plan(kind, board, down) {
        next.push((GameAction::SoftDrop, down, 4));
    }

    if let Some(rot_cw) = rotate_plan(kind, board, state, true) {
        next.push((GameAction::RotateCw, rot_cw, 3));
    }
    if let Some(rot_ccw) = rotate_plan(kind, board, state, false) {
        next.push((GameAction::RotateCcw, rot_ccw, 3));
    }
    next
}

fn rotate_plan(
    kind: TetrominoType,
    board: &crate::game::board::Board,
    state: PlanState,
    clockwise: bool,
) -> Option<PlanState> {
    let next_rotation = if clockwise {
        state.rotation.cw()
    } else {
        state.rotation.ccw()
    };
    for (dx, dy) in srs_kicks(kind, state.rotation, next_rotation) {
        let candidate = PlanState {
            x: state.x + dx,
            y: state.y + dy,
            rotation: next_rotation,
        };
        if can_place_plan(kind, board, candidate) {
            return Some(candidate);
        }
    }
    None
}

fn can_place_plan(
    kind: TetrominoType,
    board: &crate::game::board::Board,
    state: PlanState,
) -> bool {
    let piece = Tetromino {
        kind,
        rotation: state.rotation,
        x: state.x,
        y: state.y,
    };
    for (dx, dy) in piece.blocks(state.rotation) {
        let nx = state.x + dx;
        let ny = state.y + dy;
        if nx < 0 || nx >= BOARD_WIDTH as i32 || ny < 0 || ny >= BOARD_HEIGHT as i32 {
            return false;
        }
        if board.cells[ny as usize][nx as usize].filled {
            return false;
        }
    }
    true
}

fn reconstruct_actions(
    start: PlanState,
    mut end: PlanState,
    prev: &HashMap<PlanState, (PlanState, GameAction)>,
) -> Vec<GameAction> {
    let mut actions = Vec::new();
    while end != start {
        let Some((parent, action)) = prev.get(&end) else {
            break;
        };
        actions.push(*action);
        end = *parent;
    }
    actions.reverse();
    actions
}

fn srs_kicks(kind: TetrominoType, from: Rotation, to: Rotation) -> &'static [(i32, i32); 5] {
    use Rotation::*;

    const JLSTZ_0_R: [(i32, i32); 5] = [(0, 0), (-1, 0), (-1, 1), (0, -2), (-1, -2)];
    const JLSTZ_R_0: [(i32, i32); 5] = [(0, 0), (1, 0), (1, -1), (0, 2), (1, 2)];
    const JLSTZ_R_2: [(i32, i32); 5] = [(0, 0), (1, 0), (1, -1), (0, 2), (1, 2)];
    const JLSTZ_2_R: [(i32, i32); 5] = [(0, 0), (-1, 0), (-1, 1), (0, -2), (-1, -2)];
    const JLSTZ_2_L: [(i32, i32); 5] = [(0, 0), (1, 0), (1, 1), (0, -2), (1, -2)];
    const JLSTZ_L_2: [(i32, i32); 5] = [(0, 0), (-1, 0), (-1, -1), (0, 2), (-1, 2)];
    const JLSTZ_L_0: [(i32, i32); 5] = [(0, 0), (-1, 0), (-1, -1), (0, 2), (-1, 2)];
    const JLSTZ_0_L: [(i32, i32); 5] = [(0, 0), (1, 0), (1, 1), (0, -2), (1, -2)];

    const I_0_R: [(i32, i32); 5] = [(0, 0), (-2, 0), (1, 0), (-2, -1), (1, 2)];
    const I_R_0: [(i32, i32); 5] = [(0, 0), (2, 0), (-1, 0), (2, 1), (-1, -2)];
    const I_R_2: [(i32, i32); 5] = [(0, 0), (-1, 0), (2, 0), (-1, 2), (2, -1)];
    const I_2_R: [(i32, i32); 5] = [(0, 0), (1, 0), (-2, 0), (1, -2), (-2, 1)];
    const I_2_L: [(i32, i32); 5] = [(0, 0), (2, 0), (-1, 0), (2, 1), (-1, -2)];
    const I_L_2: [(i32, i32); 5] = [(0, 0), (-2, 0), (1, 0), (-2, -1), (1, 2)];
    const I_L_0: [(i32, i32); 5] = [(0, 0), (1, 0), (-2, 0), (1, -2), (-2, 1)];
    const I_0_L: [(i32, i32); 5] = [(0, 0), (-1, 0), (2, 0), (-1, 2), (2, -1)];
    const O_KICKS: [(i32, i32); 5] = [(0, 0), (0, 0), (0, 0), (0, 0), (0, 0)];

    if kind == TetrominoType::O {
        return &O_KICKS;
    }
    match (kind, from, to) {
        (TetrominoType::I, North, East) => &I_0_R,
        (TetrominoType::I, East, North) => &I_R_0,
        (TetrominoType::I, East, South) => &I_R_2,
        (TetrominoType::I, South, East) => &I_2_R,
        (TetrominoType::I, South, West) => &I_2_L,
        (TetrominoType::I, West, South) => &I_L_2,
        (TetrominoType::I, West, North) => &I_L_0,
        (TetrominoType::I, North, West) => &I_0_L,
        (_, North, East) => &JLSTZ_0_R,
        (_, East, North) => &JLSTZ_R_0,
        (_, East, South) => &JLSTZ_R_2,
        (_, South, East) => &JLSTZ_2_R,
        (_, South, West) => &JLSTZ_2_L,
        (_, West, South) => &JLSTZ_L_2,
        (_, West, North) => &JLSTZ_L_0,
        (_, North, West) => &JLSTZ_0_L,
        _ => &JLSTZ_0_R,
    }
}

#[derive(Deserialize)]
struct EnvelopeType {
    #[serde(rename = "type")]
    message_type: String,
}

#[derive(Deserialize)]
struct HelloMessage {
    seq: u64,
    #[allow(dead_code)]
    ts: u64,
    protocol_version: String,
    requested: RequestedConfig,
}

#[derive(Deserialize)]
struct RequestedConfig {
    stream_observations: bool,
    command_mode: CommandMode,
}

#[derive(Deserialize)]
struct CommandMessage {
    seq: u64,
    #[allow(dead_code)]
    ts: u64,
    mode: CommandMode,
    actions: Option<Vec<ActionName>>,
    place: Option<PlacePayload>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum CommandMode {
    Action,
    Place,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
enum ActionName {
    MoveLeft,
    MoveRight,
    SoftDrop,
    HardDrop,
    RotateCw,
    RotateCcw,
    Hold,
    Pause,
    Restart,
}

#[derive(Deserialize)]
struct PlacePayload {
    x: i32,
    rotation: RotationValue,
    #[serde(rename = "useHold")]
    use_hold: bool,
}

#[derive(Clone, Copy, Debug)]
enum RotationValue {
    North,
    East,
    South,
    West,
}

impl RotationValue {
    fn to_rotation(self) -> Rotation {
        match self {
            RotationValue::North => Rotation::North,
            RotationValue::East => Rotation::East,
            RotationValue::South => Rotation::South,
            RotationValue::West => Rotation::West,
        }
    }
}

impl<'de> Deserialize<'de> for RotationValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        match value.to_ascii_lowercase().as_str() {
            "north" => Ok(Self::North),
            "east" => Ok(Self::East),
            "south" => Ok(Self::South),
            "west" => Ok(Self::West),
            _ => Err(serde::de::Error::custom("invalid rotation")),
        }
    }
}

#[derive(Deserialize)]
struct ControlMessage {
    seq: u64,
    #[allow(dead_code)]
    ts: u64,
    action: ControlAction,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum ControlAction {
    Claim,
    Release,
}

enum InMessage {
    Hello(HelloMessage),
    Command(CommandMessage),
    Control(ControlMessage),
}

impl InMessage {
    fn seq(&self) -> u64 {
        match self {
            InMessage::Hello(msg) => msg.seq,
            InMessage::Command(msg) => msg.seq,
            InMessage::Control(msg) => msg.seq,
        }
    }
}

fn decode_incoming(line: &[u8]) -> Option<InMessage> {
    let envelope: EnvelopeType = serde_json::from_slice(line).ok()?;
    match envelope.message_type.as_str() {
        "hello" => serde_json::from_slice::<HelloMessage>(line)
            .ok()
            .map(InMessage::Hello),
        "command" => serde_json::from_slice::<CommandMessage>(line)
            .ok()
            .map(InMessage::Command),
        "control" => serde_json::from_slice::<ControlMessage>(line)
            .ok()
            .map(InMessage::Control),
        _ => None,
    }
}

fn parse_protocol_command(
    command: CommandMessage,
) -> Result<ProtocolCommand, (&'static str, &'static str)> {
    match command.mode {
        CommandMode::Action => {
            let Some(actions) = command.actions else {
                return Err(("invalid_command", "Missing command payload."));
            };
            let mapped = actions
                .into_iter()
                .map(|action| match action {
                    ActionName::MoveLeft => GameAction::MoveLeft,
                    ActionName::MoveRight => GameAction::MoveRight,
                    ActionName::SoftDrop => GameAction::SoftDrop,
                    ActionName::HardDrop => GameAction::HardDrop,
                    ActionName::RotateCw => GameAction::RotateCw,
                    ActionName::RotateCcw => GameAction::RotateCcw,
                    ActionName::Hold => GameAction::Hold,
                    ActionName::Pause => GameAction::Pause,
                    ActionName::Restart => GameAction::Restart,
                })
                .collect();
            Ok(ProtocolCommand::Action(mapped))
        }
        CommandMode::Place => {
            let Some(place) = command.place else {
                return Err(("invalid_command", "Missing command payload."));
            };
            Ok(ProtocolCommand::Place {
                x: place.x,
                rotation: place.rotation.to_rotation(),
                use_hold: place.use_hold,
            })
        }
    }
}

#[derive(Serialize)]
struct Capabilities {
    formats: Vec<&'static str>,
    command_modes: Vec<&'static str>,
    features: Vec<&'static str>,
}

#[derive(Serialize)]
struct WelcomeMessage {
    #[serde(rename = "type")]
    r#type: &'static str,
    seq: u64,
    ts: u64,
    protocol_version: String,
    game_id: String,
    capabilities: Capabilities,
}

#[derive(Serialize)]
struct AckMessage<'a> {
    #[serde(rename = "type")]
    r#type: &'static str,
    seq: u64,
    ts: u64,
    status: &'a str,
}

#[derive(Serialize)]
struct ErrorMessage<'a> {
    #[serde(rename = "type")]
    r#type: &'static str,
    seq: u64,
    ts: u64,
    code: &'a str,
    message: &'a str,
}

#[derive(Serialize)]
struct Observation {
    #[serde(rename = "type")]
    r#type: &'static str,
    seq: u64,
    ts: u64,
    playable: bool,
    paused: bool,
    game_over: bool,
    episode_id: u64,
    seed: u64,
    piece_id: u64,
    step_in_piece: u64,
    board: ObservationBoard,
    active: ObservationActive,
    next: Option<PieceKind>,
    next_queue: Vec<PieceKind>,
    hold: Option<PieceKind>,
    can_hold: bool,
    last_event: LastEvent,
    state_hash: String,
    score: u32,
    level: u32,
    lines: u32,
    timers: Timers,
}

impl Observation {
    fn from_state(
        seq: u64,
        ts: u64,
        episode_id: u64,
        piece_id: u64,
        step_in_piece: u64,
        state: &GameState,
    ) -> Self {
        let board = ObservationBoard::from_state(state);
        let next_queue: Vec<PieceKind> = state
            .next_queue
            .iter()
            .copied()
            .map(PieceKind::from)
            .collect();
        let next = state.next_queue.first().copied().map(PieceKind::from);
        let hold = state.hold.map(PieceKind::from);
        Self {
            r#type: "observation",
            seq,
            ts,
            playable: !state.paused && !state.game_over,
            paused: state.paused,
            game_over: state.game_over,
            episode_id,
            seed: 1,
            piece_id,
            step_in_piece,
            board,
            active: ObservationActive::from_piece(state.active),
            next,
            next_queue,
            hold,
            can_hold: state.can_hold,
            last_event: LastEvent::from_state(state),
            state_hash: state_hash(state),
            score: state.score,
            level: state.level,
            lines: state.lines,
            timers: Timers {
                drop_ms: state.drop_timer_ms,
                lock_ms: state.lock_timer_ms,
                line_clear_ms: state.line_clear_timer_ms,
            },
        }
    }
}

#[derive(Serialize)]
struct ObservationBoard {
    width: usize,
    height: usize,
    cells: Vec<Vec<u8>>,
    kinds: Vec<Vec<Option<PieceKind>>>,
}

impl ObservationBoard {
    fn from_state(state: &GameState) -> Self {
        let mut cells = vec![vec![0_u8; BOARD_WIDTH]; BOARD_HEIGHT];
        let mut kinds = vec![vec![None; BOARD_WIDTH]; BOARD_HEIGHT];
        for (y, row) in state.board.cells.iter().enumerate() {
            for (x, cell) in row.iter().enumerate() {
                cells[y][x] = if cell.filled { 1 } else { 0 };
                kinds[y][x] = cell.kind.map(PieceKind::from);
            }
        }
        Self {
            width: BOARD_WIDTH,
            height: BOARD_HEIGHT,
            cells,
            kinds,
        }
    }
}

#[derive(Serialize)]
struct ObservationActive {
    kind: PieceKind,
    rotation: &'static str,
    x: i32,
    y: i32,
}

impl ObservationActive {
    fn from_piece(piece: Tetromino) -> Self {
        Self {
            kind: PieceKind::from(piece.kind),
            rotation: rotation_name(piece.rotation),
            x: piece.x,
            y: piece.y,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "UPPERCASE")]
enum PieceKind {
    I,
    O,
    T,
    S,
    Z,
    J,
    L,
}

impl From<TetrominoType> for PieceKind {
    fn from(kind: TetrominoType) -> Self {
        match kind {
            TetrominoType::I => PieceKind::I,
            TetrominoType::O => PieceKind::O,
            TetrominoType::T => PieceKind::T,
            TetrominoType::S => PieceKind::S,
            TetrominoType::Z => PieceKind::Z,
            TetrominoType::J => PieceKind::J,
            TetrominoType::L => PieceKind::L,
        }
    }
}

#[derive(Serialize)]
struct LastEvent {
    locked: bool,
    lines_cleared: u32,
    line_clear_score: u32,
    tspin: &'static str,
    combo: i32,
    back_to_back: bool,
}

impl LastEvent {
    fn from_state(state: &GameState) -> Self {
        let lines_cleared = if state.line_clear_timer_ms > 0 { 1 } else { 0 };
        Self {
            locked: state.landing_flash_timer_ms > 0,
            lines_cleared,
            line_clear_score: 0,
            tspin: tspin_name(TSpinKind::None),
            combo: state.combo,
            back_to_back: state.back_to_back,
        }
    }
}

#[derive(Serialize)]
struct Timers {
    drop_ms: u64,
    lock_ms: u64,
    line_clear_ms: u64,
}

enum OutMessage<'a> {
    Welcome(WelcomeMessage),
    Ack(AckMessage<'a>),
    Error(ErrorMessage<'a>),
    Observation(Observation),
}

fn encode_message(message: &OutMessage<'_>) -> Option<Vec<u8>> {
    match message {
        OutMessage::Welcome(msg) => serde_json::to_vec(msg).ok(),
        OutMessage::Ack(msg) => serde_json::to_vec(msg).ok(),
        OutMessage::Error(msg) => serde_json::to_vec(msg).ok(),
        OutMessage::Observation(msg) => serde_json::to_vec(msg).ok(),
    }
}

fn state_hash(state: &GameState) -> String {
    let mut hash: u64 = 14_695_981_039_346_656_037;
    fn mix_byte(hash: &mut u64, byte: u8) {
        *hash ^= u64::from(byte);
        *hash = hash.wrapping_mul(1_099_511_628_211);
    }
    fn mix_u64(hash: &mut u64, value: u64) {
        let bytes = value.to_le_bytes();
        for byte in bytes {
            mix_byte(hash, byte);
        }
    }

    mix_u64(&mut hash, state.score as u64);
    mix_u64(&mut hash, state.level as u64);
    mix_u64(&mut hash, state.lines as u64);
    mix_u64(&mut hash, state.drop_timer_ms);
    mix_u64(&mut hash, state.lock_timer_ms);
    mix_u64(&mut hash, state.line_clear_timer_ms);
    mix_u64(&mut hash, state.active.x as u64);
    mix_u64(&mut hash, state.active.y as u64);
    mix_byte(&mut hash, piece_kind_byte(state.active.kind));
    mix_byte(&mut hash, rotation_byte(state.active.rotation));
    if let Some(hold) = state.hold {
        mix_byte(&mut hash, piece_kind_byte(hold));
    } else {
        mix_byte(&mut hash, 0);
    }
    mix_byte(&mut hash, if state.can_hold { 1 } else { 0 });

    for kind in &state.next_queue {
        mix_byte(&mut hash, piece_kind_byte(*kind));
    }
    for row in &state.board.cells {
        for cell in row {
            mix_byte(&mut hash, if cell.filled { 1 } else { 0 });
            if let Some(kind) = cell.kind {
                mix_byte(&mut hash, piece_kind_byte(kind));
            } else {
                mix_byte(&mut hash, 0);
            }
        }
    }

    format!("{hash:016x}")
}

fn piece_kind_byte(kind: TetrominoType) -> u8 {
    match kind {
        TetrominoType::I => 1,
        TetrominoType::O => 2,
        TetrominoType::T => 3,
        TetrominoType::S => 4,
        TetrominoType::Z => 5,
        TetrominoType::J => 6,
        TetrominoType::L => 7,
    }
}

fn rotation_byte(rotation: Rotation) -> u8 {
    match rotation {
        Rotation::North => 1,
        Rotation::East => 2,
        Rotation::South => 3,
        Rotation::West => 4,
    }
}

fn rotation_name(rotation: Rotation) -> &'static str {
    match rotation {
        Rotation::North => "north",
        Rotation::East => "east",
        Rotation::South => "south",
        Rotation::West => "west",
    }
}

fn tspin_name(kind: TSpinKind) -> &'static str {
    match kind {
        TSpinKind::None => "none",
        TSpinKind::Mini => "mini",
        TSpinKind::Full => "full",
    }
}

fn compatible_major_version(server: &str, client: &str) -> bool {
    let parse_major = |value: &str| {
        value
            .split('.')
            .next()
            .and_then(|part| part.parse::<u64>().ok())
    };
    matches!((parse_major(server), parse_major(client)), (Some(a), Some(b)) if a == b)
}

fn drain_lines(buffer: &mut Vec<u8>) -> Vec<Vec<u8>> {
    let mut lines = Vec::new();
    loop {
        let Some(pos) = buffer.iter().position(|byte| *byte == b'\n') else {
            break;
        };
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::state::GameConfig;

    #[test]
    fn decode_rotation_accepts_lower_and_upper_case() {
        let json = br#"{"type":"command","seq":2,"ts":1,"mode":"place","place":{"x":3,"rotation":"EAST","useHold":false}}"#;
        let msg = decode_incoming(json).expect("decode");
        match msg {
            InMessage::Command(command) => {
                let parsed = parse_protocol_command(command).expect("parse");
                match parsed {
                    ProtocolCommand::Place { rotation, .. } => {
                        assert_eq!(rotation, Rotation::East);
                    }
                    _ => panic!("unexpected command"),
                }
            }
            _ => panic!("unexpected"),
        }
    }

    #[test]
    fn place_planner_finds_simple_path() {
        let state = GameState::new(1, GameConfig::default());
        let actions = plan_place_actions(&state, state.active.x + 1, state.active.rotation);
        assert!(actions.is_some());
        let actions = actions.unwrap();
        assert!(actions.contains(&GameAction::MoveRight));
    }

    #[test]
    fn state_hash_changes_when_board_changes() {
        let mut state = GameState::new(1, GameConfig::default());
        let before = state_hash(&state);
        state.board.cells[0][0] = crate::game::board::Cell {
            filled: true,
            kind: Some(TetrominoType::I),
        };
        let after = state_hash(&state);
        assert_ne!(before, after);
    }
}

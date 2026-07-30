mod planning;
mod transport;

use planning::plan_place_actions;

pub use transport::{AdapterConfig, SocketAdapter};

use crate::game::board::{BOARD_HEIGHT, BOARD_WIDTH};
use crate::game::input::GameAction;
use crate::game::pieces::{Rotation, Tetromino, TetrominoType};
use crate::game::state::{GameEvent, GameState, TSpinKind};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
enum ProtocolCommand {
    Action {
        actions: Vec<GameAction>,
        restart_seed: Option<u32>,
    },
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

fn apply_protocol_command(
    command: &ProtocolCommand,
    state: &mut GameState,
) -> Result<(), CommandMapError> {
    match command {
        ProtocolCommand::Action {
            actions,
            restart_seed,
        } => {
            for action in actions {
                if *action == GameAction::Restart {
                    if let Some(seed) = restart_seed {
                        state.reset_with_seed(u64::from(*seed));
                    } else {
                        state.apply_action(*action);
                    }
                } else {
                    state.apply_action(*action);
                }
            }
            Ok(())
        }
        ProtocolCommand::Place {
            x,
            rotation,
            use_hold,
        } => {
            if state.paused || state.game_over {
                return Err(CommandMapError::InvalidPlace);
            }
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
            for action in actions {
                working.apply_action(action);
            }
            *state = working;
            Ok(())
        }
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
    #[allow(dead_code)]
    client: ClientIdentity,
    protocol_version: String,
    formats: Vec<String>,
    requested: RequestedConfig,
}

#[derive(Deserialize)]
struct ClientIdentity {
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    version: String,
}

#[derive(Deserialize)]
struct RequestedConfig {
    stream_observations: bool,
    command_mode: CommandMode,
    #[serde(default)]
    role: RequestedRole,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum RequestedRole {
    #[default]
    Auto,
    Controller,
    Observer,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CommandMessage {
    #[serde(rename = "type")]
    _message_type: String,
    seq: u64,
    #[allow(dead_code)]
    ts: u64,
    mode: CommandMode,
    actions: Option<Vec<ActionName>>,
    place: Option<PlacePayload>,
    restart: Option<RestartPayload>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RestartPayload {
    seed: u32,
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
        match value.as_str() {
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
            if command.place.is_some() {
                return Err(("invalid_command", "Unexpected place payload."));
            }
            let Some(actions) = command.actions else {
                return Err(("invalid_command", "Missing command payload."));
            };
            if actions.len() > 32 {
                return Err(("invalid_command", "Too many actions."));
            }
            let has_restart = actions
                .iter()
                .any(|action| matches!(action, ActionName::Restart));
            if command.restart.is_some() && !has_restart {
                return Err((
                    "invalid_command",
                    "restart parameters require a restart action.",
                ));
            }
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
            Ok(ProtocolCommand::Action {
                actions: mapped,
                restart_seed: command.restart.map(|restart| restart.seed),
            })
        }
        CommandMode::Place => {
            if command.actions.is_some() || command.restart.is_some() {
                return Err(("invalid_command", "Unexpected action payload."));
            }
            let Some(place) = command.place else {
                return Err(("invalid_command", "Missing command payload."));
            };
            if !(-128..=127).contains(&place.x) {
                return Err(("invalid_command", "Place origin is outside schema range."));
            }
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
    features_always: Vec<&'static str>,
    features_optional: Vec<&'static str>,
    control_policy: ControlPolicy,
}

#[derive(Serialize)]
struct ControlPolicy {
    auto_promote_on_disconnect: bool,
    promotion_order: &'static str,
}

#[derive(Serialize)]
struct WelcomeMessage {
    #[serde(rename = "type")]
    r#type: &'static str,
    seq: u64,
    ts: u64,
    protocol_version: String,
    client_id: usize,
    role: &'static str,
    controller_id: Option<usize>,
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
    correlation_seq: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    applied_step: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    state_hash: Option<&'a str>,
}

#[derive(Serialize)]
struct ErrorMessage<'a> {
    #[serde(rename = "type")]
    r#type: &'static str,
    seq: u64,
    ts: u64,
    code: &'a str,
    message: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    retry_after_ms: Option<u64>,
}

#[derive(Serialize)]
struct Observation {
    #[serde(rename = "type")]
    r#type: &'static str,
    seq: u64,
    ts: u64,
    logical_step: u64,
    playable: bool,
    paused: bool,
    game_over: bool,
    episode_id: u64,
    seed: u64,
    piece_id: u64,
    step_in_piece: u64,
    board: ObservationBoard,
    board_id: u64,
    active: Option<ObservationActive>,
    ghost_y: Option<i32>,
    next: PieceKind,
    next_queue: Vec<PieceKind>,
    hold: Option<PieceKind>,
    can_hold: bool,
    events: Vec<ObservationEvent>,
    state_hash: String,
    score: u32,
    level: u32,
    lines: u32,
    timers: Timers,
}

impl Observation {
    fn from_state(seq: u64, ts: u64, state: &GameState, events: Vec<GameEvent>) -> Self {
        let board = ObservationBoard::from_state(state);
        let next_queue: Vec<PieceKind> = state
            .next_queue
            .iter()
            .copied()
            .map(PieceKind::from)
            .take(5)
            .collect();
        let next = PieceKind::from(state.next_queue[0]);
        let hold = state.hold.map(PieceKind::from);
        Self {
            r#type: "observation",
            seq,
            ts,
            logical_step: state.logical_step,
            playable: !state.paused && !state.game_over,
            paused: state.paused,
            game_over: state.game_over,
            episode_id: state.episode_id,
            seed: state.seed,
            piece_id: state.piece_id,
            step_in_piece: state.step_in_piece,
            board,
            board_id: state.board_revision(),
            active: (!state.game_over).then(|| ObservationActive::from_piece(state.active)),
            ghost_y: (!state.game_over).then(|| state.ghost_y()),
            next,
            next_queue,
            hold,
            can_hold: state.can_hold,
            events: events.into_iter().map(ObservationEvent::from).collect(),
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
                cells[y][x] = cell.kind.map(piece_kind_byte).unwrap_or(0);
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
#[serde(rename_all = "lowercase")]
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
struct ObservationEvent {
    locked: bool,
    lines_cleared: u32,
    line_clear_score: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    tspin: Option<&'static str>,
    combo: i32,
    back_to_back: bool,
}

impl From<GameEvent> for ObservationEvent {
    fn from(event: GameEvent) -> Self {
        Self {
            locked: event.locked,
            lines_cleared: u32::from(event.lines_cleared),
            line_clear_score: event.line_clear_score,
            tspin: match event.t_spin {
                TSpinKind::None => None,
                other => Some(tspin_name(other)),
            },
            combo: event.combo,
            back_to_back: event.back_to_back,
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
    mix_u64(&mut hash, state.seed);
    mix_u64(&mut hash, state.episode_id);
    mix_u64(&mut hash, state.piece_id);
    mix_u64(&mut hash, state.step_in_piece);
    mix_u64(&mut hash, state.logical_step);
    mix_u64(&mut hash, state.level as u64);
    mix_u64(&mut hash, state.lines as u64);
    mix_u64(&mut hash, state.combo as u64);
    mix_byte(&mut hash, u8::from(state.back_to_back));
    mix_byte(&mut hash, u8::from(state.is_classic_ruleset()));
    mix_u64(&mut hash, state.drop_timer_ms);
    mix_u64(&mut hash, state.lock_timer_ms);
    mix_u64(&mut hash, state.line_clear_timer_ms);
    mix_u64(&mut hash, state.soft_drop_timeout_ms);
    mix_u64(&mut hash, state.lock_reset_count as u64);
    mix_u64(&mut hash, state.board_revision());
    mix_byte(&mut hash, u8::from(state.paused));
    mix_byte(&mut hash, u8::from(state.game_over));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::state::GameConfig;

    #[test]
    fn decode_rotation_requires_schema_case() {
        let json = br#"{"type":"command","seq":2,"ts":1,"mode":"place","place":{"x":3,"rotation":"EAST","useHold":false}}"#;
        assert!(decode_incoming(json).is_none());

        let json = br#"{"type":"command","seq":2,"ts":1,"mode":"place","place":{"x":3,"rotation":"east","useHold":false}}"#;
        let msg = decode_incoming(json).expect("decode lowercase rotation");
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

    #[test]
    fn state_hash_covers_logical_and_scoring_identity() {
        let mut state = GameState::new(1, GameConfig::default());
        let before = state_hash(&state);
        state.logical_step += 1;
        assert_ne!(before, state_hash(&state));

        let before = state_hash(&state);
        state.combo += 1;
        assert_ne!(before, state_hash(&state));
    }
}

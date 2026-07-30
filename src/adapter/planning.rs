use crate::game::board::Board;
use crate::game::input::GameAction;
use crate::game::pieces::{Rotation, Tetromino, TetrominoType};
use crate::game::state::GameState;
use crate::game::state::kicks::srs_kicks;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct PlanState {
    x: i32,
    y: i32,
    rotation: Rotation,
}

pub(super) fn plan_place_actions(
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
    board: &Board,
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
    board: &Board,
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

fn can_place_plan(kind: TetrominoType, board: &Board, state: PlanState) -> bool {
    let piece = Tetromino {
        kind,
        rotation: state.rotation,
        x: state.x,
        y: state.y,
    };
    board.can_place(&piece, state.x, state.y, state.rotation)
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

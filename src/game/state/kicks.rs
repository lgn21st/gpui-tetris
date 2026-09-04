use crate::game::pieces::{Rotation, TetrominoType};

pub(crate) fn srs_kicks(
    kind: TetrominoType,
    from: Rotation,
    to: Rotation,
) -> &'static [(i32, i32); 5] {
    use Rotation::*;

    const JLSTZ_0_R: [(i32, i32); 5] = [(0, 0), (-1, 0), (-1, -1), (0, 2), (-1, 2)];
    const JLSTZ_R_0: [(i32, i32); 5] = [(0, 0), (1, 0), (1, 1), (0, -2), (1, -2)];
    const JLSTZ_R_2: [(i32, i32); 5] = [(0, 0), (1, 0), (1, 1), (0, -2), (1, -2)];
    const JLSTZ_2_R: [(i32, i32); 5] = [(0, 0), (-1, 0), (-1, -1), (0, 2), (-1, 2)];
    const JLSTZ_2_L: [(i32, i32); 5] = [(0, 0), (1, 0), (1, -1), (0, 2), (1, 2)];
    const JLSTZ_L_2: [(i32, i32); 5] = [(0, 0), (-1, 0), (-1, 1), (0, -2), (-1, -2)];
    const JLSTZ_L_0: [(i32, i32); 5] = [(0, 0), (-1, 0), (-1, 1), (0, -2), (-1, -2)];
    const JLSTZ_0_L: [(i32, i32); 5] = [(0, 0), (1, 0), (1, -1), (0, 2), (1, 2)];

    const I_0_R: [(i32, i32); 5] = [(0, 0), (-2, 0), (1, 0), (-2, 1), (1, -2)];
    const I_R_0: [(i32, i32); 5] = [(0, 0), (2, 0), (-1, 0), (2, -1), (-1, 2)];
    const I_R_2: [(i32, i32); 5] = [(0, 0), (-1, 0), (2, 0), (-1, -2), (2, 1)];
    const I_2_R: [(i32, i32); 5] = [(0, 0), (1, 0), (-2, 0), (1, 2), (-2, -1)];
    const I_2_L: [(i32, i32); 5] = [(0, 0), (2, 0), (-1, 0), (2, -1), (-1, 2)];
    const I_L_2: [(i32, i32); 5] = [(0, 0), (-2, 0), (1, 0), (-2, 1), (1, -2)];
    const I_L_0: [(i32, i32); 5] = [(0, 0), (1, 0), (-2, 0), (1, 2), (-2, -1)];
    const I_0_L: [(i32, i32); 5] = [(0, 0), (-1, 0), (2, 0), (-1, -2), (2, 1)];
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

/// Resolve a rotation using the same ordered kick candidates for play and planning.
pub(crate) fn rotated_piece(
    board: &crate::game::board::Board,
    piece: crate::game::pieces::Tetromino,
    clockwise: bool,
) -> Option<crate::game::pieces::Tetromino> {
    let rotation = if clockwise {
        piece.rotation.cw()
    } else {
        piece.rotation.ccw()
    };
    srs_kicks(piece.kind, piece.rotation, rotation)
        .iter()
        .find_map(|(dx, dy)| {
            let next = crate::game::pieces::Tetromino {
                x: piece.x + dx,
                y: piece.y + dy,
                rotation,
                ..piece
            };
            board
                .can_place(&next, next.x, next.y, next.rotation)
                .then_some(next)
        })
}

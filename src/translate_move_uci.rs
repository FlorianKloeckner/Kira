use crate::{Color, Piece};
use crate::Move;
use crate::MoveType;
use crate::board::Board;


fn square_to_algebraic(square: u8) -> String {
    let file = (b'a' + (square % 8)) as char;
    let rank = (b'1' + (square / 8)) as char;
    format!("{file}{rank}")
}

fn algebraic_to_square(s: &str) -> Option<u8> {
    let mut chars = s.chars();
    let file = chars.next()?;
    let rank = chars.next()?;
    if !('a'..='h').contains(&file) || !('1'..='8').contains(&rank) {
        return None;
    }
    let file_idx = file as u8 - b'a';
    let rank_idx = rank as u8 - b'1';
    Some(rank_idx * 8 + file_idx)
}

fn piece_to_promo_char(piece: Piece) -> char {
    match piece {
        Piece::WQueen | Piece::BQueen => 'q',
        Piece::WRook | Piece::BRook => 'r',
        Piece::WBishop | Piece::BBishop => 'b',
        Piece::WKnight | Piece::BKnight => 'n',
        _ => unreachable!("only Q/R/B/N are legal promotion pieces"),
    }
}

/// Builds the correctly-colored promoted piece — the UCI char alone
/// ('q'/'r'/'b'/'n') doesn't carry color, so the mover's color is needed.
fn promo_char_to_piece(c: char, color: Color) -> Option<Piece> {
    match (c, color) {
        ('q', Color::White) => Some(Piece::WQueen),
        ('q', Color::Black) => Some(Piece::BQueen),
        ('r', Color::White) => Some(Piece::WRook),
        ('r', Color::Black) => Some(Piece::BRook),
        ('b', Color::White) => Some(Piece::WBishop),
        ('b', Color::Black) => Some(Piece::BBishop),
        ('n', Color::White) => Some(Piece::WKnight),
        ('n', Color::Black) => Some(Piece::BKnight),
        _ => None,
    }
}

/// Move -> UCI string, e.g. "e2e4", "e1g1", "e7e8q".
/// Castling/en-passant/double-step all serialize the same as a plain
/// from+to; the move_type only matters for round-tripping back into a Move.
pub fn move_to_uci(mv: &Move) -> String {
    let mut s = format!("{}{}", square_to_algebraic(mv.from), square_to_algebraic(mv.to));
    if let MoveType::Promotion(piece) = mv.move_type {
        s.push(piece_to_promo_char(piece));
    }
    s
}

/// UCI string -> Move, using the board (before the move is applied) to
/// figure out which MoveType this actually is.
pub fn uci_to_move(uci: &str, board: &Board) -> Option<Move> {
    if uci.len() != 4 && uci.len() != 5 {
        return None;
    }

    let from = algebraic_to_square(&uci[0..2])?;
    let to = algebraic_to_square(&uci[2..4])?;

    let piece = board.pieces[from as usize];
    if piece == Piece::Empty {
        eprintln!("Invalid move, from square was empty");
        return None;
    }
    let color = piece.color()?;

    let promotion = if uci.len() == 5 {
        Some(promo_char_to_piece(uci.chars().nth(4)?, color)?)
    } else {
        None
    };

    let from_file = from % 8;
    let to_file = to % 8;
    let from_rank = from / 8;
    let to_rank = to / 8;

    let is_king = matches!(piece, Piece::WKing | Piece::BKing);
    let is_pawn = matches!(piece, Piece::WPawn | Piece::BPawn);

    let move_type = if let Some(promo_piece) = promotion {
        MoveType::Promotion(promo_piece)
    } else if is_king && (to_file as i8 - from_file as i8).abs() == 2 {
        if to_file > from_file {
            MoveType::CastleKingside
        } else {
            MoveType::CastleQueenside
        }
    } else if is_pawn && from_file != to_file && board.pieces[to as usize] == Piece::Empty {
        // Diagonal pawn move onto an empty square = en passant capture.
        MoveType::EnPassant
    } else if is_pawn && (to_rank as i8 - from_rank as i8).abs() == 2 {
        MoveType::PawnDoubleStep
    } else {
        MoveType::Normal
    };

    Some(Move { from, to, move_type })
}
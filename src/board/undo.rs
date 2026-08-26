use crate::Piece;
use crate::Move;
pub struct Undo {
    pub captured_piece: Piece,
    pub old_last_move: Move,
    pub old_white_queen_castling_possible: bool,
    pub old_white_king_castling_possible: bool,
    pub old_black_king_castling_possible: bool,
    pub old_black_queen_castling_possible: bool,
    //rest of the information we can get from the move that we are supposed to unmake
}
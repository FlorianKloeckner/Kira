use crate::Piece;
pub struct Undo {
    pub captured_piece: Piece,
    pub old_en_passant_target_square: Option<u8>,
    pub old_white_queen_castling_possible: bool,
    pub old_white_king_castling_possible: bool,
    pub old_black_king_castling_possible: bool,
    pub old_black_queen_castling_possible: bool,
    //rest of the information we can get from the move that we are supposed to unmake
}
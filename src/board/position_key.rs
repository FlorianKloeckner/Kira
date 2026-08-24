use crate::Move;
use crate::piece::Piece;
use crate::Color;


#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct PositionKey {
    pub pieces: [Piece; 64], 
    pub side_to_move: Color, 
    pub white_king_castling: bool, 
    pub white_queen_castling: bool,
    pub black_king_castling: bool,
    pub black_queen_castling: bool,
}
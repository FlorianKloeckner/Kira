use std::fmt;
use crate::Color;

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum Piece {
    Empty,
    WKing,
    WQueen, 
    WBishop,
    WKnight,
    WRook,
    WPawn,
    BKing,
    BQueen,
    BBishop,
    BKnight,
    BRook,
    BPawn,
}

impl Piece {
    pub fn color(&self) -> Option<Color>{
        match self {
            Piece::Empty => None,
            Piece::WKing|Piece::WQueen|Piece::WRook|Piece::WBishop|Piece::WKnight|Piece::WPawn  => Some(Color::White), //dangerous, always check if Piece is empty beforehand
            Piece::BKing|Piece::BQueen|Piece::BBishop|Piece::BRook|Piece::BKnight|Piece::BPawn => Some(Color::Black),
        }
    }
    pub fn score(&self) -> isize {
        match self {
            Piece::Empty|Piece::WKing|Piece::BKing => 0, 
            Piece::WPawn => 1, 
            Piece::WBishop|Piece::WKnight => 3,
            Piece::WRook => 5,
            Piece::WQueen => 9,
            Piece::BPawn => -1, 
            Piece::BBishop|Piece::BKnight => -3, 
            Piece::BRook => -5,
            Piece::BQueen => -9,  
        }
    }
}

impl fmt::Display for Piece {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let symbol = match self {
            Piece::Empty => '.',
            Piece::BKing  => '♔',
            Piece::BQueen => '♕',
            Piece::BBishop => '♗',
            Piece::BKnight => '♘',
            Piece::BRook  => '♖',
            Piece::BPawn  => '♙',
            Piece::WKing  => '♚',
            Piece::WQueen => '♛',
            Piece::WBishop => '♝',
            Piece::WKnight => '♞',
            Piece::WRook  => '♜',
            Piece::WPawn  => '♟',
        };

        write!(f, "{}", symbol)
    }
}
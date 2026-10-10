
use core::panic;
use std::fmt;
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use crate::piece::Piece;
use crate::Move;
use crate::Color;
use crate::MoveType;
use crate::offset_square;
use crate::get_rank;
use anyhow::Error;
use arrayvec::ArrayVec;
use undo::Undo;

pub mod position_key;
mod undo;


const fn flip(table: [isize; 64]) -> [isize; 64] {
    let mut flipped = [0; 64];
    let mut i = 0;
    while i < 64 {
        let rank = i / 8;
        let file = i % 8;
        let flipped_rank = 7 - rank;
        flipped[flipped_rank * 8 + file] = table[i];
        i += 1;
    }
    flipped
}

// PAWNS
const W_PAWN_EVAL: [isize; 64] = [
     0,  0,  0,  0,  0,  0,  0,  0,  
     5, 10, 10,-20,-20, 10, 10,  5, 
     5, -5,-10,  0,  0,-10, -5,  5, 
     0,  0,  0, 20, 20,  0,  0,  0, 
     5,  5, 10, 25, 25, 10,  5,  5, 
    10, 10, 20, 30, 30, 20, 10, 10, 
    50, 50, 50, 50, 50, 50, 50, 50, 
     0,  0,  0,  0,  0,  0,  0,  0  
];
const B_PAWN_EVAL: [isize; 64] = flip(W_PAWN_EVAL);

// KNIGHTS
const W_KNIGHT_EVAL: [isize; 64] = [
    -50,-40,-30,-30,-30,-30,-40,-50,
    -40,-20,  0,  5,  5,  0,-20,-40,
    -30,  5, 10, 15, 15, 10,  5,-30,
    -30,  0, 15, 20, 20, 15,  0,-30,
    -30,  5, 15, 20, 20, 15,  5,-30,
    -30,  0, 10, 15, 15, 10,  0,-30,
    -40,-20,  0,  0,  0,  0,-20,-40,
    -50,-40,-30,-30,-30,-30,-40,-50
];
const B_KNIGHT_EVAL: [isize; 64] = flip(W_KNIGHT_EVAL);

// BISHOPS
const W_BISHOP_EVAL: [isize; 64] = [
    -20,-10,-10,-10,-10,-10,-10,-20,
    -10,  5,  0,  0,  0,  0,  5,-10,
    -10, 10, 10, 10, 10, 10, 10,-10,
    -10,  0, 10, 10, 10, 10,  0,-10,
    -10,  5,  5, 10, 10,  5,  5,-10,
    -10,  0,  5, 10, 10,  5,  0,-10,
    -10,  0,  0,  0,  0,  0,  0,-10,
    -20,-10,-10,-10,-10,-10,-10,-20
];
const B_BISHOP_EVAL: [isize; 64] = flip(W_BISHOP_EVAL);

// ROOKS
const W_ROOK_EVAL: [isize; 64] = [
     0,  0,  0,  5,  5,  0,  0,  0,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
     5, 10, 10, 10, 10, 10, 10,  5,
     0,  0,  0,  0,  0,  0,  0,  0
];
const B_ROOK_EVAL: [isize; 64] = flip(W_ROOK_EVAL);

// QUEENS
const W_QUEEN_EVAL: [isize; 64] = [
    -20,-10,-10, -5, -5,-10,-10,-20,
    -10,  0,  5,  0,  0,  0,  0,-10,
    -10,  5,  5,  5,  5,  5,  0,-10,
      0,  0,  5,  5,  5,  5,  0, -5,
     -5,  0,  5,  5,  5,  5,  0, -5,
    -10,  0,  5,  5,  5,  5,  0,-10,
    -10,  0,  0,  0,  0,  0,  0,-10,
    -20,-10,-10, -5, -5,-10,-10,-20
];
const B_QUEEN_EVAL: [isize; 64] = flip(W_QUEEN_EVAL);

// KING MIDDLEGAME
const W_KING_MG_EVAL: [isize; 64] = [
     20, 30, 10,  0,  0, 10, 30, 20, // Rank 1: Strongly favor G1/C1 for castling
     20, 20,  0,  0,  0,  0, 20, 20,
    -10,-20,-20,-20,-20,-20,-20,-10,
    -20,-30,-30,-40,-40,-30,-30,-20,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30
];
const B_KING_MG_EVAL: [isize; 64] = flip(W_KING_MG_EVAL);

// KING ENDGAME
const W_KING_EG_EVAL: [isize; 64] = [
    -50,-30,-30,-30,-30,-30,-30,-50, // Rank 1: Penalize sitting on the back rank
    -30,-30,  0,  0,  0,  0,-30,-30,
    -30,-10, 20, 30, 30, 20,-10,-30,
    -30,-10, 30, 40, 40, 30,-10,-30,
    -30,-10, 30, 40, 40, 30,-10,-30,
    -30,-10, 20, 30, 30, 20,-10,-30,
    -30,-20,-10,  0,  0,-10,-20,-30,
    -50,-40,-30,-20,-20,-30,-40,-50
];
const B_KING_EG_EVAL: [isize; 64] = flip(W_KING_EG_EVAL);

pub struct Context {
    pub nodes: usize,
    pub start_time: Duration
}


#[derive(Clone, PartialEq, Debug)]
pub struct Board {
    pub pieces: [Piece; 64], 
    pub en_passant_target_square: Option<u8>,
    pub side_to_move:Color, 
    pub white_queen_castling_possible: bool,
    pub white_king_castling_possible: bool,
    pub black_queen_castling_possible: bool,
    pub black_king_castling_possible: bool,
    pub position_history_hashed: Vec<u64>,

}

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for rank in (0..8).rev() {
            write!(f, "{}", rank+1)?;

            for file in 0..8 {
                let index = rank*8 + file;
                write!(f,"{}", self.pieces[index])?;
                write!(f, " ")?;
            }

            writeln!(f)?;
        }

        writeln!(f, " a b c d e f g h")
    }
}

impl Board {
        
    const INF:isize = 100_000;
    pub fn make_move(&mut self, m: &Move) -> Undo{

        let captured_piece = match m.move_type {
            MoveType::EnPassant => {
                match self.side_to_move {
                    Color::White => self.pieces[(m.to - 8) as usize],
                    Color::Black => self.pieces[(m.to + 8) as usize],
                }
            }
            _ => self.pieces[m.to as usize],
        };
        let undo = Undo {
            captured_piece,
            old_black_king_castling_possible: self.black_king_castling_possible,
            old_black_queen_castling_possible: self.black_queen_castling_possible,
            old_white_king_castling_possible: self.white_king_castling_possible,
            old_white_queen_castling_possible: self.white_queen_castling_possible,
            old_en_passant_target_square: self.en_passant_target_square,
        };

        let piece = self.pieces[m.from as usize];
        let from = m.from as usize;
        let to = m.to as usize;
        match m.move_type {
            MoveType::Normal => {
                self.pieces[to] = piece;
                self.pieces[from ] = Piece::Empty;}
            MoveType::PawnDoubleStep => {
                self.pieces[to] = piece;
                self.pieces[from] = Piece::Empty;
                match self.side_to_move {
                    Color::White => self.en_passant_target_square = Some((to - 8) as u8),
                    Color::Black => self.en_passant_target_square = Some((to + 8) as u8), 
                }
            }
            MoveType::CastleKingside => {
                self.pieces[to ] = piece;
                self.pieces[from ] = Piece::Empty;
                self.pieces[from+1] = self.pieces[to +1];
                self.pieces[to + 1]  = Piece::Empty;}
            MoveType::CastleQueenside => {
                self.pieces[to ] = piece;
                self.pieces[from ] = Piece::Empty;
                self.pieces[from -1] = self.pieces[from-4];
                self.pieces[from -4] = Piece::Empty;}
            MoveType::Promotion(promotion_piece) => {
                self.pieces[to] = promotion_piece;
                self.pieces[from] = Piece::Empty}
            MoveType::EnPassant => {
                self.pieces[to] = self.pieces[from]; 
                self.pieces[from] = Piece::Empty;
                match self.side_to_move {
                    Color::White => self.pieces[to-8] = Piece::Empty,
                    Color::Black => self.pieces[to+8] = Piece::Empty,
                }
            }
        }
        if m.move_type != MoveType::PawnDoubleStep {self.en_passant_target_square = None;}
        self.set_castling_rights(m); 
        //self.add_current_position_to_history();
        self.side_to_move = self.side_to_move.invert();
        undo

    }
    
    pub fn unmake_move(&mut self, m: &Move, undo: Undo) {
        self.black_king_castling_possible = undo.old_black_king_castling_possible;
        self.black_queen_castling_possible = undo.old_black_queen_castling_possible;
        self.white_king_castling_possible = undo.old_white_king_castling_possible;
        self.white_queen_castling_possible = undo.old_white_queen_castling_possible;
        self.en_passant_target_square = undo.old_en_passant_target_square;
        self.side_to_move = self.side_to_move.invert(); // at the end of the done move, we gave over control to the other player
        let from = m.from as usize;
        let to = m.to as usize;
        match m.move_type {
            MoveType::Normal => {
                self.pieces[from] = self.pieces[to];
                self.pieces[to] = undo.captured_piece;
            }
            MoveType::PawnDoubleStep => {
                self.pieces[from] = self.pieces[to];
                assert_eq!(self.pieces[to], self.side_to_move.pawn());
                self.pieces[to] = Piece::Empty;

            }
            MoveType::Promotion(_) => {
                self.pieces[from] = self.side_to_move.pawn();
                self.pieces[to] = undo.captured_piece;
            } 
            MoveType::EnPassant => {
                self.pieces[from] = self.pieces[to];
                self.pieces[to] = Piece::Empty;
                //match color
                match self.side_to_move {
                    Color::White => self.pieces[to-8] = Color::Black.pawn(),
                    Color::Black => self.pieces[to+8] = Color::White.pawn(),
                }
            }
            MoveType::CastleKingside => {
                self.pieces[from] = self.pieces[to];
                self.pieces[to] = Piece::Empty;
                self.pieces[to+1] = self.pieces[from+1];
                self.pieces[from+1] = Piece::Empty;
            }
            MoveType::CastleQueenside => {
                self.pieces[from] = self.pieces[to];
                self.pieces[to] = Piece::Empty;
                self.pieces[from -4] = self.pieces[from -1];
                self.pieces[from -1] = Piece::Empty;
            }
        }
        //self.position_history_hashed.pop();
    }
 

    pub fn generate_possible_moves(&self) -> ArrayVec<Move, 218> {
        let mut possible_moves = ArrayVec::<Move, 218>::new();
        for (i, piece) in self.pieces.iter().enumerate() {
            let square: u8 = i.try_into().unwrap();
            let color = self.side_to_move;
            match color {
                Color::White => {
                    match piece {
                        Piece::WKing => 
                            self.generate_king_moves(square, &mut possible_moves),
                        Piece::WQueen => 
                            self.generate_queen_moves(square, &mut possible_moves),
                        Piece::WBishop =>
                            self.generate_bishop_moves(square, &mut possible_moves), 
                        Piece::WKnight => 
                            self.generate_knight_moves(square, &mut possible_moves),
                        Piece::WRook => 
                            self.generate_rook_moves(square, &mut possible_moves),
                        Piece::WPawn =>
                            self.generate_pawn_moves(square, &mut possible_moves),
                        _ => continue
                    }
                }
                Color::Black => {
                    match piece {
                        Piece::BKing => 
                            self.generate_king_moves(square, &mut possible_moves),
                        Piece::BQueen => 
                            self.generate_queen_moves(square, &mut possible_moves),
                        Piece::BBishop =>
                            self.generate_bishop_moves(square, &mut possible_moves), 
                        Piece::BKnight => 
                            self.generate_knight_moves(square, &mut possible_moves),
                        Piece::BRook => 
                            self.generate_rook_moves(square, &mut possible_moves),
                        Piece::BPawn =>
                            self.generate_pawn_moves(square, &mut possible_moves),
                        _ => continue
                    }
                }
                

            }
        }

        let mut legal_moves = ArrayVec::<Move, 218>::new();
        //test pseudo-move to see if it is check
        for m in possible_moves {
            let mut test_board = self.clone();
            test_board.make_move(&m);
            if !test_board.is_in_check(self.side_to_move) {
                legal_moves.push(m);
            }
        }
        legal_moves
    }
    fn has_move(&self) -> bool {
        for (i, piece) in self.pieces.iter().enumerate() {
            let square: u8 = i.try_into().unwrap();
            let mut possible_moves = ArrayVec::<Move, 218>::new();

            match piece {
                Piece::WKing|Piece::BKing => {
                    self.generate_king_moves(square, &mut possible_moves);
                    if !possible_moves.is_empty() {return true;}
                }
                Piece::WQueen|Piece::BQueen => {
                    self.generate_queen_moves(square, &mut possible_moves);
                    if !possible_moves.is_empty() {return true;}
                }
                Piece::WBishop|Piece::BBishop =>{
                    self.generate_bishop_moves(square, &mut possible_moves);
                    if !possible_moves.is_empty() {return true;}
                }
                Piece::WKnight|Piece::BKnight => {
                    self.generate_knight_moves(square, &mut possible_moves);
                    if !possible_moves.is_empty() {return true;}
                }
                Piece::WRook|Piece::BRook => {
                    self.generate_rook_moves(square, &mut possible_moves);
                    if !possible_moves.is_empty() {return true;}
                }
                Piece::WPawn|Piece::BPawn =>{
                    self.generate_pawn_moves(square, &mut possible_moves);
                    if !possible_moves.is_empty() {return true;}
                }
                _ => continue
            }
        }
        false
    }
    

    fn generate_pawn_moves(&self, square: u8, possible_moves: &mut ArrayVec<Move, 218>){

        let pawn_rank_direction = match self.side_to_move {
            Color::White => 1,
            Color::Black => -1
        };
        //step by one
        if let Some(pawn_square) = 
            offset_square(square, 0, pawn_rank_direction)
            && self.pieces[pawn_square as usize] == Piece::Empty {
                if get_rank(pawn_square) == 7 || get_rank(pawn_square) == 0{
                    self._generate_promotion_moves(square, pawn_square, possible_moves);
                }
                else {
                    possible_moves.push(Move {
                    from: square, 
                    to: pawn_square,
                    move_type: MoveType::Normal,
                    });
                }
                //step by two
                if get_rank(square) == match self.side_to_move {
                        Color::White => 1, 
                        Color::Black => 6
                    }
                    && let Some(pawn_square) = 
                        offset_square(square, 0, pawn_rank_direction*2)
                            && self.pieces[pawn_square as usize] == Piece::Empty {
                                possible_moves.push(Move {
                                    from: square,
                                    to: pawn_square,
                                    move_type: MoveType::PawnDoubleStep,
                                });
                            }
                
            }
        //captures
        for file_offset in [-1, 1] {
            if let Some(pawn_square) = 
                offset_square(square, file_offset, pawn_rank_direction){
                    if self.pieces[pawn_square as usize].color() == Some(self.side_to_move.invert()) {
                        if get_rank(pawn_square) == 7 || get_rank(pawn_square) == 0{
                                self._generate_promotion_moves(square, pawn_square, possible_moves);
                        }
                        else {
                            possible_moves.push(Move {
                                from: square,
                                to: pawn_square,
                                move_type: MoveType::Normal,
                            });
                        }
                    }
                    //en passant
                    if let Some(en_square) = self.en_passant_target_square
                        && pawn_square == en_square{
                            possible_moves.push(Move{
                                from: square,
                                to: pawn_square,
                                move_type: MoveType::EnPassant,
                            });
                        } 
                    
                }

        }
            
    }
    fn _generate_promotion_moves(&self, square: u8, pawn_square: u8, possible_moves: &mut ArrayVec<Move, 218>){
        possible_moves.push(Move {
            from: square,
            to: pawn_square,
            move_type: MoveType::Promotion(self.side_to_move.bishop())
        });
        possible_moves.push(Move {
            from: square,
            to: pawn_square,
            move_type: MoveType::Promotion(self.side_to_move.knight())
        });
        possible_moves.push(Move {
            from: square,
            to: pawn_square,
            move_type: MoveType::Promotion(self.side_to_move.rook())
        });
        possible_moves.push(Move {
            from: square,
            to: pawn_square,
            move_type: MoveType::Promotion(self.side_to_move.queen())
        });
        
    }

    fn generate_knight_moves(&self, square: u8, possible_moves: &mut ArrayVec<Move, 218>){
        const KNIGHT_OFFSETS: [(i8, i8); 8] = [
            (1, 2),
            (2, 1),
            (2, -1),
            (1, -2),
            (-1, -2),
            (-2, -1),
            (-2, 1),
            (-1, 2),
        ];

        for (file_offset, rank_offset) in KNIGHT_OFFSETS {
            if let Some(destination) = offset_square(square, file_offset, rank_offset)
                && self.pieces[destination as usize].color() != Some(self.side_to_move) {
                    possible_moves.push(Move {
                        from: square,
                        to: destination,
                        move_type: MoveType::Normal,
                    })
                }
            
        }
    }

    fn generate_bishop_moves(&self, square: u8, possible_moves: &mut ArrayVec<Move, 218>){
        const BISHOP_DIRECTIONS: [(i8, i8); 4] = [
            (1, 1),
            (1, -1),
            (-1, 1),
            (-1, -1),
        ];
        for (file_offset, rank_offset) in BISHOP_DIRECTIONS {
            let mut current = square;

            while let Some(next) = offset_square(current, file_offset, rank_offset) {
                current = next; 
                if self.pieces[current as usize].color() !=
                    Some(self.side_to_move) {
                        possible_moves.push(Move{
                        from: square,
                        to: current,
                        move_type: MoveType::Normal,
                    });
                        if self.pieces[current as usize].color() == Some(self.side_to_move.invert()) {
                            break; //break here, because we cannot move further than a piece we could capture
                        }
                }
                else {
                    break;
                }
            }
        }
    }

    fn generate_rook_moves(&self, square: u8, possible_moves: &mut ArrayVec<Move, 218>){

        const ROOK_DIRECTIONS: [(i8, i8); 4] = [
            (1, 0),
            (-1, 0),
            (0, 1),
            (0, -1),
        ];

        for (file_offset, rank_offset) in ROOK_DIRECTIONS {
            let mut current = square;

            loop {
                let Some(next) = offset_square(current, file_offset, rank_offset) else{
                    break;
                };

                current = next; 
                if self.pieces[current as usize].color() !=
                    Some(self.side_to_move) {
                        possible_moves.push(Move{
                        from: square,
                        to: current,
                        move_type: MoveType::Normal,
                    });
                    if self.pieces[current as usize].color() == Some(self.side_to_move.invert()) {
                        break; //break here, because we cannot move further than a piece we could capture
                    }
                }
                else {
                    break;
                }
            }
        }

    }

    fn generate_king_moves(&self, square: u8, possible_moves: &mut ArrayVec<Move, 218>){
        const KING_OFFSETS: [(i8, i8); 8] = [
            (1, 0),
            (1, 1),
            (0, 1),
            (-1, 1),
            (-1, 0),
            (-1, -1),
            (0, -1),
            (1, -1),
        ];

        for (file_offset, rank_offset) in KING_OFFSETS {
            if let Some(destination) = 
                offset_square(square, file_offset, rank_offset)
                    && self.pieces[destination as usize].color() != Some(self.side_to_move) {
                        possible_moves.push(Move {
                            from: square, 
                            to: destination,
                            move_type: MoveType::Normal,
                        });
                    }
        }
        //determine if castling is possible
        //castling is possible if king and rook have not been taken or been moved
        if self.side_to_move == Color::White {
            if self.white_king_castling_possible
                && !self.is_square_attacked(5, Color::Black) &&
                    !self.is_square_attacked(6, Color::Black) &&
                    !self.is_square_attacked(4, Color::Black)
                        && self.pieces[5] == Piece::Empty && self.pieces[6] == Piece::Empty{
                            possible_moves.push(Move {
                                from: 4,
                                to: 6,
                                move_type: MoveType::CastleKingside
                            });
                        }
            if self.white_queen_castling_possible
                && !self.is_square_attacked(2, Color::Black) &&
                    !self.is_square_attacked(3, Color::Black) &&
                    !self.is_square_attacked(4, Color::Black)
                        && self.pieces[1] == Piece::Empty && self.pieces[2] == Piece::Empty && self.pieces[3] == Piece::Empty{
                            possible_moves.push(Move {
                                from: 4, 
                                to: 2,
                                move_type: MoveType::CastleQueenside
                            });
                        }
        }
        else {
            if self.black_king_castling_possible
                && !self.is_square_attacked(62, Color::White) &&
                    !self.is_square_attacked(61, Color::White) &&
                    !self.is_square_attacked(60, Color::White)
                        && self.pieces[61] == Piece::Empty && self.pieces[62] == Piece::Empty{
                            possible_moves.push(Move {
                            from: 60,
                            to: 62,
                            move_type: MoveType::CastleKingside
                            });
                        }
            if self.black_queen_castling_possible
                && !self.is_square_attacked(58, Color::White) &&
                    !self.is_square_attacked(59, Color::White) &&
                    !self.is_square_attacked(60, Color::White)
                        && self.pieces[59] == Piece::Empty && self.pieces[58] == Piece::Empty && self.pieces[57] == Piece::Empty{
                            possible_moves.push(Move {
                            from: 60, 
                            to: 58,
                            move_type: MoveType::CastleQueenside
                        });
                        }
        }
    }

    fn generate_queen_moves(&self, square: u8, possible_moves: &mut ArrayVec<Move, 218>){
        self.generate_bishop_moves(square, possible_moves);
        self.generate_rook_moves(square, possible_moves);


    }

    fn is_square_attacked(&self, square: u8, color: Color) -> bool {
        //color is the attacking color
        // -------------------------------------------------
        // Pawns
        // -------------------------------------------------

        // We are looking for a pawn that could capture `square`.
        //
        // White pawns attack one rank upwards.
        // Black pawns attack one rank downwards.
        let pawn_rank_direction = match color {
            Color::White => -1,
            Color::Black => 1,
        };

        for file_offset in [-1, 1] {
            if let Some(pawn_square) =
                offset_square(square, file_offset, pawn_rank_direction)
                && self.pieces[pawn_square as usize] == color.pawn() {
                    return true;
                }
        }

        // -------------------------------------------------
        // Knights
        // -------------------------------------------------

        const KNIGHT_OFFSETS: [(i8, i8); 8] = [
            (1, 2),
            (2, 1),
            (2, -1),
            (1, -2),
            (-1, -2),
            (-2, -1),
            (-2, 1),
            (-1, 2),
        ];

        for (file_offset, rank_offset) in KNIGHT_OFFSETS {
            if let Some(source) = offset_square(square, file_offset, rank_offset)
                && self.pieces[source as usize] == color.knight() {
                    return true;
                }
        }

        // -------------------------------------------------
        // King
        // -------------------------------------------------

        const KING_OFFSETS: [(i8, i8); 8] = [
            (1, 0),
            (1, 1),
            (0, 1),
            (-1, 1),
            (-1, 0),
            (-1, -1),
            (0, -1),
            (1, -1),
        ];

        for (file_offset, rank_offset) in KING_OFFSETS {
            if let Some(source) = offset_square(square, file_offset, rank_offset) { //can the square be offset like that?
                if self.pieces[source as usize] == color.king() {
                    return true;
                }
            }
        }

        // -------------------------------------------------
        // Rooks / Queens
        // -------------------------------------------------

        const ROOK_DIRECTIONS: [(i8, i8); 4] = [
            (1, 0),
            (-1, 0),
            (0, 1),
            (0, -1),
        ];

        if self.is_slider_attack(square, color, &ROOK_DIRECTIONS, true) {
            return true;
        }

        // -------------------------------------------------
        // Bishops / Queens
        // -------------------------------------------------

        const BISHOP_DIRECTIONS: [(i8, i8); 4] = [
            (1, 1),
            (1, -1),
            (-1, 1),
            (-1, -1),
        ];

        if self.is_slider_attack(square, color, &BISHOP_DIRECTIONS, false) {
            return true;
        }

        false
    }

    fn is_slider_attack(&self,square: u8,color: Color,directions: &[(i8, i8)],rook_direction: bool) -> bool {
        for &(file_offset, rank_offset) in directions {
            let mut current = square;

            while let Some(next) = offset_square(current, file_offset, rank_offset) {
                current = next;

                match self.pieces[current as usize] {
                    Piece::Empty => continue,

                    piece if piece == color.queen() => return true,

                    piece if rook_direction && piece == color.rook() => return true,

                    piece if !rook_direction && piece == color.bishop() => return true,

                    _ => break,
                }
            }
        }

        false
    }

    fn is_in_check(&self, color: Color) -> bool {
        let king = color.king();

        let opt_king_square = self
            .pieces
            .iter()
            .position(|&piece| piece == king);
        let king_square = match opt_king_square {
            Some(k) => k as u8,
            None => {
                eprintln!("Board must contain a king!");
                eprintln!("current board: {}", self);
                panic!()
            }
        };


        let attacking_color = match color {
            Color::White => Color::Black,
            Color::Black => Color::White,
        };

        self.is_square_attacked(king_square, attacking_color)
    }

    fn set_castling_rights(&mut self, m: &Move) {
        // A rook moved
        match m.from {
            0 => self.white_queen_castling_possible = false,  // a1
            7 => self.white_king_castling_possible = false,  // h1
            56 => self.black_queen_castling_possible = false, // a8
            63 => self.black_king_castling_possible = false, // h8
            _ => {}
        }

        // A rook was captured
        match m.to {
            0 => self.white_queen_castling_possible = false,
            7 => self.white_king_castling_possible = false,
            56 => self.black_queen_castling_possible = false,
            63 => self.black_king_castling_possible = false,
            _ => {}
        }

        // A king moved
        match self.pieces[m.to as usize] {
            Piece::WKing => {
                self.white_king_castling_possible = false;
                self.white_queen_castling_possible = false;
            }
            Piece::BKing => {
                self.black_king_castling_possible = false;
                self.black_queen_castling_possible = false;
            }
            _ => {}
        }
    }

    pub fn fen(&self) -> String {
        let mut pieces = self.pieces;
        pieces.reverse();
        let mut fen_string = String::new();
        let mut counter = 0;
        for (i,p) in pieces.iter().enumerate() {
            if i%8 == 0 && i != 0 {
                if counter != 0 {
                    fen_string.push_str(&counter.to_string());
                    counter = 0;
                }
                fen_string.push('/');
            }

            match p {
                Piece::WKing => fen_string.push('K'),
                Piece::WQueen => fen_string.push('Q'),
                Piece::WBishop => fen_string.push('B'),
                Piece::WKnight => fen_string.push('N'),
                Piece::WRook => fen_string.push('R'),
                Piece::WPawn => fen_string.push('P'),
                Piece::BKing => fen_string.push('k'),
                Piece::BQueen => fen_string.push('q'),
                Piece::BBishop => fen_string.push('b'),
                Piece::BKnight => fen_string.push('n'),
                Piece::BRook => fen_string.push('r'),
                Piece::BPawn => fen_string.push('p'),
                Piece::Empty => counter += 1
            }
            if p != &Piece::Empty && counter != 0 {
                fen_string.push_str(&counter.to_string());
                counter = 0
            }
            if counter == 8 {
                fen_string.push_str(&counter.to_string());
                counter = 0;
            }
        }
        fen_string
    }

    pub fn evaluate(&self) -> isize {
        let mut score = 0;
        let side_val = self.side_to_move.value();
        if self.is_in_check(self.side_to_move){
            score -= 30;
        }
        for (i,p) in self.pieces.iter().enumerate() {
            score += p.score();
            match p {
                Piece::Empty => {continue},
                Piece::WPawn => score += W_PAWN_EVAL[i],
                Piece::BPawn => score -= B_PAWN_EVAL[i],
                Piece::WKnight =>  score += W_KNIGHT_EVAL[i],
                Piece::BKnight =>  score -= B_KNIGHT_EVAL[i],
                Piece::WBishop =>  score += W_BISHOP_EVAL[i] ,
                Piece::BBishop =>  score -= B_BISHOP_EVAL[i] ,
                Piece::WRook =>  score += W_ROOK_EVAL[i],
                Piece::BRook =>  score -= B_ROOK_EVAL[i],
                Piece::WQueen =>  score += W_QUEEN_EVAL[i],
                Piece::BQueen =>  score -= B_QUEEN_EVAL[i],
                Piece::WKing =>  score += W_KING_MG_EVAL[i],
                Piece::BKing =>  score -= B_KING_MG_EVAL[i] ,
                
            };
            
        }
        
        score * side_val
    }

    pub fn search(&mut self, depth: usize, adaptive_depth_allowed: bool) -> Move{
        let mut adaptive_depth = depth;

        println!("info: now searching for best move with depth {depth}, side to move is {:?}", self.side_to_move);
        println!("board is: \n{}", self);
        let time_before = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let mut possible_moves = self.generate_possible_moves();
        let mut best_move: Move = possible_moves[0];
        let mut current_depth = 1;

        let mut cntxt = Context{
            nodes: 0,
            start_time: time_before,
        };
        //iterative deepening
        loop {
            current_depth += 1;
            
            if current_depth > adaptive_depth {
                break;
            } 
            let mut alpha:isize = -Board::INF;
            let beta:isize = Board::INF;

            let mut current_best_move = possible_moves[0];
            let mut current_best_move_idx = 0;

            for (j,m) in possible_moves.iter().enumerate() {
                let undo = self.make_move(m);
                let result = self.nega_max(current_depth-1, -beta, -alpha, &mut cntxt);
                let score = match result {
                    Ok(r) => -r,
                    Err(e) => {
                        println!("{e}");
                        let time_after = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
                        println!("search took {:?}", time_after-time_before);
                        return best_move
                     } //time is up, we respond with the best move from the previous iteration
                };
                if score > alpha {
                    alpha = score;
                    current_best_move = *m;
                    current_best_move_idx = j;
                }
                self.unmake_move(m, undo);

            }
            possible_moves.swap(0, current_best_move_idx); //ordering after the best move
            best_move = current_best_move;
            let time_after = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
            let time_taken = time_after - time_before;
            println!("completed search with depth {}, score {}, in {:?}", current_depth, alpha, time_taken);
            
        }
        
        let time_after = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();

        println!("search took {:?}", time_after-time_before);
        best_move
    }

    fn nega_max(&mut self, depth: usize, mut alpha: isize, beta: isize, context: &mut Context) -> Result<isize, &'static str>{
        if context.nodes & 1024 == 0 {
            let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
            if current_time - context.start_time > Duration::from_millis(950) {
                return Err("out of time")
            }
        }
        if self.is_draw() {
            return Ok(0); //meh outcome, there is more to be had :[
        }
        
        if depth == 0 {
            return Ok(self.quiesce_search(alpha, beta, context)?)
        }

        let possible_moves = self.generate_possible_moves();

        if possible_moves.is_empty() {
            if self.is_in_check(self.side_to_move){
                return Ok(-Board::INF)
            }
            return Ok(0)
        }        
        let mut best_score = -Board::INF; //mistake here likely
        for m in possible_moves.iter(){
            context.nodes += 1;
            let undo = self.make_move(m);
            let score = -self.nega_max(depth-1, -beta, -alpha, context)?;
                
            self.unmake_move(m, undo);

            best_score = best_score.max(score);
            //println!("score is : {score}, best score is {best_score}");
            alpha = alpha.max(score);

            if alpha >= beta {
                break;
            }
        }
        Ok(best_score)
    }

    pub fn quiesce_search(&mut self, mut alpha: isize, beta: isize, context: &mut Context) -> Result<isize, &'static str>  {
        if context.nodes & 1023 == 0 {
            let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
            if current_time - context.start_time > Duration::from_millis(950) {
                return Err("out of time")
            }
        }
        let static_eval = self.evaluate();

        if static_eval >= beta {
            return Ok(static_eval);
        }
        
        let mut best_score = static_eval;
        if static_eval > alpha {
            alpha = static_eval;
        }

        let mut captures = self.generate_possible_moves();
        
        captures.retain(|m| {
            self.pieces[m.to as usize] != Piece::Empty || m.move_type == MoveType::EnPassant
        });

        captures.sort_by_key(|m| {
            let attacker_val = self.pieces[m.from as usize].base_value();
            let victim_val = if m.move_type == MoveType::EnPassant {
                100 
            } else {
                self.pieces[m.to as usize].base_value()
            };
            (10 * victim_val) - attacker_val
        });

        while let Some(current_move) = captures.pop() {
            let target_square = current_move.to;
            let victim_val = if current_move.move_type == MoveType::EnPassant {
                100
            } else {
                self.pieces[target_square as usize].base_value()
            };

            if !matches!(current_move.move_type, MoveType::Promotion(_)) 
                && static_eval + victim_val + 200 < alpha 
            {
                continue; // delta pruning
            }
            let undo = self.make_move(&current_move);
            let score = -self.quiesce_search(-beta, -alpha, context)?;
            self.unmake_move(&current_move, undo);

            if score > best_score {
                best_score = score;
            }
            if score >= beta {
                return Ok(score); // Fail-high
            }
            if score > alpha {
                alpha = score;
            }
        }

        Ok(best_score)
    }

    //returns true if the side currently to move is in checkmate

    pub fn is_checkmate(&self) -> bool {
        self.is_in_check(self.side_to_move) && !self.has_move()
    }

    pub fn is_draw(&self) -> bool {
        //returns true if no pieces other than kings are on the board
        self.pieces.iter().filter(|x| **x != Piece::WKing && **x != Piece::BKing && **x != Piece::Empty).count() == 0
    }
}
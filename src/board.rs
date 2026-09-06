use core::panic;
use std::fmt;
use std::collections::HashMap;
use crate::piece::Piece;
use crate::Move;
use crate::Color;
use crate::MoveType;
use crate::offset_square;
use crate::get_rank;
use crate::get_file;
use position_key::zobrist_hash;
use undo::Undo;

pub mod position_key;
mod undo;
#[derive(Clone)]
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
    pub fn make_move(&mut self, m: &Move) -> Undo{
        //TODO

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
            captured_piece: captured_piece,
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
                self.pieces[(from+1) ] = self.pieces[(to +1) ];
                self.pieces[(to + 1) ] = Piece::Empty;}
            MoveType::CastleQueenside => {
                self.pieces[to ] = piece;
                self.pieces[from ] = Piece::Empty;
                self.pieces[(from -1) ] = self.pieces[(from-4) ];
                self.pieces[(from -4)] = Piece::Empty;}
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
        self.position_history_hashed.push(position_key::zobrist_hash(&self));
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
    }
 

    pub fn generate_possible_moves(&self) -> Vec<Move> {
        let mut possible_moves:Vec<Move> = Vec::new();
        for (i, piece) in self.pieces.iter().enumerate() {
            let square: u8 = i.try_into().unwrap();
            let color = self.side_to_move;
            match color {
                Color::White => {
                    match piece {
                        Piece::WKing => 
                            possible_moves.append(&mut self.generate_king_moves(square)),
                        Piece::WQueen => 
                            possible_moves.append(&mut self.generate_queen_moves(square)),
                        Piece::WBishop =>
                            possible_moves.append(&mut self.generate_bishop_moves(square)), 
                        Piece::WKnight => 
                            possible_moves.append(&mut self.generate_knight_moves(square)),
                        Piece::WRook => 
                            possible_moves.append(&mut self.generate_rook_moves(square)),
                        Piece::WPawn =>
                            possible_moves.append(&mut self.generate_pawn_moves(square)),
                        _ => continue
                    }
                }
                Color::Black => {
                    match piece {
                        Piece::BKing => 
                            possible_moves.append(&mut self.generate_king_moves(square)),
                        Piece::BQueen => 
                            possible_moves.append(&mut self.generate_queen_moves(square)),
                        Piece::BBishop =>
                            possible_moves.append(&mut self.generate_bishop_moves(square)), 
                        Piece::BKnight => 
                            possible_moves.append(&mut self.generate_knight_moves(square)),
                        Piece::BRook => 
                            possible_moves.append(&mut self.generate_rook_moves(square)),
                        Piece::BPawn =>
                            possible_moves.append(&mut self.generate_pawn_moves(square)),
                        _ => continue
                    }
                }
                

            }
        }

        let mut legal_moves = Vec::new();
        
        for m in possible_moves {
            let mut test_board = self.clone();
            test_board.make_move(&m);
            if !test_board.is_in_check(self.side_to_move) {
                legal_moves.push(m);
            }
        }
        legal_moves
    }

    fn generate_pawn_moves(&self, square: u8) -> Vec<Move>{
        let mut possible_moves = Vec::new();

        let pawn_rank_direction = match self.side_to_move {
            Color::White => 1,
            Color::Black => -1
        };
        //step by one
        if let Some(pawn_square) = 
            offset_square(square, 0, pawn_rank_direction) {
            if self.pieces[pawn_square as usize] == Piece::Empty {
                if get_rank(pawn_square) == 7 || get_rank(pawn_square) == 0{
                    possible_moves = self._generate_promotion_moves(square, pawn_square, possible_moves);
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
                    }{
                    if let Some(pawn_square) = 
                        offset_square(square, 0, pawn_rank_direction*2) {
                            if self.pieces[pawn_square as usize] == Piece::Empty {
                                possible_moves.push(Move {
                                    from: square,
                                    to: pawn_square,
                                    move_type: MoveType::PawnDoubleStep,
                                });
                            }
                        }
                }
                
            } 
            
        }
        //captures
        for file_offset in [-1, 1] {
            if let Some(pawn_square) = 
                offset_square(square, file_offset, pawn_rank_direction){
                    if self.pieces[pawn_square as usize].color() == Some(self.side_to_move.invert()) {
                        if get_rank(pawn_square) == 7 || get_rank(pawn_square) == 0{
                                possible_moves = self._generate_promotion_moves(square, pawn_square, possible_moves);
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
                    if let Some(en_square) = self.en_passant_target_square {
                        if pawn_square == en_square{
                            possible_moves.push(Move{
                                from: square,
                                to: pawn_square,
                                move_type: MoveType::EnPassant,
                            });
                        }
                    } 
                    
                }

        }
            
        possible_moves
    }
    fn _generate_promotion_moves(&self, square: u8, pawn_square: u8, mut possible_moves: Vec<Move>) -> Vec<Move> {
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
        
        possible_moves
    }

    fn generate_knight_moves(&self, square: u8) -> Vec<Move> {
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

        let mut possible_moves = Vec::new();
        
        for (file_offset, rank_offset) in KNIGHT_OFFSETS {
            if let Some(destination) = offset_square(square, file_offset, rank_offset) {
                if self.pieces[destination as usize].color() != Some(self.side_to_move) {
                    possible_moves.push(Move {
                        from: square,
                        to: destination,
                        move_type: MoveType::Normal,
                    })
                }
            }
            
        }
        possible_moves
    }

    fn generate_bishop_moves(&self, square: u8) -> Vec<Move>{
        let mut possible_moves = Vec::new();
        const BISHOP_DIRECTIONS: [(i8, i8); 4] = [
            (1, 1),
            (1, -1),
            (-1, 1),
            (-1, -1),
        ];
        for (file_offset, rank_offset) in BISHOP_DIRECTIONS {
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
        possible_moves
    }

    fn generate_rook_moves(&self, square: u8) -> Vec<Move> {
        let mut possible_moves = Vec::new();

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
        possible_moves

    }

    fn generate_king_moves(&self, square: u8) -> Vec<Move>{
        let mut possible_moves = Vec::new();
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
                offset_square(square, file_offset, rank_offset) {
                    if self.pieces[destination as usize].color() != Some(self.side_to_move) {
                        possible_moves.push(Move {
                            from: square, 
                            to: destination,
                            move_type: MoveType::Normal,
                        });
                    }
                }
        }
        //determine if castling is possible
        //castling is possible if king and rook have not been taken or been moved
        if self.side_to_move == Color::White {
            if self.white_king_castling_possible {
                if !self.is_square_attacked(5, Color::Black) &&
                    !self.is_square_attacked(6, Color::Black) &&
                    !self.is_square_attacked(4, Color::Black){
                        if self.pieces[5] == Piece::Empty && self.pieces[6] == Piece::Empty{
                            possible_moves.push(Move {
                                from: 4,
                                to: 6,
                                move_type: MoveType::CastleKingside
                            });
                        }   
                    }
            }
            if self.white_queen_castling_possible {
                if !self.is_square_attacked(2, Color::Black) &&
                    !self.is_square_attacked(3, Color::Black) &&
                    !self.is_square_attacked(4, Color::Black) {
                        if self.pieces[1] == Piece::Empty && self.pieces[2] == Piece::Empty && self.pieces[3] == Piece::Empty{
                            possible_moves.push(Move {
                                from: 4, 
                                to: 2,
                                move_type: MoveType::CastleQueenside
                            });
                        }
                        
                    }
            }
        }
        else {
            if self.black_king_castling_possible {
                if !self.is_square_attacked(62, Color::White) &&
                    !self.is_square_attacked(61, Color::White) &&
                    !self.is_square_attacked(60, Color::White){
                        if self.pieces[61] == Piece::Empty && self.pieces[62] == Piece::Empty{
                            possible_moves.push(Move {
                            from: 60,
                            to: 62,
                            move_type: MoveType::CastleKingside
                            });
                        }
                        
                    }
                
            }
            if self.black_queen_castling_possible {
                if !self.is_square_attacked(58, Color::White) &&
                    !self.is_square_attacked(59, Color::White) &&
                    !self.is_square_attacked(60, Color::White) {
                        if self.pieces[59] == Piece::Empty && self.pieces[58] == Piece::Empty && self.pieces[57] == Piece::Empty{
                            possible_moves.push(Move {
                            from: 60, 
                            to: 58,
                            move_type: MoveType::CastleQueenside
                        });
                        }
                    }
            }
        }
        
        possible_moves
    }

    fn generate_queen_moves(&self, square: u8) -> Vec<Move>{
        let mut possible_moves = Vec::new();
        possible_moves.append(&mut self.generate_bishop_moves(square));
        possible_moves.append(&mut self.generate_rook_moves(square));

        possible_moves

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
            {
                if self.pieces[pawn_square as usize] == color.pawn() {
                    return true;
                }
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
            if let Some(source) = offset_square(square, file_offset, rank_offset) {
                if self.pieces[source as usize] == color.knight() {
                    return true;
                }
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

            loop {
                let Some(next) = offset_square(current, file_offset, rank_offset) else {
                    break;
                };

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

        let mut king_square:u8 = 0;
        let opt_king_square = self
            .pieces
            .iter()
            .position(|&piece| piece == king);
        match opt_king_square {
            Some(k) => king_square = k as u8,
            None => {
                eprintln!("Board must contain a king!");
                eprintln!("current board: {}", self);
                panic!()
            }
        }


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
        let mut pieces = self.pieces.clone();
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
}
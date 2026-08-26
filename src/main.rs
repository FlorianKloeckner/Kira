use std::collections::HashMap;
use std::io::{self, Write};
use std::time::{SystemTime, UNIX_EPOCH};
use rand::distr::Uniform;
use rand::seq::IndexedRandom;
use crate::piece::Piece;
use crate::board::Board;

use crate::translate_move_uci::{uci_to_move, move_to_uci};

pub mod translate_move_uci;
pub mod board;
pub mod piece;

//TODO write unmake move 
    //TODO to calculate possible moves
//TODO fuck 50-Move rule, we dont fucking care



fn main() {
    let mut board = setup_board();
    let mut counter: u64 = 0;
    let depth = 5;
    let start = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    search(&mut board, depth, &mut counter);
    let end = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();

    eprintln!("Found {counter} moves from the starting position with depth {depth} in {:?}", (end-start));
    return;
    loop {
        let possible_moves = board.generate_possible_moves();
        let ai_move = possible_moves.choose(&mut rand::rng());
        let ai_move = ai_move.expect("AI lost, as it has no more moves");
        let undo = board.make_move(ai_move);
        board.unmake_move(ai_move, undo);
        board.make_move(ai_move);

        println!("{}", move_to_uci(ai_move));
        io::stdout().flush().expect("failed to flush move to gui"); // <-- without this, Python's readline() can hang forever

        let player_move: Move;
        loop {
            let opt_player_move = get_player_move_from_gui(&board);
            
            match opt_player_move {
                Some(m) => {
                    player_move = m;
                    break;
                }
                None => (),
            }
        }
        eprintln!("converted extracted move: {player_move:?}");
        let undo = board.make_move(&player_move);
        board.unmake_move(&player_move, undo);
        board.make_move(&player_move);
        print_possible_engine_moves(&board);
        eprintln!("Current board state [Kira] : ", );
        eprintln!("{board}");

    }
}

fn get_player_move_from_gui(board: &Board) -> Option<Move> {
    let mut user_move_buf = String::new();
    io::stdin()
        .read_line(&mut user_move_buf)
        .expect("failed to read gui move");
    eprintln!("read user move {user_move_buf}"); 
    


    let uci_move = extract_move_token(&user_move_buf).expect("malformed gui message");

    eprintln!("Extracted move token: {}", uci_move);

    let res = uci_to_move(uci_move, board);
    Some(res.expect("invalid move"))
}

fn search(b:&mut Board, depth: u8, mut counter: &mut u64){
    if depth == 0{
        *counter += 1;
        return;
    }
    let possible_moves = b.generate_possible_moves();
    for m in possible_moves.iter() {
        let undo = b.make_move(m);
        search(b, depth-1, counter);
        b.unmake_move(m, undo);
    }
}

/// Parses "position <fen> move <uci>" and returns just the trailing UCI move,
/// discarding the fen — board state is tracked internally, not taken from the gui.
fn extract_move_token(line: &str) -> Option<&str> {
    line.trim().rsplit(' ').nth(0)
}

fn print_possible_engine_moves(board: &Board) {
    let possible_moves = board.generate_possible_moves();
    eprintln!("Engine has {} possible moves", possible_moves.len());
    for m in possible_moves.iter() {
        eprintln!("{}",move_to_uci(m));
    }
}

fn get_player_move() -> Move {
    let mut user_move_from = String::new();
    io::stdin()
        .read_line(&mut user_move_from)
        .expect("Failed to read your move");
    let mut user_move_to = String::new();
    io::stdin()
        .read_line(&mut user_move_to)
        .expect("Could not read your move");

    let user_move = Move {
        from: user_move_from.trim().parse().expect("Move origin invalid"),

        to: user_move_to.trim().parse().expect("Move destination invalid"),
        move_type: MoveType::Normal, //TODO update, depending on what move the user does
    };
    user_move
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
enum Color {
    White, 
    Black
}

impl Color {
    fn pawn(self) -> Piece {
        match self {
            Color::White => Piece::WPawn,
            Color::Black => Piece::BPawn,
        }
    }

    fn knight(self) -> Piece {
        match self {
            Color::White => Piece::WKnight,
            Color::Black => Piece::BKnight,
        }
    }

    fn bishop(self) -> Piece {
        match self {
            Color::White => Piece::WBishop,
            Color::Black => Piece::BBishop,
        }
    }

    fn rook(self) -> Piece {
        match self {
            Color::White => Piece::WRook,
            Color::Black => Piece::BRook,
        }
    }

    fn queen(self) -> Piece {
        match self {
            Color::White => Piece::WQueen,
            Color::Black => Piece::BQueen,
        }
    }

    fn king(self) -> Piece {
        match self {
            Color::White => Piece::WKing,
            Color::Black => Piece::BKing,
        }
    }

    fn invert(self) -> Color{
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }
}



fn setup_board() -> Board{
    let b = Board {
        pieces: {
            let mut piece_arr: [Piece; 64] = {[Piece::Empty; 64]};
            piece_arr[0] = Piece::WRook;
            piece_arr[1] = Piece::WKnight;
            piece_arr[2] = Piece::WBishop;
            piece_arr[3] = Piece::WQueen;
            piece_arr[4] = Piece::WKing;
            piece_arr[5] = Piece::WBishop;
            piece_arr[6] = Piece::WKnight;
            piece_arr[7] = Piece::WRook;
            for i in 8..16 {
                piece_arr[i] = Piece::WPawn;
            }
            for i in 48..56 {
                piece_arr[i] = Piece::BPawn;
            }
            piece_arr[56] = Piece::BRook;
            piece_arr[57] = Piece::BKnight;
            piece_arr[58] = Piece::BBishop;
            piece_arr[59] = Piece::BQueen;
            piece_arr[60] = Piece::BKing;
            piece_arr[61] = Piece::BBishop;
            piece_arr[62] = Piece::BKnight;
            piece_arr[63] = Piece::BRook;
            piece_arr
        },
        last_move: Move{
            from: 0,
            to: 0,
            move_type: MoveType::Normal,
        },
        white_to_move: Color::White,
        white_king_castling_possible: true,
        black_queen_castling_possible: true,
        white_queen_castling_possible: true,
        black_king_castling_possible: true,
        move_history: Vec::new(),
    };
    b
}

fn get_rank(index: u8) -> u8{
    (index / 8) as u8
}

fn get_file(index: u8) -> u8{
    (index % 8) as u8
}



#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
struct Move {
    from: u8, 
    to: u8,
    move_type: MoveType,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
enum MoveType {
    Normal,
    CastleKingside,
    CastleQueenside,
    EnPassant,
    Promotion(Piece),
    PawnDoubleStep,
}

pub fn offset_square(square: u8, file_offset: i8, rank_offset: i8) -> Option<u8> {
    let file = get_file(square) as i8 + file_offset;
    let rank = get_rank(square) as i8 + rank_offset;

    if (0..8).contains(&file) && (0..8).contains(&rank) {
        Some((rank * 8 + file) as u8)
    } else {
        None
    }
}

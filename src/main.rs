use std::env;
use std::io::{self, Write};
use std::time::{SystemTime, UNIX_EPOCH};
use rand::seq::IndexedRandom;
use crate::piece::Piece;
use crate::board::Board;

use crate::translate_move_uci::{uci_to_move, move_to_uci};

pub mod translate_move_uci;
pub mod board;
pub mod piece;

//TODO debug 7 depth perft (setup stockfish with the same stuff)
//TODO load_fen
//TODO fuck 50-Move rule, we dont fucking care



fn main() {
    let args: Vec<String> = env::args().collect();
    let mut board = setup_board();
    println!("{:?}", args);
    if args[1] == "perft" {
        let start = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let amount = perft(&mut board, args[2].parse().expect("argument after perft must be integer"));
        let end = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();

        eprintln!("found {amount} nodes in {:?}", end-start);
    }
    else if args[1] == "dperft" {
        divided_perft(&mut board, args[2].parse().expect("argument after dperft must be depth"));
    }
    else if args[1] == "fperft" {
        board = load_fen(&args[2]).unwrap();
        divided_perft(&mut board, args[3].parse().expect(""));
    }
    else if args[1] == "fen" {
        let res = load_fen(&args[2]).unwrap();
        println!("successfully loaded board: ");
        println!("{res}");
    }
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

fn perft(b: &mut Board, depth: u8) -> usize {
    let mut nodes: usize = 0;

    if depth == 0 {
        return 1;
    }

    let possible_moves = b.generate_possible_moves();

    for m in possible_moves.iter() {
        // Save the complete position BEFORE make_move
        let original = b.clone();

        let undo = b.make_move(m);

        nodes += perft(b, depth - 1);

        b.unmake_move(m, undo);

        // Verify that unmake restored everything
        assert_eq!(
            b.pieces,
            original.pieces,
            "PIECES CORRUPTED by move {:?} at depth {}",
            m,
            depth
        );

        assert_eq!(
            b.side_to_move,
            original.side_to_move,
            "SIDE TO MOVE CORRUPTED by move {:?} at depth {}",
            m,
            depth
        );

        assert_eq!(
            b.en_passant_target_square,
            original.en_passant_target_square,
            "EN PASSANT CORRUPTED by move {:?} at depth {}",
            m,
            depth
        );

        assert_eq!(
            b.white_king_castling_possible,
            original.white_king_castling_possible,
            "WHITE KING CASTLING CORRUPTED by move {:?} at depth {}",
            m,
            depth
        );

        assert_eq!(
            b.white_queen_castling_possible,
            original.white_queen_castling_possible,
            "WHITE QUEEN CASTLING CORRUPTED by move {:?} at depth {}",
            m,
            depth
        );

        assert_eq!(
            b.black_king_castling_possible,
            original.black_king_castling_possible,
            "BLACK KING CASTLING CORRUPTED by move {:?} at depth {}",
            m,
            depth
        );

        assert_eq!(
            b.black_queen_castling_possible,
            original.black_queen_castling_possible,
            "BLACK QUEEN CASTLING CORRUPTED by move {:?} at depth {}",
            m,
            depth
        );
    }

    nodes
}

fn divided_perft(b:&mut Board, depth: u8) {
    let possible_moves = b.generate_possible_moves();
    let mut total_nodes = 0;
    for m in possible_moves.iter() {
        let undo = b.make_move(m);
        let nodes = perft(b, depth-1);
        b.unmake_move(m, undo);
        eprintln!("{}: {:?}", move_to_uci(m), nodes);
        total_nodes += nodes;
    }
    eprintln!("Searched {} nodes", total_nodes);
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
        en_passant_target_square: None,
        side_to_move: Color::White,
        white_king_castling_possible: true,
        black_queen_castling_possible: true,
        white_queen_castling_possible: true,
        black_king_castling_possible: true,
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

fn load_fen(fen: &str) -> Result<Board, String> {
    let fields: Vec<&str> = fen.split_whitespace().collect();

    if fields.len() < 6 {
        return Err("Invalid FEN: expected 6 fields".to_string());
    }

    // -------------------------------------------------
    // Piece placement
    // -------------------------------------------------

    let mut piece_arr: [Piece; 64] = [Piece::Empty; 64];

    for (fen_rank, rank_data) in fields[0].split('/').enumerate() {
        if fen_rank >= 8 {
            return Err("Invalid FEN: too many ranks".to_string());
        }

        // FEN starts at rank 8, while our board starts at rank 1.
        let board_rank = 7 - fen_rank;

        let mut file = 0usize;

        for c in rank_data.chars() {
            if c.is_ascii_digit() {
                let empty_squares = c.to_digit(10).unwrap() as usize;
                file += empty_squares;
            } else {
                if file >= 8 {
                    return Err("Invalid FEN: too many squares in rank".to_string());
                }

                let piece = match c {
                    'P' => Piece::WPawn,
                    'N' => Piece::WKnight,
                    'B' => Piece::WBishop,
                    'R' => Piece::WRook,
                    'Q' => Piece::WQueen,
                    'K' => Piece::WKing,

                    'p' => Piece::BPawn,
                    'n' => Piece::BKnight,
                    'b' => Piece::BBishop,
                    'r' => Piece::BRook,
                    'q' => Piece::BQueen,
                    'k' => Piece::BKing,

                    _ => {
                        return Err(format!(
                            "Invalid FEN: unknown piece '{}'",
                            c
                        ))
                    }
                };

                let board_index = board_rank * 8 + file;
                piece_arr[board_index] = piece;

                file += 1;
            }
        }

        if file != 8 {
            return Err(format!(
                "Invalid FEN: rank {} does not contain 8 squares",
                8 - fen_rank
            ));
        }
    }

    // -------------------------------------------------
    // Side to move
    // -------------------------------------------------

    let side_to_move = match fields[1] {
        "w" => Color::White,
        "b" => Color::Black,
        _ => return Err("Invalid FEN: invalid side to move".to_string()),
    };

    // -------------------------------------------------
    // Castling rights
    // -------------------------------------------------

    let castling = fields[2];

    let white_king_castling_possible = castling.contains('K');
    let white_queen_castling_possible = castling.contains('Q');
    let black_king_castling_possible = castling.contains('k');
    let black_queen_castling_possible = castling.contains('q');

    // -------------------------------------------------
    // En passant target square
    // -------------------------------------------------

    let en_passant_target_square = if fields[3] == "-" {
        None
    } else {
        let bytes = fields[3].as_bytes();

        if bytes.len() != 2 {
            return Err("Invalid FEN: invalid en-passant square".to_string());
        }

        let file = match bytes[0] {
            b'a'..=b'h' => (bytes[0] - b'a') as u8,
            _ => return Err("Invalid FEN: invalid en-passant file".to_string()),
        };

        let rank = match bytes[1] {
            b'1'..=b'8' => (bytes[1] - b'1') as u8,
            _ => return Err("Invalid FEN: invalid en-passant rank".to_string()),
        };

        Some(rank * 8 + file)
    };

    // -------------------------------------------------
    // Create board
    // -------------------------------------------------

    Ok(Board {
        pieces: piece_arr,

        en_passant_target_square,

        side_to_move,

        white_queen_castling_possible,
        white_king_castling_possible,
        black_queen_castling_possible,
        black_king_castling_possible,
    })
}

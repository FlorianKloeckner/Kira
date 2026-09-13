use anyhow::{anyhow, Result};
use reqwest::blocking::Client;
use serde_json::Value;
use core::error;
use std::{collections::HashMap, io::Read};

use crate::{Color, Move, MoveType, board::Board, offset_square, piece::Piece, requests, translate_move_uci::{self, square_to_algebraic}};

const URL:&str = "http://localhost:12345";


pub fn update_board(board: &mut Board) -> Option<Board>{
    //get board gets almost everything, we just want to keep the old position history 
    //and add the current positon on it
    let mut new_board = get_board()?;
    new_board.position_history_hashed = board.position_history_hashed.clone();
    new_board.add_current_position_to_history();

    Some(new_board)
    

}
//add check if server is reachable
fn get_board() -> Option<Board> {
    if !is_server_online() {
        eprintln!("Server is not online!");
        //maybe start server?
        return None
    }
    let mut map = HashMap::new();
    map.insert("type", "GET_BOARD");
    map.insert("id", "1");

    let client = reqwest::blocking::Client::new();

    let text = client
        .post(URL)
        .header("Content-Type", "application/json")
        .json(&map)
        .send()
        .ok()?
        .text()
        .ok()?;
    println!("Extracted text: {}", text);
    // serde deserialization needs type annotations, because it can also deserialize
    // into custom structs
    // if the json wasnt this odd, I might have done that...
    let json: Value = serde_json::from_str(&text).ok()?; 
    let side_to_move = json["response"]["board"]["activePlayer"].as_str()?;
    let side_to_move = match side_to_move {
        "WHITE" => Color::White,
        "BLACK" => Color::Black,
        _ => {eprintln!("Invalid color in json: {}", side_to_move);
            return None;}
    };
    let json_pieces = json["response"]["board"]["pieces"].as_array()?;
    println!("Pieces arr: {:?}", json_pieces);
    let mut pieces = [Piece::Empty; 64];
    let mut en_passant_target_square:Option<u8> = None;
    for p in json_pieces {
        let coords = p["coordinates"].as_str()?.to_ascii_lowercase();
        let square = translate_move_uci::algebraic_to_square(&coords)
        .or_else(|| {
            eprintln!("could not convert {coords} to square number");
            None})?;
        let piece_type = p["type"].as_str()
            .or_else(|| {
                eprintln!("Could not deserialize piece_type from json");
                None
            })?;
        let piece_color = p["player"].as_str()
            .or_else(|| {
                eprintln!("Could not deserialize player from json");
                None
            })?;
        let piece = piece_name_color_to_piece(&piece_type, &piece_color)
            .or_else(|| {
                eprintln!("Could not get piece from color {piece_color} and type: {piece_type}");
                None
            })?;        
        if piece == Piece::WPawn || piece == Piece::BPawn {
            let en_pass_possible = p["enPassePossible"].as_bool()
            .or_else(|| {
                eprintln!("Could not deserialize en_pass_possible from json");
                None
            })?;
            if en_pass_possible{
                en_passant_target_square = Some(get_en_pass_target_square(square, side_to_move.invert()));
            }
        }
        pieces[square as usize] = piece;

    }

    //we cannot return the whole position history, since the server does not return 
    //all moves
    println!("Side to move: {:?}", side_to_move);
    //figure out castling rights
    let mut white_queen_castling_possible = false;
    let mut white_king_castling_possible = false;
    let mut black_queen_castling_possible = false;
    let mut black_king_castling_possible = false;

    if !has_piece_on_square_moved(&json_pieces, "a1") &&
        !has_piece_on_square_moved(&json_pieces, "e1") &&
        pieces[0] == Piece::WRook &&
        pieces[4] == Piece::WKing {
            white_queen_castling_possible = true;
        }
    
    if !has_piece_on_square_moved(&json_pieces, "e1") &&
        !has_piece_on_square_moved(&json_pieces, "h1") &&
        pieces[4] == Piece::WKing &&
        pieces[7] == Piece::WRook{
            white_king_castling_possible = true;
        }
    if !has_piece_on_square_moved(&json_pieces, "a8") &&
        !has_piece_on_square_moved(&json_pieces, "e8") &&
        pieces[56] == Piece::BRook &&
        pieces[60] == Piece::BKing{
            black_queen_castling_possible = true;
        }
    if !has_piece_on_square_moved(&json_pieces, "e8") &&
        !has_piece_on_square_moved(&json_pieces, "h8") &&
        pieces[60] == Piece::BKing &&
        pieces[63] == Piece::BRook{
            black_king_castling_possible = true;
        }

    Some(
        Board{
        pieces,
        side_to_move,
        en_passant_target_square,
        white_king_castling_possible,
        white_queen_castling_possible,
        black_king_castling_possible,
        black_queen_castling_possible,
        position_history_hashed: Vec::new(),
    })
} 
//color is the color of the piece on square
fn get_en_pass_target_square(square: u8, color: Color) -> u8{
    offset_square(square, 0, (color.value()*-1) as i8).expect("could not offset square: {square} by rank: {color.value}")
}

fn has_piece_on_square_moved(pieces: &Vec<Value>, square: &str) -> bool{
    for p in pieces.iter(){
        let coords = p["coordinates"].as_str();
        match coords {
            Some(c) => {
                if c.to_ascii_lowercase() == square {
                    let has_moved = p["hasMoved"].as_bool();
                    match has_moved {
                        Some(b) => return b,
                        None => return false,
                    }
                    
                } 
            }
            None => continue,
        }
    }
    
    true
}

fn piece_name_color_to_piece(piece_name: &str, piece_color: &str) -> Option<Piece>{
    match (piece_name, piece_color) {
        ("PAWN", "WHITE") => Some(Piece::WPawn),
        ("PAWN", "BLACK") => Some(Piece::BPawn),
        ("BISHOP", "WHITE") => Some(Piece::WKnight), //Knight and Bishop are swapped in the api lol
        ("BISHOP", "BLACK") => Some(Piece::BKnight),
        ("KNIGHT", "WHITE") => Some(Piece::WBishop),
        ("KNIGHT", "BLACK") => Some(Piece::BBishop),
        ("ROOK", "WHITE") => Some(Piece::WRook),
        ("ROOK", "BLACK") => Some(Piece::BRook),
        ("QUEEN", "WHITE") => Some(Piece::WQueen),
        ("QUEEN", "BLACK") => Some(Piece::BQueen),
        ("KING", "WHITE") => Some(Piece::WKing),
        ("KING", "BLACK") => Some(Piece::BKing),
        (_,_) => None,
    }
}

fn is_server_online() -> bool {
    //send a simple request to the server and check if we get a response
    let client = reqwest::blocking::Client::new();
    let res = client.get(URL).send();
    match res {
        Ok(_) => true,
        Err(_) => false,
    }
}

//parses api response text into Board or returns null on error
fn parse_response(text: &str) -> Option<Board> {

    let mut res_board = Board{
        side_to_move: Color::White,
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
        white_queen_castling_possible: true,
        white_king_castling_possible: true,
        black_king_castling_possible: true,
        black_queen_castling_possible: true,
        position_history_hashed: Vec::new()
    };
    let val = serde_json::from_str(text);
    let mut json = Value::Null;
    
    match val {
        Err(e) => {
            eprintln!("{e}");
            return None;
        }
        Ok(v) => {json = v;}
    }
    println!("{json}");
    let json_board = &json["response"]["board"];
    if let Some(active_player) = json_board["active_player"].as_str() {
        match active_player {
            "WHITE" => res_board.side_to_move = Color::White,
            "BLACK" => res_board.side_to_move = Color::Black,
            _ => panic!()
        }
    }

    None
}

pub fn send_move(m: &Move, color: Color) -> Result<String>{
    let client = Client::new();
    let move_json = match m.move_type {
        MoveType::Promotion(new_piece) => serde_json::json!({
            "type": "PAWN_REACHES_END_MOVE",
            "from": square_to_algebraic(m.from).to_ascii_uppercase(),
            "to": square_to_algebraic(m.to).to_ascii_uppercase(), 
            "newPiece": &new_piece.uppercase_str()
        }),
        MoveType::CastleKingside| MoveType::CastleQueenside => {
            let rook_square = match m.move_type {
                MoveType::CastleKingside => m.to +1,
                MoveType::CastleQueenside => m.to -2,
                _ => return Err(anyhow!("Something went very wrong!!"))
            };
            serde_json::json!({
            "type": "CASTLING_MOVE",
            "king": square_to_algebraic(m.from).to_ascii_uppercase(),
            "rook":&square_to_algebraic(rook_square).to_ascii_uppercase(),
            
        })},
        _ => serde_json::json!({
            "type": "NORMAL_MOVE",
            "from": square_to_algebraic(m.from).to_ascii_uppercase(),
            "to": square_to_algebraic(m.to).to_ascii_uppercase()
        }),
    };
    let payload = serde_json::json!({
        "type": "MOVE",
        "password": color.password(),
        "move": move_json
    });
    let response = client
        .post(URL)
        .json(&payload)
        .send()?;

    let txt = response.text()?;
    eprintln!("{txt}");
    let dict: Value = serde_json::from_str(&txt)?;
    if let Some(err_msg) = dict["response"].get("error") {
        return Err(anyhow!(err_msg.to_string()));
    }
    Ok(txt)
}
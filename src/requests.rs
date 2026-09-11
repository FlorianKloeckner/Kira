use anyhow::Result;
use reqwest::blocking::Client;
use serde_json::Value;
use std::{collections::HashMap, io::Read};

use crate::{Color, board::Board, piece::Piece, requests};

const URL:&str = "http://localhost:12345";


pub fn get_board() -> Option<Board> {
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

    // serde deserialization needs type annotations, because it can also deserialize
    // into custom structs
    // if the json wasnt this odd, I might have done that...
    let json: Value = serde_json::from_str(&text).ok()?; 
    let side_to_move = json["Color"].as_str()?;
    println!("Side to move: {}", side_to_move);
    
    None
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
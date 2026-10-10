#!/usr/bin/env python3
"""
Live chess viewer for the Kira chess server.

Modes:
  python viewer.py              → launches a web UI on http://localhost:8080
  python viewer.py --terminal   → terminal-only viewer

Both modes connect to the chess server at http://localhost:12345.
"""

import json
import time
import os
import sys
import threading
import webbrowser
import urllib.request
import urllib.error
from http.server import HTTPServer, BaseHTTPRequestHandler

CHESS_SERVER_URL = "http://localhost:12345"
WEB_PORT = 8080
POLL_INTERVAL = 1.0  # seconds

# Unicode chess piece symbols (for terminal mode)
PIECE_SYMBOLS = {
    ("WHITE", "KING"):   "♔",
    ("WHITE", "QUEEN"):  "♕",
    ("WHITE", "ROOK"):   "♖",
    ("WHITE", "BISHOP"): "♗",
    ("WHITE", "KNIGHT"): "♘",
    ("WHITE", "PAWN"):   "♙",
    ("BLACK", "KING"):   "♚",
    ("BLACK", "QUEEN"):  "♛",
    ("BLACK", "ROOK"):   "♜",
    ("BLACK", "BISHOP"): "♝",
    ("BLACK", "KNIGHT"): "♞",
    ("BLACK", "PAWN"):   "♟",
}

# ─────────────────────────────────────────────────────────────
# Shared helpers (used by both terminal and web modes)
# ─────────────────────────────────────────────────────────────

def post_json(url, data):
    """POST JSON to a URL and return parsed response."""
    req = urllib.request.Request(
        url,
        data=json.dumps(data).encode("utf-8"),
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    with urllib.request.urlopen(req, timeout=5) as resp:
        return json.loads(resp.read().decode("utf-8"))


def fetch_board():
    """Call GET_BOARD on the chess server; return the board dict or None."""
    try:
        envelope = post_json(CHESS_SERVER_URL, {"type": "GET_BOARD", "id": 1})
        res = envelope.get("response", {})
        if res.get("type") == "GET_BOARD":
            return res.get("board", {})
    except Exception:
        pass
    return None


def fetch_possible_moves(pieces, active_player, time_since=0):
    """Call POSSIBLE_MOVES; return list of move dicts or None."""
    try:
        envelope = post_json(CHESS_SERVER_URL, {
            "type": "POSSIBLE_MOVES",
            "board": {
                "pieces": pieces,
                "timeSincePieceTaken": time_since,
                "activePlayer": active_player,
            },
        })
        return envelope.get("response", {}).get("possibleMoves", [])
    except Exception:
        return None


def is_king_attacked(pieces, player):
    """Is `player`'s king attacked? Uses POSSIBLE_MOVES from the opponent."""
    king_square = None
    for p in pieces:
        if p["player"] == player and p["type"] == "KING":
            king_square = p["coordinates"]
            break
    if king_square is None:
        return None

    opponent = "BLACK" if player == "WHITE" else "WHITE"
    opp_moves = fetch_possible_moves(pieces, opponent)
    if opp_moves is None:
        return None

    for m in opp_moves:
        target = m.get("to") or m.get("king")
        if target and target.upper() == king_square.upper():
            return True
    return False


def determine_game_state(board_data):
    """Return (state_key, description) for the current board."""
    pieces = board_data.get("pieces", [])
    active = board_data.get("activePlayer", "WHITE")
    game_over = board_data.get("gameIsOver", False)
    time_since = board_data.get("timeSincePieceTaken", 0)

    if not game_over:
        in_check = is_king_attacked(pieces, active)
        if in_check:
            return "check", f"{active} is in CHECK!"
        return "playing", "Game in progress"

    non_king = [p for p in pieces if p["type"] != "KING"]
    if len(non_king) == 0:
        return "draw", "DRAW — Insufficient material (kings only)"

    if time_since >= 20:
        return "draw", "DRAW — 20 moves without capture"

    in_check = is_king_attacked(pieces, active)
    if in_check:
        winner = "BLACK" if active == "WHITE" else "WHITE"
        return "checkmate", f"CHECKMATE! {winner} wins!"
    else:
        return "stalemate", "STALEMATE — No legal moves"


# ─────────────────────────────────────────────────────────────
# Terminal viewer (original mode)
# ─────────────────────────────────────────────────────────────

def clear_screen():
    os.system("cls" if os.name == "nt" else "clear")


def parse_coordinate(coord_str):
    col = ord(coord_str[0].upper()) - ord("A")
    row = 8 - int(coord_str[1])
    return row, col


def render_board_terminal(pieces):
    grid = [["·" for _ in range(8)] for _ in range(8)]
    for piece in pieces:
        r, c = parse_coordinate(piece["coordinates"])
        symbol = PIECE_SYMBOLS.get((piece["player"], piece["type"]), "?")
        grid[r][c] = symbol

    print("  ┌─────────────────┐")
    for rank in range(8):
        row_str = " ".join(grid[rank])
        print(f"{8 - rank} │ {row_str} │")
    print("  └─────────────────┘")
    print("    A B C D E F G H\n")


def terminal_main():
    print(f"Connecting to Chess Server at {CHESS_SERVER_URL} ...")
    prev_hash = None

    while True:
        try:
            board_data = fetch_board()
            if board_data is None:
                raise urllib.error.URLError("no response")

            pieces = board_data.get("pieces", [])
            active = board_data.get("activePlayer", "?")
            game_over = board_data.get("gameIsOver", False)
            time_since = board_data.get("timeSincePieceTaken", 0)

            board_hash = json.dumps(pieces, sort_keys=True)
            if board_hash == prev_hash:
                time.sleep(POLL_INTERVAL)
                continue
            prev_hash = board_hash

            state_key, state_desc = determine_game_state(board_data)

            clear_screen()
            print("═" * 40)
            print("       CHESS SERVER LIVE VIEWER")
            print("═" * 40)
            render_board_terminal(pieces)
            print(f"  Turn             : {active}")
            print(f"  Moves w/o capture: {time_since}")
            print(f"  Piece count      : {len(pieces)}")
            print()
            print(f"  Status: {state_desc}")
            print()
            print("═" * 40)
            if game_over:
                print("  Game has ended. Press Ctrl+C to exit.")
            else:
                print("  Polling... Press Ctrl+C to exit.")

        except urllib.error.URLError:
            clear_screen()
            print(f"[OFFLINE] Cannot reach chess server")
            print(f"Retrying {CHESS_SERVER_URL} ...")
            prev_hash = None
        except Exception as e:
            print(f"Error: {e}")

        time.sleep(POLL_INTERVAL)


# ─────────────────────────────────────────────────────────────
# Web viewer
# ─────────────────────────────────────────────────────────────

HTML_PAGE = r"""<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Kira — Chess Viewer</title>
<style>
  :root {
    --sq-light: #f0d9b5;
    --sq-dark:  #b58863;
    --sq-light-highlight: #f7ec5a;
    --sq-dark-highlight:  #dac34b;
    --bg:       #1a1a2e;
    --card:     #16213e;
    --accent:   #e94560;
    --text:     #eee;
    --text-dim: #8899aa;
  }

  * { margin: 0; padding: 0; box-sizing: border-box; }

  body {
    background: var(--bg);
    color: var(--text);
    font-family: 'Segoe UI', system-ui, -apple-system, sans-serif;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    min-height: 100vh;
    padding: 30px 20px;
  }

  .container {
    display: flex;
    gap: 32px;
    align-items: flex-start;
    flex-wrap: wrap;
    justify-content: center;
  }

  /* ── Board ── */
  .board-wrap {
    background: var(--card);
    border-radius: 12px;
    padding: 20px;
    box-shadow: 0 8px 32px rgba(0,0,0,.4);
  }

  .board-label-top, .board-label-bottom {
    display: grid;
    grid-template-columns: 32px repeat(8, 1fr);
    height: 24px;
    align-items: center;
  }
  .board-label-top  span,
  .board-label-bottom span {
    text-align: center;
    font-size: 13px;
    color: var(--text-dim);
    font-weight: 600;
  }

  .board-row {
    display: grid;
    grid-template-columns: 32px repeat(8, 1fr);
    align-items: center;
  }

  .rank-label {
    font-size: 13px;
    color: var(--text-dim);
    font-weight: 600;
    text-align: center;
  }

  .board-grid {
    display: grid;
    grid-template-columns: repeat(8, 1fr);
    grid-template-rows: repeat(8, 1fr);
    width:  min(70vw, 520px);
    height: min(70vw, 520px);
    border-radius: 4px;
    overflow: hidden;
    border: 3px solid #222;
  }

  .square {
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: min(8vw, 54px);
    line-height: 1;
    user-select: none;
    position: relative;
    transition: background .15s;
  }

  .square.light { background: var(--sq-light); }
  .square.dark  { background: var(--sq-dark);  }
  .square.light.check-square { background: #ff6b6b; }
  .square.dark.check-square  { background: #e05555; }

  .piece-char {
    filter: drop-shadow(1px 1px 2px rgba(0,0,0,.35));
    transition: transform .15s;
  }

  .square:hover .piece-char {
    transform: scale(1.08);
  }

  /* Coordinate overlays on the board squares */
  .coord-file {
    position: absolute; bottom: 2px; right: 4px;
    font-size: 10px; font-weight: 700;
    pointer-events: none;
  }
  .coord-rank {
    position: absolute; top: 2px; left: 4px;
    font-size: 10px; font-weight: 700;
    pointer-events: none;
  }
  .square.light .coord-file,
  .square.light .coord-rank { color: var(--sq-dark); }
  .square.dark  .coord-file,
  .square.dark  .coord-rank { color: var(--sq-light); }

  /* ── Sidebar ── */
  .sidebar {
    background: var(--card);
    border-radius: 12px;
    padding: 24px 28px;
    min-width: 260px;
    box-shadow: 0 8px 32px rgba(0,0,0,.4);
    display: flex;
    flex-direction: column;
    gap: 18px;
  }

  .sidebar h1 {
    font-size: 22px;
    font-weight: 700;
    letter-spacing: .5px;
  }
  .sidebar h1 span { color: var(--accent); }

  .info-row {
    display: flex;
    justify-content: space-between;
    font-size: 14px;
  }
  .info-label { color: var(--text-dim); }
  .info-value { font-weight: 600; }

  .status-badge {
    display: inline-block;
    padding: 8px 16px;
    border-radius: 8px;
    font-weight: 700;
    font-size: 15px;
    text-align: center;
  }
  .status-playing   { background: #0f3460; color: #53a8f5; }
  .status-check     { background: #5c1a1a; color: #ff8a8a; animation: pulse 1s infinite; }
  .status-checkmate { background: #1a5c1a; color: #8aff8a; }
  .status-stalemate { background: #3d3d00; color: #e6e68a; }
  .status-draw      { background: #3d3d00; color: #e6e68a; }
  .status-offline   { background: #333;    color: #888;     }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50%      { opacity: .7; }
  }

  .captured {
    font-size: 22px;
    letter-spacing: 2px;
    min-height: 30px;
    line-height: 1.4;
    word-break: break-all;
  }
  .captured-label { font-size: 12px; color: var(--text-dim); margin-bottom: 2px; }

  .connection-dot {
    display: inline-block;
    width: 8px; height: 8px;
    border-radius: 50%;
    margin-right: 6px;
  }
  .dot-online  { background: #4caf50; box-shadow: 0 0 6px #4caf50; }
  .dot-offline { background: #666; }

  /* ── Captured pieces ── */
  .captured-section {
    border-top: 1px solid #2a2a4a;
    padding-top: 14px;
  }

  .move-counter {
    font-size: 42px;
    font-weight: 800;
    text-align: center;
    color: var(--accent);
  }
  .move-counter-label {
    font-size: 11px;
    text-align: center;
    color: var(--text-dim);
    text-transform: uppercase;
    letter-spacing: 1px;
  }
</style>
</head>
<body>

<div class="container">
  <div class="board-wrap">
    <div id="board-grid" class="board-grid"></div>
  </div>

  <div class="sidebar">
    <h1>♚ <span>Kira</span> Viewer</h1>

    <div>
      <span class="connection-dot dot-offline" id="conn-dot"></span>
      <span id="conn-text" style="font-size:13px; color:var(--text-dim)">Connecting…</span>
    </div>

    <div id="status-box" class="status-badge status-offline">Waiting for server…</div>

    <div class="info-row">
      <span class="info-label">Turn</span>
      <span class="info-value" id="info-turn">—</span>
    </div>
    <div class="info-row">
      <span class="info-label">Pieces</span>
      <span class="info-value" id="info-pieces">—</span>
    </div>
    <div class="info-row">
      <span class="info-label">Moves w/o capture</span>
      <span class="info-value" id="info-nocap">—</span>
    </div>

    <div class="captured-section">
      <div class="captured-label">Captured by White</div>
      <div class="captured" id="cap-white"></div>
    </div>
    <div class="captured-section">
      <div class="captured-label">Captured by Black</div>
      <div class="captured" id="cap-black"></div>
    </div>
  </div>
</div>

<script>
const CHESS_URL = "/api/state";
const POLL_MS  = 1000;

const PIECE_CHARS = {
  WHITE_KING:   "♔", WHITE_QUEEN:  "♕", WHITE_ROOK:   "♖",
  WHITE_BISHOP: "♗", WHITE_KNIGHT: "♘", WHITE_PAWN:   "♙",
  BLACK_KING:   "♚", BLACK_QUEEN:  "♛", BLACK_ROOK:   "♜",
  BLACK_BISHOP: "♝", BLACK_KNIGHT: "♞", BLACK_PAWN:   "♟",
};

// Full starting set of pieces for computing captures
const START_PIECES = {
  WHITE_PAWN: 8, WHITE_KNIGHT: 2, WHITE_BISHOP: 2,
  WHITE_ROOK: 2, WHITE_QUEEN: 1, WHITE_KING: 1,
  BLACK_PAWN: 8, BLACK_KNIGHT: 2, BLACK_BISHOP: 2,
  BLACK_ROOK: 2, BLACK_QUEEN: 1, BLACK_KING: 1,
};

const CAPTURE_ORDER = ["QUEEN","ROOK","BISHOP","KNIGHT","PAWN"];

function coordToIndex(coord) {
  const col = coord.charCodeAt(0) - 65;  // A=0
  const row = 8 - parseInt(coord[1]);    // 8→0, 1→7
  return { row, col };
}

function buildBoard(pieces, checkSquare) {
  const grid = document.getElementById("board-grid");
  grid.innerHTML = "";

  const board = Array.from({length: 8}, () => Array(8).fill(null));
  for (const p of pieces) {
    const {row, col} = coordToIndex(p.coordinates);
    board[row][col] = p;
  }

  for (let r = 0; r < 8; r++) {
    for (let c = 0; c < 8; c++) {
      const sq = document.createElement("div");
      const isLight = (r + c) % 2 === 0;
      sq.className = "square " + (isLight ? "light" : "dark");

      // Highlight king in check
      const coordStr = String.fromCharCode(65 + c) + (8 - r);
      if (checkSquare && coordStr === checkSquare) {
        sq.classList.add("check-square");
      }

      // Coordinate labels on edge squares
      if (r === 7) {
        const fl = document.createElement("span");
        fl.className = "coord-file";
        fl.textContent = String.fromCharCode(97 + c);
        sq.appendChild(fl);
      }
      if (c === 0) {
        const rl = document.createElement("span");
        rl.className = "coord-rank";
        rl.textContent = 8 - r;
        sq.appendChild(rl);
      }

      if (board[r][c]) {
        const span = document.createElement("span");
        span.className = "piece-char";
        const key = board[r][c].player + "_" + board[r][c].type;
        span.textContent = PIECE_CHARS[key] || "?";
        sq.appendChild(span);
      }

      grid.appendChild(sq);
    }
  }
}

function computeCaptured(pieces) {
  const counts = {...START_PIECES};
  for (const p of pieces) {
    const key = p.player + "_" + p.type;
    if (key in counts) counts[key]--;
  }
  // White captured = missing black pieces (captured BY white)
  let capByWhite = "", capByBlack = "";
  for (const t of CAPTURE_ORDER) {
    const bk = "BLACK_" + t;
    for (let i = 0; i < counts[bk]; i++) capByWhite += PIECE_CHARS[bk];
    const wk = "WHITE_" + t;
    for (let i = 0; i < counts[wk]; i++) capByBlack += PIECE_CHARS[wk];
  }
  return { capByWhite, capByBlack };
}

let prevHash = "";

async function poll() {
  try {
    const resp = await fetch(CHESS_URL);
    const data = await resp.json();

    if (data.error) {
      setOffline(data.error);
      return;
    }

    // Connection indicator
    document.getElementById("conn-dot").className = "connection-dot dot-online";
    document.getElementById("conn-text").textContent = "Connected";

    const board = data.board;
    const pieces = board.pieces;
    const active = board.activePlayer;
    const gameOver = board.gameIsOver;
    const timeSince = board.timeSincePieceTaken;
    const stateKey = data.stateKey;
    const stateDesc = data.stateDesc;
    const checkSquare = data.checkSquare || null;

    // Only redraw if something changed
    const hash = JSON.stringify(pieces);
    if (hash === prevHash) return;
    prevHash = hash;

    buildBoard(pieces, checkSquare);

    // Info
    document.getElementById("info-turn").textContent = active;
    document.getElementById("info-pieces").textContent = pieces.length;
    document.getElementById("info-nocap").textContent = timeSince;

    // Status badge
    const badge = document.getElementById("status-box");
    badge.textContent = stateDesc;
    badge.className = "status-badge status-" + stateKey;

    // Captured
    const cap = computeCaptured(pieces);
    document.getElementById("cap-white").textContent = cap.capByWhite || "—";
    document.getElementById("cap-black").textContent = cap.capByBlack || "—";

  } catch (e) {
    setOffline("Cannot reach viewer server");
  }
}

function setOffline(msg) {
  document.getElementById("conn-dot").className = "connection-dot dot-offline";
  document.getElementById("conn-text").textContent = msg;
  const badge = document.getElementById("status-box");
  badge.textContent = "Offline";
  badge.className = "status-badge status-offline";
}

// Initial empty board
buildBoard([], null);
setInterval(poll, POLL_MS);
poll();
</script>
</body>
</html>"""


class ViewerHandler(BaseHTTPRequestHandler):
    """Serves the HTML page and proxies board state from the chess server."""

    def log_message(self, fmt, *args):
        pass  # silence request logs

    def do_GET(self):
        if self.path == "/" or self.path == "/index.html":
            self._serve_html()
        elif self.path == "/api/state":
            self._serve_state()
        else:
            self.send_error(404)

    def _serve_html(self):
        self.send_response(200)
        self.send_header("Content-Type", "text/html; charset=utf-8")
        self.end_headers()
        self.wfile.write(HTML_PAGE.encode("utf-8"))

    def _serve_state(self):
        """Fetch board from chess server, compute game state, return JSON."""
        board_data = fetch_board()
        if board_data is None:
            payload = {"error": "Chess server offline"}
        else:
            state_key, state_desc = determine_game_state(board_data)

            # Find the king square to highlight if in check/checkmate
            check_square = None
            if state_key in ("check", "checkmate"):
                active = board_data.get("activePlayer", "WHITE")
                for p in board_data.get("pieces", []):
                    if p["player"] == active and p["type"] == "KING":
                        check_square = p["coordinates"]
                        break

            payload = {
                "board": board_data,
                "stateKey": state_key,
                "stateDesc": state_desc,
                "checkSquare": check_square,
            }

        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Access-Control-Allow-Origin", "*")
        self.end_headers()
        self.wfile.write(json.dumps(payload).encode("utf-8"))


def web_main():
    """Start the web viewer server."""
    server = HTTPServer(("0.0.0.0", WEB_PORT), ViewerHandler)
    url = f"http://localhost:{WEB_PORT}"
    print(f"Web viewer running at {url}")
    print(f"Chess server expected at {CHESS_SERVER_URL}")
    print("Press Ctrl+C to stop.\n")

    # Open browser after a short delay
    threading.Timer(0.5, lambda: webbrowser.open(url)).start()

    server.serve_forever()


# ─────────────────────────────────────────────────────────────
# Entry point
# ─────────────────────────────────────────────────────────────

if __name__ == "__main__":
    try:
        if "--terminal" in sys.argv:
            terminal_main()
        else:
            web_main()
    except KeyboardInterrupt:
        print("\nViewer terminated.")
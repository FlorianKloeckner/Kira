"""
Pygame GUI for playing chess against an external engine (e.g. your Rust bot).

- python-chess handles rules/legality/state.
- pygame handles rendering + mouse input. Pieces are drawn as vector shapes
  (no image files / no font glyph dependency, so this always renders the
  same regardless of what's installed on the system).
- The bot is any external process that speaks a trivial line-based protocol
  over stdin/stdout (see EngineProcess below). This maps directly onto your
  Rust Move/MoveType struct:

    UCI string      -> Move { from, to, move_type }
    "e2e4"           -> Normal / PawnDoubleStep (you can tell which from the
                        board: pawn moving two ranks from its start rank)
    "e1g1"/"e1c1"    -> CastleKingside / CastleQueenside (king moving two
                        files tells you which)
    "e5d6" en passant -> EnPassant (destination square is empty + pawn +
                        previous move was a double step onto the adjacent file)
    "e7e8q"          -> Promotion(Queen) (5th char is the promoted piece)

  So on the Rust side you just need: read a line like "e2e4" or "e7e8q" from
  stdin, and after computing your move, print the same format to stdout.

Run:
    python chess_gui.py                     # human vs random bot (no engine)
    python chess_gui.py --engine ./my_bot    # human (white) vs your Rust engine
    python chess_gui.py --engine ./my_bot --bot-color white
"""

import argparse
import random
import subprocess
import sys

import chess
import pygame

SQUARE_SIZE = 90
BOARD_SIZE = SQUARE_SIZE * 8
SIDEBAR_WIDTH = 220
WINDOW_WIDTH = BOARD_SIZE + SIDEBAR_WIDTH
WINDOW_HEIGHT = BOARD_SIZE

COLOR_LIGHT = (240, 217, 181)
COLOR_DARK = (181, 136, 99)
COLOR_SELECTED = (106, 168, 79)
COLOR_LEGAL_DOT = (30, 30, 30)
COLOR_LAST_MOVE = (205, 210, 106)
COLOR_CHECK = (220, 90, 90)
COLOR_SIDEBAR_BG = (40, 40, 40)
COLOR_TEXT = (230, 230, 230)

WHITE_FILL = (250, 250, 250)
WHITE_OUTLINE = (25, 25, 25)
BLACK_FILL = (40, 40, 40)
BLACK_OUTLINE = (235, 235, 235)

PROMOTION_PIECES = [chess.QUEEN, chess.ROOK, chess.BISHOP, chess.KNIGHT]

# ---------------------------------------------------------------------------
# Vector piece rendering: every piece is a handful of polygons/circles in a
# normalized 0..1 coordinate space (0,0 = top-left of the square), scaled to
# whatever SQUARE_SIZE is at draw time. No image assets, no font glyphs.
# ---------------------------------------------------------------------------

BASE_TRAPEZOID = [(0.20, 0.74), (0.80, 0.74), (0.70, 0.88), (0.30, 0.88)]


def _pts(points, ox, oy, size):
    return [(ox + px * size, oy + py * size) for px, py in points]


def _poly(surface, points, ox, oy, size, fill, outline):
    pixel_points = _pts(points, ox, oy, size)
    pygame.draw.polygon(surface, fill, pixel_points)
    pygame.draw.polygon(surface, outline, pixel_points, width=2)


def _circle(surface, center, radius, ox, oy, size, fill, outline):
    cx, cy = ox + center[0] * size, oy + center[1] * size
    r = radius * size
    pygame.draw.circle(surface, fill, (cx, cy), r)
    pygame.draw.circle(surface, outline, (cx, cy), r, width=2)


def draw_pawn(surface, ox, oy, size, fill, outline):
    _circle(surface, (0.5, 0.32), 0.13, ox, oy, size, fill, outline)
    _poly(surface, [(0.40, 0.44), (0.60, 0.44), (0.66, 0.60), (0.34, 0.60)], ox, oy, size, fill, outline)
    _poly(surface, [(0.24, 0.72), (0.76, 0.72), (0.66, 0.84), (0.34, 0.84)], ox, oy, size, fill, outline)
    _poly(surface, [(0.22, 0.84), (0.78, 0.84), (0.78, 0.90), (0.22, 0.90)], ox, oy, size, fill, outline)


def draw_rook(surface, ox, oy, size, fill, outline):
    body = [
        (0.28, 0.38), (0.28, 0.26), (0.36, 0.26), (0.36, 0.32), (0.44, 0.32), (0.44, 0.26),
        (0.56, 0.26), (0.56, 0.32), (0.64, 0.32), (0.64, 0.26), (0.72, 0.26), (0.72, 0.38),
        (0.72, 0.74), (0.28, 0.74),
    ]
    _poly(surface, body, ox, oy, size, fill, outline)
    _poly(surface, BASE_TRAPEZOID, ox, oy, size, fill, outline)


def draw_bishop(surface, ox, oy, size, fill, outline):
    body = [(0.5, 0.14), (0.64, 0.34), (0.60, 0.52), (0.50, 0.58), (0.40, 0.52), (0.36, 0.34)]
    neck = [(0.42, 0.56), (0.58, 0.56), (0.64, 0.74), (0.36, 0.74)]
    _poly(surface, body, ox, oy, size, fill, outline)
    _poly(surface, neck, ox, oy, size, fill, outline)
    _poly(surface, BASE_TRAPEZOID, ox, oy, size, fill, outline)
    _circle(surface, (0.5, 0.12), 0.035, ox, oy, size, fill, outline)


def draw_knight(surface, ox, oy, size, fill, outline):
    body = [
        (0.26, 0.74), (0.26, 0.58), (0.30, 0.46), (0.27, 0.36), (0.32, 0.26), (0.42, 0.18),
        (0.52, 0.15), (0.60, 0.19), (0.55, 0.27), (0.66, 0.29), (0.73, 0.37), (0.70, 0.45),
        (0.60, 0.42), (0.58, 0.50), (0.67, 0.56), (0.63, 0.64), (0.74, 0.74),
    ]
    _poly(surface, body, ox, oy, size, fill, outline)
    _poly(surface, BASE_TRAPEZOID, ox, oy, size, fill, outline)
    _circle(surface, (0.43, 0.28), 0.02, ox, oy, size, outline, outline)


def draw_queen(surface, ox, oy, size, fill, outline):
    body = [
        (0.24, 0.58), (0.32, 0.26), (0.41, 0.44), (0.50, 0.20), (0.59, 0.44), (0.68, 0.26), (0.76, 0.58),
        (0.70, 0.74), (0.30, 0.74),
    ]
    _poly(surface, body, ox, oy, size, fill, outline)
    _poly(surface, BASE_TRAPEZOID, ox, oy, size, fill, outline)
    for peak in [(0.32, 0.24), (0.50, 0.17), (0.68, 0.24)]:
        _circle(surface, peak, 0.025, ox, oy, size, fill, outline)


def draw_king(surface, ox, oy, size, fill, outline):
    body = [(0.24, 0.58), (0.30, 0.34), (0.50, 0.28), (0.70, 0.34), (0.76, 0.58), (0.70, 0.74), (0.30, 0.74)]
    _poly(surface, body, ox, oy, size, fill, outline)
    _poly(surface, BASE_TRAPEZOID, ox, oy, size, fill, outline)
    _poly(surface, [(0.47, 0.06), (0.53, 0.06), (0.53, 0.22), (0.47, 0.22)], ox, oy, size, fill, outline)
    _poly(surface, [(0.40, 0.10), (0.60, 0.10), (0.60, 0.16), (0.40, 0.16)], ox, oy, size, fill, outline)


PIECE_DRAW_FUNCS = {
    chess.PAWN: draw_pawn,
    chess.ROOK: draw_rook,
    chess.BISHOP: draw_bishop,
    chess.KNIGHT: draw_knight,
    chess.QUEEN: draw_queen,
    chess.KING: draw_king,
}


def draw_piece(surface, piece, ox, oy, size):
    fill, outline = (WHITE_FILL, WHITE_OUTLINE) if piece.color == chess.WHITE else (BLACK_FILL, BLACK_OUTLINE)
    PIECE_DRAW_FUNCS[piece.piece_type](surface, ox, oy, size, fill, outline)


# ---------------------------------------------------------------------------
# Engine plumbing
# ---------------------------------------------------------------------------

class EngineProcess:
    """Wraps an external engine process speaking line-based UCI-move I/O.

    Protocol (deliberately minimal):
      -> "position <fen>\\n"   (Python writes the current FEN)
      <- "<uci_move>\\n"       (engine writes back one move, e.g. "e2e4" or "e7e8q")
    Adjust this to match whatever protocol you implement in Rust.
    """

    def __init__(self, path):
        self.process = subprocess.Popen(
            [path],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            text=True,
            bufsize=1,
        )

    def get_move(self, board, last_human_move):
        # The engine always speaks first: on the very first call (no human
        # move exists yet) we only read, we never write. On every later
        # call, we write the human's move (fen included for board sync /
        # future use, though this engine currently only reads the move
        # token off the end of the line) and then read the engine's reply.
        if last_human_move is not None:
            self.process.stdin.write(f"position {board.fen()} move {last_human_move.uci()}\n")
            self.process.stdin.flush()
        line = self.process.stdout.readline().strip()
        if not line:
            raise RuntimeError("engine process closed its output unexpectedly")
        return chess.Move.from_uci(line)

    def close(self):
        try:
            self.process.terminate()
        except Exception:
            pass


class RandomBot:
    """Fallback bot so the GUI is runnable without a real engine attached."""

    def get_move(self, board, last_human_move=None):
        return random.choice(list(board.legal_moves))

    def close(self):
        pass


def square_to_pixel(square, flipped):
    file_idx = chess.square_file(square)
    rank_idx = chess.square_rank(square)
    if flipped:
        col, row = 7 - file_idx, rank_idx
    else:
        col, row = file_idx, 7 - rank_idx
    return col * SQUARE_SIZE, row * SQUARE_SIZE


def pixel_to_square(x, y, flipped):
    col, row = x // SQUARE_SIZE, y // SQUARE_SIZE
    if flipped:
        file_idx, rank_idx = 7 - col, row
    else:
        file_idx, rank_idx = col, 7 - row
    return chess.square(file_idx, rank_idx)


def draw_board(screen, board, selected_square, legal_targets, last_move, flipped):
    for square in chess.SQUARES:
        x, y = square_to_pixel(square, flipped)
        is_light = (chess.square_file(square) + chess.square_rank(square)) % 2 == 1
        color = COLOR_LIGHT if is_light else COLOR_DARK

        if last_move and square in (last_move.from_square, last_move.to_square):
            color = COLOR_LAST_MOVE
        if board.is_check() and board.piece_at(square) == chess.Piece(chess.KING, board.turn):
            color = COLOR_CHECK
        if square == selected_square:
            color = COLOR_SELECTED

        pygame.draw.rect(screen, color, (x, y, SQUARE_SIZE, SQUARE_SIZE))

        piece = board.piece_at(square)
        if piece:
            draw_piece(screen, piece, x, y, SQUARE_SIZE)

    for square in legal_targets:
        x, y = square_to_pixel(square, flipped)
        pygame.draw.circle(
            screen, COLOR_LEGAL_DOT,
            (x + SQUARE_SIZE // 2, y + SQUARE_SIZE // 2), 12,
        )


def draw_sidebar(screen, board, font_small, engine_name, human_color, thinking):
    pygame.draw.rect(screen, COLOR_SIDEBAR_BG, (BOARD_SIZE, 0, SIDEBAR_WIDTH, WINDOW_HEIGHT))
    lines = [
        f"You: {'White' if human_color == chess.WHITE else 'Black'}",
        f"Bot: {engine_name}",
        "",
        "Turn: White" if board.turn == chess.WHITE else "Turn: Black",
    ]
    if thinking:
        lines.append("(bot thinking...)")
    if board.is_checkmate():
        lines.append("Checkmate!")
    elif board.is_stalemate():
        lines.append("Stalemate")
    elif board.is_check():
        lines.append("Check!")

    for i, line in enumerate(lines):
        text = font_small.render(line, True, COLOR_TEXT)
        screen.blit(text, (BOARD_SIZE + 15, 20 + i * 28))


def ask_promotion_piece(screen, human_color):
    """Simple modal: click one of four piece icons to choose promotion."""
    options = PROMOTION_PIECES
    box_w, box_h = 4 * SQUARE_SIZE, SQUARE_SIZE
    box_x = (BOARD_SIZE - box_w) // 2
    box_y = (BOARD_SIZE - box_h) // 2

    while True:
        pygame.draw.rect(screen, (250, 250, 250), (box_x, box_y, box_w, box_h))
        for i, piece_type in enumerate(options):
            piece = chess.Piece(piece_type, human_color)
            draw_piece(screen, piece, box_x + i * SQUARE_SIZE, box_y, SQUARE_SIZE)
        pygame.display.flip()

        for event in pygame.event.get():
            if event.type == pygame.QUIT:
                pygame.quit()
                sys.exit()
            if event.type == pygame.MOUSEBUTTONDOWN:
                mx, my = event.pos
                if box_y <= my <= box_y + box_h and box_x <= mx <= box_x + box_w:
                    index = (mx - box_x) // SQUARE_SIZE
                    if 0 <= index < len(options):
                        return options[index]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--engine", help="Path to your Rust engine executable", default=None)
    parser.add_argument("--bot-color", choices=["white", "black"], default="black")
    args = parser.parse_args()

    if args.engine and args.bot_color == "black":
        print(
            "Note: this Rust engine's main loop always moves first, so it can only "
            "play White as a subprocess. Ignoring --bot-color black.",
            file=sys.stderr,
        )
        args.bot_color = "white"

    human_color = chess.BLACK if args.bot_color == "white" else chess.WHITE
    bot_color = chess.WHITE if args.bot_color == "white" else chess.BLACK
    flipped = human_color == chess.BLACK

    if args.engine:
        bot = EngineProcess(args.engine)
        engine_name = args.engine
    else:
        bot = RandomBot()
        engine_name = "random (no --engine given)"

    pygame.init()
    screen = pygame.display.set_mode((WINDOW_WIDTH, WINDOW_HEIGHT))
    pygame.display.set_caption("Chess vs Bot")
    sidebar_font = pygame.font.SysFont("arial", 20)
    clock = pygame.time.Clock()

    board = chess.Board()
    selected_square = None
    legal_targets = []
    last_move = None
    running = True

    if isinstance(bot, EngineProcess) and bot_color == chess.WHITE:
        # The engine writes its opening move unprompted, before reading anything.
        opening_move = bot.get_move(board, None)
        if opening_move not in board.legal_moves:
            print(f"Engine returned illegal opening move: {opening_move}", file=sys.stderr)
            running = False
        else:
            board.push(opening_move)
            last_move = opening_move

    while running:
        for event in pygame.event.get():
            if event.type == pygame.QUIT:
                running = False

            elif event.type == pygame.MOUSEBUTTONDOWN and board.turn == human_color and not board.is_game_over():
                mx, my = event.pos
                if mx >= BOARD_SIZE:
                    continue
                clicked_square = pixel_to_square(mx, my, flipped)

                if selected_square is None:
                    piece = board.piece_at(clicked_square)
                    if piece and piece.color == human_color:
                        selected_square = clicked_square
                        legal_targets = [
                            m.to_square for m in board.legal_moves if m.from_square == clicked_square
                        ]
                else:
                    if clicked_square == selected_square:
                        selected_square, legal_targets = None, []
                    elif clicked_square in legal_targets:
                        move = chess.Move(selected_square, clicked_square)
                        needs_promo = (
                            board.piece_at(selected_square).piece_type == chess.PAWN
                            and chess.square_rank(clicked_square) in (0, 7)
                        )
                        if needs_promo:
                            promo_piece = ask_promotion_piece(screen, human_color)
                            move = chess.Move(selected_square, clicked_square, promotion=promo_piece)
                        board.push(move)
                        last_move = move
                        selected_square, legal_targets = None, []
                    else:
                        piece = board.piece_at(clicked_square)
                        if piece and piece.color == human_color:
                            selected_square = clicked_square
                            legal_targets = [
                                m.to_square for m in board.legal_moves if m.from_square == clicked_square
                            ]
                        else:
                            selected_square, legal_targets = None, []

        screen.fill((0, 0, 0))
        draw_board(screen, board, selected_square, legal_targets, last_move, flipped)
        draw_sidebar(screen, board, sidebar_font, engine_name, human_color, thinking=False)
        pygame.display.flip()

        if board.turn == bot_color and not board.is_game_over():
            draw_sidebar(screen, board, sidebar_font, engine_name, human_color, thinking=True)
            pygame.display.flip()
            try:
                move = bot.get_move(board, last_move)
                if move not in board.legal_moves:
                    print(f"Engine returned illegal move: {move}", file=sys.stderr)
                    running = False
                else:
                    board.push(move)
                    last_move = move
            except Exception as exc:
                print(f"Engine error: {exc}", file=sys.stderr)
                running = False

        clock.tick(30)

    bot.close()
    pygame.quit()


if __name__ == "__main__":
    main()
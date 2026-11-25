import { Chessground } from './vendor/chessground.min.js';
import { Chess } from './vendor/chess.js';

const $ = (id) => document.getElementById(id);
const els = {
  board: $('board'),
  status: $('status'),
  moves: $('moves'),
  info: $('engine-info'),
  side: $('side'),
  newGame: $('new-game'),
  takeback: $('takeback'),
  flip: $('flip'),
  copyPgn: $('copy-pgn'),
  promotion: $('promotion'),
  promotionChoices: $('promotion-choices'),
  top: document.querySelector('.player-top'),
  bottom: document.querySelector('.player-bottom'),
  nameTop: $('name-top'),
  nameBottom: $('name-bottom'),
};

// UCI command Kira gets for every move (Kira currently thinks about 950 ms on this).
const GO_COMMAND = 'go wtime 60000 btime 60000';

const chess = new Chess();
const worker = new Worker(new URL('./engine-worker.js', import.meta.url), { type: 'module' });

let userColor = 'white';
let orientation = 'white';
let thinking = false;
let requestId = 0; // bumped on new game / takeback so stale engine replies are ignored
let engineError = null;

const cg = Chessground(els.board, {
  orientation,
  movable: { free: false, showDests: true, events: { after: onUserMove } },
  premovable: { enabled: false },
  highlight: { lastMove: true, check: true },
  animation: { enabled: true, duration: 180 },
});

// ---------------------------------------------------------------------------
// Rendering

const colorName = (c) => (c === 'w' ? 'white' : 'black');

function legalDests() {
  const dests = new Map();
  for (const m of chess.moves({ verbose: true })) {
    if (!dests.has(m.from)) dests.set(m.from, []);
    dests.get(m.from).push(m.to);
  }
  return dests;
}

function render() {
  const history = chess.history({ verbose: true });
  const last = history[history.length - 1];
  const turn = colorName(chess.turn());
  const userToMove = turn === userColor && !chess.isGameOver() && !thinking;

  cg.set({
    fen: chess.fen(),
    orientation,
    turnColor: turn,
    check: chess.inCheck() ? turn : false,
    lastMove: last ? [last.from, last.to] : undefined,
    movable: {
      color: userToMove ? userColor : undefined,
      dests: userToMove ? legalDests() : new Map(),
    },
  });

  renderPlayers();
  renderMoves(history);
  renderStatus();
  els.takeback.disabled = !history.some((m) => m.color === userColor[0]);
}

function renderPlayers() {
  const topColor = orientation === 'white' ? 'black' : 'white';
  const bottomColor = orientation;
  const label = (c) => (c === userColor ? 'You' : 'Kira');
  els.nameTop.textContent = label(topColor);
  els.nameBottom.textContent = label(bottomColor);
  els.top.className = `player player-top ${topColor}`;
  els.bottom.className = `player player-bottom ${bottomColor}`;
  const kiraRow = topColor === userColor ? els.bottom : els.top;
  kiraRow.classList.toggle('thinking', thinking);
}

function renderMoves(history) {
  els.moves.textContent = '';
  for (let i = 0; i < history.length; i += 2) {
    const li = document.createElement('li');
    const num = document.createElement('span');
    num.className = 'num';
    num.textContent = `${i / 2 + 1}.`;
    li.append(num);
    for (const j of [i, i + 1]) {
      const ply = document.createElement('span');
      ply.className = 'ply' + (j === history.length - 1 ? ' last' : '');
      ply.textContent = history[j] ? history[j].san : '';
      li.append(ply);
    }
    els.moves.append(li);
  }
  els.moves.scrollTop = els.moves.scrollHeight;
}

function gameOverText() {
  if (chess.isCheckmate()) {
    const winner = chess.turn() === 'w' ? 'black' : 'white';
    return winner === userColor ? 'Checkmate. You win!' : 'Checkmate. Kira wins.';
  }
  if (chess.isStalemate()) return 'Draw by stalemate.';
  if (chess.isThreefoldRepetition()) return 'Draw by threefold repetition.';
  if (chess.isInsufficientMaterial()) return 'Draw by insufficient material.';
  if (chess.isDrawByFiftyMoves()) return 'Draw by the 50-move rule.';
  return 'Game over.';
}

function renderStatus() {
  els.status.className = 'status';
  if (engineError) {
    els.status.classList.add('error');
    els.status.textContent = engineError;
  } else if (chess.isGameOver()) {
    els.status.classList.add('over');
    els.status.textContent = gameOverText();
  } else if (thinking) {
    els.status.textContent = 'Kira is thinking…';
  } else {
    els.status.textContent = chess.inCheck() ? 'Your move. You are in check.' : 'Your move.';
  }
}

// ---------------------------------------------------------------------------
// User moves

function onUserMove(from, to) {
  const piece = chess.get(from);
  const promotes = piece && piece.type === 'p' && (to[1] === '8' || to[1] === '1');
  if (promotes) {
    askPromotion(piece.color).then((promotion) => {
      if (promotion) playUserMove(from, to, promotion);
      else render(); // cancelled: put the pawn back
    });
  } else {
    playUserMove(from, to);
  }
}

function playUserMove(from, to, promotion) {
  try {
    chess.move({ from, to, promotion });
  } catch {
    render();
    return;
  }
  render();
  if (!chess.isGameOver()) askEngine();
}

function askPromotion(color) {
  return new Promise((resolve) => {
    els.promotionChoices.textContent = '';
    for (const [role, letter] of [['queen', 'q'], ['rook', 'r'], ['bishop', 'b'], ['knight', 'n']]) {
      const btn = document.createElement('button');
      btn.title = role[0].toUpperCase() + role.slice(1);
      btn.innerHTML = `<div class="cg-wrap"><piece class="${role} ${colorName(color)}"></piece></div>`;
      btn.addEventListener('click', (e) => {
        e.stopPropagation();
        close(letter);
      });
      els.promotionChoices.append(btn);
    }
    const close = (choice) => {
      els.promotion.hidden = true;
      els.promotion.onclick = null;
      resolve(choice);
    };
    els.promotion.onclick = () => close(null); // click outside the choices cancels
    els.promotion.hidden = false;
  });
}

// ---------------------------------------------------------------------------
// Engine

function askEngine() {
  thinking = true;
  engineError = null;
  const id = ++requestId;
  worker.postMessage({
    id,
    moves: chess.history({ verbose: true }).map((m) => m.lan),
    go: GO_COMMAND,
  });
  render();
}

worker.onmessage = ({ data }) => {
  if (data.id !== requestId) return; // reply to a position that no longer exists
  thinking = false;
  if (data.error) {
    engineError = data.error;
    render();
    return;
  }
  const uci = data.bestmove;
  try {
    chess.move({ from: uci.slice(0, 2), to: uci.slice(2, 4), promotion: uci[4] });
  } catch {
    engineError = `Kira suggested an illegal move: ${uci}`;
  }
  els.info.textContent = data.depth
    ? `Kira: ${uci}, depth ${data.depth}, ${data.ms} ms`
    : `Kira: ${uci}, ${data.ms} ms`;
  render();
};

worker.onerror = (e) => {
  thinking = false;
  engineError = `The engine could not be started: ${e.message || 'unknown error'}`;
  render();
};

// ---------------------------------------------------------------------------
// Controls

function newGame() {
  requestId++;
  thinking = false;
  engineError = null;
  chess.reset();
  const side = els.side.value;
  userColor = side === 'random' ? (Math.random() < 0.5 ? 'white' : 'black') : side;
  orientation = userColor;
  els.info.textContent = '';
  chess.setHeader('White', userColor === 'white' ? 'You' : 'Kira');
  chess.setHeader('Black', userColor === 'black' ? 'You' : 'Kira');
  chess.setHeader('Date', new Date().toISOString().slice(0, 10).replace(/-/g, '.'));
  render();
  if (userColor === 'black') askEngine();
}

function takeback() {
  requestId++; // ignore a reply that may still be on its way
  thinking = false;
  engineError = null;
  // Undo back to the position before the user's last move.
  let undone;
  do {
    undone = chess.undo();
  } while (undone && undone.color !== userColor[0]);
  render();
  if (colorName(chess.turn()) !== userColor) askEngine(); // e.g. back at move 1 as Black
}

els.newGame.addEventListener('click', newGame);
els.takeback.addEventListener('click', takeback);
els.flip.addEventListener('click', () => {
  orientation = orientation === 'white' ? 'black' : 'white';
  render();
});
els.copyPgn.addEventListener('click', async () => {
  try {
    await navigator.clipboard.writeText(chess.pgn());
    els.copyPgn.textContent = 'Copied';
  } catch {
    els.copyPgn.textContent = 'Copy failed';
  }
  setTimeout(() => (els.copyPgn.textContent = 'Copy PGN'), 1500);
});

newGame();

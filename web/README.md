# Kira-web

A web page for playing against [Kira](../Kira) in the browser. The engine runs locally on the
visitor's device as WebAssembly, so the page is fully static and can be hosted on GitHub Pages.

## How it works

- `engine/Cargo.toml` builds **the unmodified Kira source** (`../Kira/src/main.rs`) for the
  `wasm32-wasip1` target. It only lists `arrayvec`, the one dependency the engine source uses,
  so Kira's own `Cargo.toml` (tokio, reqwest, ...) is not involved.
- `site/engine-worker.js` runs `kira.wasm` in a Web Worker with a tiny WASI shim. For each move
  it starts a fresh instance and feeds Kira's normal UCI loop a pre-filled stdin:
  `position startpos moves ...`, `go ...`, `quit`. It reads `bestmove` from the output.
- `site/app.js` draws the board (chessground) and checks legality and game end (chess.js).

Because the build points at `../Kira`, this folder must sit next to the `Kira` folder.

## Build

One-time setup:

```
rustup target add wasm32-wasip1
```

Then, whenever Kira changes:

```
.\build.ps1        # Windows (PowerShell)
./build.sh         # Linux / macOS
```

This writes `site/kira.wasm`.

## Run locally

Browsers do not load WebAssembly or module workers from `file://`, so serve the folder:

```
python -m http.server 8000 -d site
```

and open <http://localhost:8000>.

## Think time

For every move the page sends Kira `go wtime 60000 btime 60000` (`GO_COMMAND` in `site/app.js`),
which Kira currently maps to about 950 ms.

## Publishing

`site/` is the whole website. To publish it on GitHub Pages, serve that folder (for example
from a `docs/` folder or a Pages workflow). No server-side code is needed.

## Licenses

- chessground (board): GPL-3.0-or-later, see `site/vendor/LICENSE-chessground.txt`
- chess.js (rules): BSD-2-Clause, see `site/vendor/LICENSE-chess.js.txt`

Because chessground is GPL, the page as a whole should be published under a GPL-compatible
license.

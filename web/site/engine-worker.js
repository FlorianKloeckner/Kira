// Runs the unmodified Kira binary (compiled to wasm32-wasip1) inside a Web Worker.
//
// Kira is a normal UCI program that reads commands from stdin. Browsers have no
// stdin, so for every move we start a fresh instance of the module and hand it a
// pre-filled stdin ("position ...", "go ...", "quit"). Everything Kira prints is
// collected, and the "bestmove" line is sent back to the page.
//
// Only the eight WASI functions Kira actually imports are implemented below.

const WASM_URL = new URL('./kira.wasm', import.meta.url);
let modulePromise = null;

function loadModule() {
  if (!modulePromise) {
    modulePromise = fetch(WASM_URL).then((r) => {
      if (!r.ok) throw new Error(`Could not load kira.wasm (HTTP ${r.status})`);
      return WebAssembly.compileStreaming
        ? WebAssembly.compileStreaming(r).catch(() => fetch(WASM_URL).then((r2) => r2.arrayBuffer()).then((b) => WebAssembly.compile(b)))
        : r.arrayBuffer().then((b) => WebAssembly.compile(b));
    });
  }
  return modulePromise;
}

class ProcExit extends Error {
  constructor(code) {
    super(`exit ${code}`);
    this.code = code;
  }
}

const ERRNO_SUCCESS = 0;
const ERRNO_BADF = 8;
const ERRNO_INVAL = 28;

async function runUci(stdinText) {
  const module = await loadModule();
  const encoder = new TextEncoder();
  const decoder = new TextDecoder();
  const stdin = encoder.encode(stdinText);
  let stdinPos = 0;
  const out = { 1: '', 2: '' };
  const args = [encoder.encode('kira\0')];
  let memory;

  const view = () => new DataView(memory.buffer);
  const bytes = () => new Uint8Array(memory.buffer);

  const wasi = {
    args_sizes_get(argcPtr, bufSizePtr) {
      view().setUint32(argcPtr, args.length, true);
      view().setUint32(bufSizePtr, args.reduce((n, a) => n + a.length, 0), true);
      return ERRNO_SUCCESS;
    },
    args_get(argvPtr, bufPtr) {
      for (const a of args) {
        view().setUint32(argvPtr, bufPtr, true);
        bytes().set(a, bufPtr);
        argvPtr += 4;
        bufPtr += a.length;
      }
      return ERRNO_SUCCESS;
    },
    environ_sizes_get(countPtr, bufSizePtr) {
      view().setUint32(countPtr, 0, true);
      view().setUint32(bufSizePtr, 0, true);
      return ERRNO_SUCCESS;
    },
    environ_get() {
      return ERRNO_SUCCESS;
    },
    clock_time_get(_clockId, _precision, timePtr) {
      // Kira measures its think time with SystemTime; wall-clock nanoseconds are fine.
      const ns = BigInt(Math.round((performance.timeOrigin + performance.now()) * 1e6));
      view().setBigUint64(timePtr, ns, true);
      return ERRNO_SUCCESS;
    },
    fd_read(fd, iovsPtr, iovsLen, nreadPtr) {
      if (fd !== 0) return ERRNO_BADF;
      let total = 0;
      for (let i = 0; i < iovsLen; i++) {
        const ptr = view().getUint32(iovsPtr + i * 8, true);
        const len = view().getUint32(iovsPtr + i * 8 + 4, true);
        const chunk = stdin.subarray(stdinPos, stdinPos + len);
        bytes().set(chunk, ptr);
        stdinPos += chunk.length;
        total += chunk.length;
        if (chunk.length < len) break;
      }
      view().setUint32(nreadPtr, total, true);
      return ERRNO_SUCCESS;
    },
    fd_write(fd, iovsPtr, iovsLen, nwrittenPtr) {
      if (fd !== 1 && fd !== 2) return ERRNO_BADF;
      let total = 0;
      for (let i = 0; i < iovsLen; i++) {
        const ptr = view().getUint32(iovsPtr + i * 8, true);
        const len = view().getUint32(iovsPtr + i * 8 + 4, true);
        out[fd] += decoder.decode(bytes().subarray(ptr, ptr + len), { stream: true });
        total += len;
      }
      view().setUint32(nwrittenPtr, total, true);
      return ERRNO_SUCCESS;
    },
    proc_exit(code) {
      throw new ProcExit(code);
    },
  };

  // Anything else Kira might import in a future version fails softly.
  const wasiImports = new Proxy(wasi, {
    get: (target, name) => (name in target ? target[name] : () => ERRNO_INVAL),
  });

  const instance = await WebAssembly.instantiate(module, { wasi_snapshot_preview1: wasiImports });
  memory = instance.exports.memory;

  try {
    instance.exports._start();
  } catch (e) {
    if (!(e instanceof ProcExit) || e.code !== 0) {
      const tail = (out[1] + out[2]).split('\n').slice(-8).join('\n');
      throw new Error(`Kira crashed: ${e.message}\n${tail}`);
    }
  }
  return out[1] + out[2];
}

self.onmessage = async ({ data }) => {
  const { id, moves, go } = data;
  try {
    const position = moves.length ? `position startpos moves ${moves.join(' ')}` : 'position startpos';
    const started = performance.now();
    const output = await runUci(`${position}\n${go}\nquit\n`);
    const match = output.match(/bestmove\s+([a-h][1-8][a-h][1-8][qrbn]?)/);
    if (!match) throw new Error(`Kira did not return a move.\n${output.slice(-400)}`);
    const depths = [...output.matchAll(/completed search with depth (\d+)/g)];
    self.postMessage({
      id,
      bestmove: match[1],
      depth: depths.length ? Number(depths[depths.length - 1][1]) : null,
      ms: Math.round(performance.now() - started),
    });
  } catch (e) {
    self.postMessage({ id, error: String(e.message || e) });
  }
};

loadModule().catch(() => {}); // start downloading right away

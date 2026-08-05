// db-bridge.js
// Interface between Rust WASM core and the SQLite Web Worker

window.yntra_persistent_storage_granted = false;

if (navigator.storage && navigator.storage.persist) {
  navigator.storage.persist().then((persistent) => {
    window.yntra_persistent_storage_granted = persistent;
    console.log(`[Yntra Storage] Persistent storage granted: ${persistent}`);
  });
}

// Sub-200ms Cold-Start Optimization: Pre-fetch & Pre-compile WASM Module with Robust MIME Fallback
const precompileWasm = async (url) => {
  try {
    if ('WebAssembly' in window && WebAssembly.compileStreaming) {
      const response = fetch(url, { cache: 'force-cache' });
      return await WebAssembly.compileStreaming(response);
    }
  } catch (e) {
    console.warn(`[WASM Pre-compile] compileStreaming failed for ${url}, trying arrayBuffer fallback:`, e);
    try {
      const res = await fetch(url, { cache: 'force-cache' });
      const buffer = await res.arrayBuffer();
      return await WebAssembly.compile(buffer);
    } catch (fallbackErr) {
      console.warn(`[WASM Pre-compile] ArrayBuffer fallback failed for ${url}:`, fallbackErr);
    }
  }
  return null;
};

// Trigger parallel WASM streaming compilation immediately on script evaluation
const wasmPreloadPromise = precompileWasm('/public/sqlite3.wasm');

const worker = new Worker('/public/db-worker.js');
const pendingRequests = new Map();
let messageId = 0;
let isDbReady = false;
const readyCallbacks = [];

wasmPreloadPromise.then((compiledModule) => {
  if (compiledModule) {
    console.log('[Yntra Cold-Start] SQLite WASM module pre-compiled in parallel.');
  }
});

worker.onmessage = function(e) {
  const { id, type, status, success, rows, rowsAffected, error, hasChanges } = e.data;
  
  if (type === "status") {
    if (status === "ready") {
      isDbReady = true;
      console.log("Database Worker is ready.");
      while (readyCallbacks.length > 0) {
        readyCallbacks.shift()();
      }
    } else if (status === "error") {
      console.error("Database Worker error:", error);
    }
    return;
  }
  
  const callbacks = pendingRequests.get(id);
  if (callbacks) {
    pendingRequests.delete(id);
    if (success) {
      if (rows !== undefined) {
        callbacks.resolve(rows);
      } else if (rowsAffected !== undefined) {
        callbacks.resolve({ rowsAffected });
      } else if (hasChanges !== undefined) {
        callbacks.resolve({ hasChanges });
      } else {
        callbacks.resolve(null);
      }
    } else {
      callbacks.reject(new Error(error));
    }
  }
};

window.yntra_execute_sql = function(type, sql, params) {
  return new Promise((resolve, reject) => {
    const id = `sql-${messageId++}`;
    
    const send = () => {
      pendingRequests.set(id, { resolve, reject });
      worker.postMessage({ id, type, sql, params: params || [] });
    };
    
    if (isDbReady) {
      send();
    } else {
      readyCallbacks.push(send);
    }
  });
};

window.yntra_save_store_bin = async function(fileName, uint8Array) {
  try {
    const root = await navigator.storage.getDirectory();
    const fileHandle = await root.getFileHandle(fileName, { create: true });
    const writable = await fileHandle.createWritable();
    await writable.write(uint8Array);
    await writable.close();
  } catch (e) {
    console.error("OPFS binary write error:", e);
  }
};

window.yntra_load_store_bin = async function(fileName) {
  try {
    const root = await navigator.storage.getDirectory();
    const fileHandle = await root.getFileHandle(fileName, { create: true });
    const file = await fileHandle.getFile();
    if (file.size > 0) {
      const buffer = await file.arrayBuffer();
      return new Uint8Array(buffer);
    }
  } catch (e) {
    console.error("OPFS binary read error:", e);
  }
  return null;
};

window.yntra_sync_db = function(url, token) {
  return new Promise((resolve, reject) => {
    const id = `sync-${messageId++}`;
    const send = () => {
      pendingRequests.set(id, { resolve, reject });
      worker.postMessage({ id, type: "sync", url, token });
    };
    
    if (isDbReady) {
      send();
    } else {
      readyCallbacks.push(send);
    }
  });
};



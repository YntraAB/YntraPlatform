// db-bridge.js
// Interface between Rust WASM core and the SQLite Web Worker

const worker = new Worker('/db-worker.js');
const pendingRequests = new Map();
let messageId = 0;
let isDbReady = false;
const readyCallbacks = [];

worker.onmessage = function(e) {
  const { id, type, status, success, rows, rowsAffected, error } = e.data;
  
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
        callbacks.resolve(JSON.stringify(rows));
      } else if (rowsAffected !== undefined) {
        callbacks.resolve(JSON.stringify({ rowsAffected }));
      } else {
        callbacks.resolve(JSON.stringify(null));
      }
    } else {
      callbacks.reject(new Error(error));
    }
  }
};

window.yntra_execute_sql = function(type, sql, params_json) {
  return new Promise((resolve, reject) => {
    const id = `sql-${messageId++}`;
    let params = [];
    if (params_json) {
      try {
        params = JSON.parse(params_json);
      } catch (e) {
        reject(new Error("Invalid parameters JSON: " + e.message));
        return;
      }
    }
    
    const send = () => {
      pendingRequests.set(id, { resolve, reject });
      worker.postMessage({ id, type, sql, params });
    };
    
    if (isDbReady) {
      send();
    } else {
      readyCallbacks.push(send);
    }
  });
};

window.yntra_save_store_bin = async function(uint8Array) {
  try {
    const root = await navigator.storage.getDirectory();
    const fileHandle = await root.getFileHandle("yntra_store.bin", { create: true });
    const writable = await fileHandle.createWritable();
    await writable.write(uint8Array);
    await writable.close();
  } catch (e) {
    console.error("OPFS binary write error:", e);
  }
};

window.yntra_load_store_bin = async function() {
  try {
    const root = await navigator.storage.getDirectory();
    const fileHandle = await root.getFileHandle("yntra_store.bin", { create: true });
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



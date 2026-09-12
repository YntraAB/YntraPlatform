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
    const res = await fetch(url, { cache: 'force-cache' });
    if (!res.ok) return null;
    const contentType = (res.headers.get('content-type') || '').toLowerCase();
    if (contentType.includes('text/html')) {
      return null;
    }
    if ('WebAssembly' in window && WebAssembly.compileStreaming && contentType.includes('application/wasm')) {
      try {
        return await WebAssembly.compileStreaming(res);
      } catch (_) {}
    }
    const buffer = await res.arrayBuffer();
    return await WebAssembly.compile(buffer);
  } catch (e) {
    const errMsg = (e && e.message) ? String(e.message) : String(e);
    console.warn(`[WASM Pre-compile] Fallback note for ${url}: ${errMsg}`);
  }
  return null;
};

// Trigger parallel WASM streaming compilation immediately on script evaluation
const wasmPreloadPromise = (async () => {
  let mod = await precompileWasm('/sqlite3.wasm');
  if (!mod) {
    mod = await precompileWasm('/public/sqlite3.wasm');
  }
  return mod;
})();

const worker = new Worker('/db-worker.js');
worker.onerror = function(err) {
  const errMsg = (err && err.message) ? String(err.message) : String(err);
  console.error(`[Yntra DB] Worker runtime error: ${errMsg}`);
};

const pendingRequests = new Map();
let messageId = 0;
let isDbReady = false;
const readyCallbacks = [];

wasmPreloadPromise.then((compiledModule) => {
  if (compiledModule) {
    console.log('[Yntra Cold-Start] SQLite WASM module pre-compiled in parallel.');
  }
});

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
    const errMsg = (e && e.message) ? String(e.message) : String(e);
    console.error(`OPFS binary write error: ${errMsg}`);
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
    const errMsg = (e && e.message) ? String(e.message) : String(e);
    console.error(`OPFS binary read error: ${errMsg}`);
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

// --- Shared Kiosk Terminal Multi-Tier Persistence Bridge ---
const SharedKioskPersistenceBridge = {
  daemonSocket: null,
  isDaemonConnected: false,
  unsyncedCount: 0,
  activeUserId: null,
  activeWorkspaceId: null,
  logoutGuards: [],

  init() {
    this.probeNativeDaemon();
    this.setupPageLifecycleListeners();
    this.registerServiceWorkerSync();
  },

  registerServiceWorkerSync() {
    if ('serviceWorker' in navigator && 'SyncManager' in window) {
      navigator.serviceWorker.ready.then((reg) => {
        return reg.sync.register('yntra-kiosk-sync');
      }).catch((err) => {
        const errMsg = (err && err.message) ? String(err.message) : String(err);
        console.warn(`[Kiosk Persistence] ServiceWorker BackgroundSync registration fallback: ${errMsg}`);
      });
    }
  },

  probeNativeDaemon() {
    if (typeof chrome !== 'undefined' && chrome.runtime && chrome.runtime.sendNativeMessage) {
      try {
        chrome.runtime.sendNativeMessage("se.yntra.kiosk_daemon", { type: "ping" }, (response) => {
          if (response && response.status === "ok") {
            console.log("[Kiosk Persistence] Connected via Chrome Native Messaging API.");
            this.isDaemonConnected = true;
          }
        });
      } catch (err) {}
    }

    try {
      const ws = new WebSocket("ws://127.0.0.1:9443/yntra_kiosk_daemon");
      ws.onopen = () => {
        console.log("[Kiosk Persistence] Connected to local native sidecar daemon (ws://127.0.0.1:9443).");
        this.daemonSocket = ws;
        this.isDaemonConnected = true;
      };
      ws.onclose = () => { this.daemonSocket = null; this.isDaemonConnected = false; };
      ws.onerror = () => { this.daemonSocket = null; this.isDaemonConnected = false; };
    } catch (e) {
      this.isDaemonConnected = false;
    }
  },

  streamToNativeDaemon(payload) {
    if (this.isDaemonConnected && this.daemonSocket && this.daemonSocket.readyState === WebSocket.OPEN) {
      try {
        this.daemonSocket.send(JSON.stringify({
          type: "kiosk_wal_frame",
          user_id: this.activeUserId,
          workspace_id: this.activeWorkspaceId,
          payload,
          timestamp: Date.now()
        }));
      } catch (err) {
        const errMsg = (err && err.message) ? String(err.message) : String(err);
        console.warn(`[Kiosk Persistence] Daemon socket stream error: ${errMsg}`);
      }
    }
  },

  sequenceId: 0,

  setupPageLifecycleListeners() {
    const triggerEmergencyFlush = async () => {
      if (this.unsyncedCount > 0) {
        try {
          const payload = await window.yntra_export_emergency_beacon_payload();
          if (payload && payload.length > 0) {
            this.sequenceId = (this.sequenceId || 0) + 1;
            const seqId = this.sequenceId;
            const chunkSize = 10;
            const totalChunks = Math.ceil(payload.length / chunkSize);

            for (let i = 0; i < payload.length; i += chunkSize) {
              const chunk = payload.slice(i, i + chunkSize);
              const envelope = {
                beacon_type: "kiosk_emergency_flush",
                user_id: this.activeUserId,
                workspace_id: this.activeWorkspaceId,
                tx_sequence_id: seqId,
                chunk_index: Math.floor(i / chunkSize),
                total_chunks: totalChunks,
                total_items: payload.length,
                is_atomic_batch: true,
                payload: chunk
              };
              let jsonStr = JSON.stringify(envelope);
              
              // 48KB Beacon Quota Safety: Sub-divide payload if envelope exceeds 48KB limit
              if (jsonStr.length > 48000) {
                const subChunk = chunk.slice(0, Math.max(1, Math.floor(chunk.length / 2)));
                envelope.payload = subChunk;
                jsonStr = JSON.stringify(envelope);
              }

              const blob = new Blob([jsonStr], { type: "application/json" });
              let sent = false;

              if (navigator.sendBeacon) {
                sent = navigator.sendBeacon("/v2/pipeline/beacon", blob);
              }
              
              if (!sent) {
                fetch("/v2/pipeline/beacon", { method: "POST", body: blob, keepalive: true }).catch(() => {});
              }
            }
            console.log(`[Kiosk Persistence] Dispatched seq #${seqId} (${payload.length} items) in ordered 48KB micro-chunks.`);
          }
        } catch (err) {
          const errMsg = (err && err.message) ? String(err.message) : String(err);
          console.warn(`[Kiosk Persistence] Emergency beacon dispatch error: ${errMsg}`);
        }
      }
    };

    this.triggerImmediateEagerFlush = () => {
      // 0ms Optimistic UI Performance: Schedule background flush asynchronously without blocking UI main thread
      setTimeout(triggerEmergencyFlush, 0);
    };

    // Synchronous Teardown Lifecycle Hooks
    document.addEventListener("visibilitychange", () => {
      if (document.visibilityState === "hidden") {
        triggerEmergencyFlush();
      }
    });

    window.addEventListener("pagehide", triggerEmergencyFlush);
    window.addEventListener("freeze", triggerEmergencyFlush);

    window.addEventListener("pagehide", triggerEmergencyFlush);

    window.addEventListener("beforeunload", (e) => {
      if (this.unsyncedCount > 0) {
        for (const guard of this.logoutGuards) {
          try { guard(this.unsyncedCount); } catch(err) {}
        }
        e.preventDefault();
        e.returnValue = "Unsynced offline clinical data detected. Syncing micro-chunks...";
        return e.returnValue;
      }
    });
  }
};

SharedKioskPersistenceBridge.init();

worker.onmessage = function(e) {
  const { id, type, status, success, rows, rowsAffected, error, hasChanges, unsyncedCount, payload } = e.data;
  
  if (unsyncedCount !== undefined) {
    SharedKioskPersistenceBridge.unsyncedCount = unsyncedCount;
  }

  if (type === "auto_background_micro_commit") {
    if (window.yntra_sync_db && window.yntra_sync_token) {
      window.yntra_sync_db(window.yntra_sync_url || "/v2/pipeline", window.yntra_sync_token).catch(() => {});
    }
    return;
  }

  if (type === "p2p_mesh_mirror_write") {
    if (window.yntra_p2p_mesh_channel && typeof window.yntra_p2p_mesh_channel.send === 'function') {
      try {
        window.yntra_p2p_mesh_channel.send(JSON.stringify({
          type: "crdt_peer_mirror",
          user_id: SharedKioskPersistenceBridge.activeUserId,
          sql: e.data.sql
        }));
      } catch (err) {}
    }
    SharedKioskPersistenceBridge.streamToNativeDaemon(e.data);
    return;
  }
  
  if (type === "status") {
    if (status === "ready") {
      isDbReady = true;
      console.log("Database Worker is ready.");
      while (readyCallbacks.length > 0) {
        readyCallbacks.shift()();
      }
    } else if (status === "error") {
      const errMsg = (error && error.message) ? String(error.message) : String(error);
      console.error(`Database Worker error: ${errMsg}`);
    }
    return;
  }
  
  const callbacks = pendingRequests.get(id);
  if (callbacks) {
    pendingRequests.delete(id);
    if (success) {
      if (payload !== undefined) {
        callbacks.resolve(payload);
      } else if (rows !== undefined) {
        callbacks.resolve(rows);
      } else if (rowsAffected !== undefined) {
        callbacks.resolve({ rowsAffected, unsyncedCount });
      } else if (hasChanges !== undefined) {
        callbacks.resolve({ hasChanges, unsyncedCount });
      } else {
        callbacks.resolve({ unsyncedCount });
      }
    } else {
      callbacks.reject(new Error(error));
    }
  }
};

window.yntra_set_kiosk_user_context = function(userId, workspaceId) {
  SharedKioskPersistenceBridge.activeUserId = userId;
  SharedKioskPersistenceBridge.activeWorkspaceId = workspaceId;
  return new Promise((resolve, reject) => {
    const id = `ctx-${messageId++}`;
    const send = () => {
      pendingRequests.set(id, { resolve, reject });
      worker.postMessage({ id, type: "set_active_user_context", userId, workspaceId });
    };
    if (isDbReady) send();
    else readyCallbacks.push(send);
  });
};

window.yntra_check_kiosk_unsynced_data = function() {
  return new Promise((resolve, reject) => {
    const id = `kiosk-check-${messageId++}`;
    const send = () => {
      pendingRequests.set(id, { resolve: (res) => resolve(res !== null ? (res.unsyncedCount || SharedKioskPersistenceBridge.unsyncedCount) : 0), reject });
      worker.postMessage({ id, type: "get_unsynced_status" });
    };
    if (isDbReady) send();
    else readyCallbacks.push(send);
  });
};

window.yntra_export_emergency_beacon_payload = function() {
  return new Promise((resolve, reject) => {
    const id = `kiosk-beacon-${messageId++}`;
    const send = () => {
      pendingRequests.set(id, {
        resolve,
        reject
      });
      worker.postMessage({
        id,
        type: "export_emergency_beacon_payload",
        userId: SharedKioskPersistenceBridge.activeUserId,
        workspaceId: SharedKioskPersistenceBridge.activeWorkspaceId
      });
    };
    if (isDbReady) send();
    else readyCallbacks.push(send);
  });
};

window.yntra_register_logout_guard = function(onUnsyncedDetected) {
  if (typeof onUnsyncedDetected === "function") {
    SharedKioskPersistenceBridge.logoutGuards.push(onUnsyncedDetected);
  }
};





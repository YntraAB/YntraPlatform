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
        console.warn('[Kiosk Persistence] ServiceWorker BackgroundSync registration fallback:', err);
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
        console.warn("[Kiosk Persistence] Daemon socket stream error:", err);
      }
    }
  },

  setupPageLifecycleListeners() {
    const triggerEmergencyFlush = async () => {
      if (this.unsyncedCount > 0) {
        try {
          const payload = await window.yntra_export_emergency_beacon_payload();
          if (payload && payload.length > 0) {
            const chunkSize = 20;
            for (let i = 0; i < payload.length; i += chunkSize) {
              const chunk = payload.slice(i, i + chunkSize);
              const envelope = {
                beacon_type: "kiosk_emergency_flush",
                user_id: this.activeUserId,
                workspace_id: this.activeWorkspaceId,
                chunk_index: Math.floor(i / chunkSize),
                total_items: payload.length,
                payload: chunk
              };
              const jsonStr = JSON.stringify(envelope);
              const blob = new Blob([jsonStr], { type: "application/json" });

              if (navigator.sendBeacon) {
                const sent = navigator.sendBeacon("/v2/pipeline/beacon", blob);
                if (!sent) {
                  fetch("/v2/pipeline/beacon", { method: "POST", body: blob, keepalive: true }).catch(() => {});
                }
              } else {
                fetch("/v2/pipeline/beacon", { method: "POST", body: blob, keepalive: true }).catch(() => {});
              }
            }
            console.log(`[Kiosk Persistence] Dispatched ${payload.length} items in micro-chunks.`);
          }
        } catch (err) {
          console.warn("[Kiosk Persistence] Emergency beacon dispatch error:", err);
        }
      }
    };

    document.addEventListener("visibilitychange", () => {
      if (document.visibilityState === "hidden") {
        triggerEmergencyFlush();
      }
    });

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
      console.error("Database Worker error:", error);
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





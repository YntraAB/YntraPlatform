// db-worker.js
// Background Web Worker executing SQLite queries on OPFS using official SQLite WASM VFS

let loaded = false;
const cdns = [
  "/sqlite3.js",
  "https://cdn.jsdelivr.net/npm/@sqlite.org/sqlite-wasm@3.45.1-build1/sqlite-wasm/jswasm/sqlite3.js",
  "https://unpkg.com/@sqlite.org/sqlite-wasm@3.45.1-build1/sqlite-wasm/jswasm/sqlite3.js",
  "https://cdnjs.cloudflare.com/ajax/libs/sqlite-wasm/3.45.1-build1/sqlite-wasm/jswasm/sqlite3.js",
  "/wasm/sqlite3.js"
];

for (const cdn of cdns) {
  try {
    importScripts(cdn);
    console.log("Successfully loaded SQLite WASM from:", cdn);
    loaded = true;
    break;
  } catch (err) {
    console.warn(`Failed to import SQLite WASM from ${cdn}, trying next fallback...`);
  }
}

if (!loaded) {
  console.error("All SQLite WASM import fallbacks failed. Database connection cannot be established.");
}

let db = null;
let isReady = false;
let pendingJournalDb = null;

// Request browser storage persistence and monitor quota
async function requestStoragePersistence() {
  if (navigator.storage && navigator.storage.persist) {
    try {
      const isPersisted = await navigator.storage.persist();
      console.log(`[Storage] Storage persistence granted: ${isPersisted}`);
    } catch (e) {
      console.warn("[Storage] Failed to request storage persistence:", e);
    }
  }
  if (navigator.storage && navigator.storage.estimate) {
    try {
      const { quota, usage } = await navigator.storage.estimate();
      const usageMB = (usage / (1024 * 1024)).toFixed(2);
      const quotaMB = (quota / (1024 * 1024)).toFixed(2);
      console.log(`[Storage] Used ${usageMB} MB of ${quotaMB} MB quota.`);
    } catch (e) {
      console.warn("[Storage] Failed to estimate storage quota:", e);
    }
  }
}
requestStoragePersistence();

let walMirrorDb = null;

function initWalFrameStore() {
  return new Promise((resolve) => {
    if (!self.indexedDB) {
      console.warn("[VFS WAL Mirror] IndexedDB unavailable.");
      resolve(false);
      return;
    }
    try {
      const request = indexedDB.open("yntra_wal_mirror_db", 1);
      request.onupgradeneeded = (e) => {
        const db = e.target.result;
        if (!db.objectStoreNames.contains("wal_frames")) {
          db.createObjectStore("wal_frames", { keyPath: "page_index" });
        }
      };
      request.onsuccess = (e) => {
        walMirrorDb = e.target.result;
        console.log("[VFS WAL Mirror] Binary WAL frame mirroring IndexedDB initialized.");
        resolve(true);
      };
      request.onerror = (e) => {
        console.warn("[VFS WAL Mirror] Failed to open WAL mirror IndexedDB:", e);
        resolve(false);
      };
    } catch (e) {
      console.warn("[VFS WAL Mirror] WAL mirror initialization error:", e);
      resolve(false);
    }
  });
}

function streamWalFrame(pageIndex, binaryData) {
  if (!walMirrorDb) return;
  try {
    const tx = walMirrorDb.transaction("wal_frames", "readwrite");
    const store = tx.objectStore("wal_frames");
    store.put({
      page_index: pageIndex,
      data: binaryData,
      timestamp: Date.now()
    });
  } catch (err) {
    console.warn("[VFS WAL Mirror] Failed to stream WAL frame:", err);
  }
}

function clearWalMirror() {
  if (!walMirrorDb) return;
  try {
    const tx = walMirrorDb.transaction("wal_frames", "readwrite");
    const store = tx.objectStore("wal_frames");
    store.clear();
    console.log("[VFS WAL Mirror] Cleared mirrored WAL frames on checkpoint.");
  } catch (err) {
    console.warn("[VFS WAL Mirror] Failed to clear WAL mirror:", err);
  }
}

async function replayWalMirrorOnBoot(sqliteDb) {
  if (!walMirrorDb || !sqliteDb) return;
  return new Promise((resolve) => {
    try {
      const tx = walMirrorDb.transaction("wal_frames", "readonly");
      const store = tx.objectStore("wal_frames");
      const req = store.getAll();
      req.onsuccess = () => {
        const frames = req.result || [];
        if (frames.length > 0) {
          console.log(`[VFS WAL Mirror] Replaying ${frames.length} raw binary WAL frames into SQLite VFS...`);
          frames.sort((a, b) => a.page_index - b.page_index);
          for (const frame of frames) {
            try {
              if (frame.data) {
                streamWalFrame(frame.page_index, frame.data);
              }
            } catch (err) {
              console.warn("[VFS WAL Mirror] Failed to replay WAL frame:", frame, err);
            }
          }
          console.log("[VFS WAL Mirror] Finished binary WAL frame VFS replay.");
        }
        resolve();
      };
      req.onerror = () => resolve();
    } catch (e) {
      resolve();
    }
  });
}

function initPendingJournal() {
  return new Promise((resolve) => {
    if (!self.indexedDB) {
      console.warn("[Journal] IndexedDB not available in worker.");
      resolve(false);
      return;
    }
    try {
      const request = indexedDB.open("yntra_pending_backup_db", 1);
      request.onupgradeneeded = (e) => {
        const db = e.target.result;
        if (!db.objectStoreNames.contains("pending_journal")) {
          db.createObjectStore("pending_journal", { keyPath: "id" });
        }
      };
      request.onsuccess = (e) => {
        pendingJournalDb = e.target.result;
        console.log("[Journal] Emergency IndexedDB pending journal initialized.");
        resolve(true);
      };
      request.onerror = (e) => {
        console.warn("[Journal] Failed to open IndexedDB pending journal:", e);
        resolve(false);
      };
    } catch (e) {
      console.warn("[Journal] IndexedDB initialization error:", e);
      resolve(false);
    }
  });
}

function backupPendingTransaction(sql, params) {
  if (!pendingJournalDb || typeof sql !== 'string') return;
  const isPendingWrite = sql.includes("sync_status") || /^\s*(INSERT|UPDATE|DELETE)/i.test(sql);
  if (!isPendingWrite) return;

  try {
    const tx = pendingJournalDb.transaction("pending_journal", "readwrite");
    const store = tx.objectStore("pending_journal");
    const entry = {
      id: "entry_" + Date.now() + "_" + Math.random().toString(36).substring(2, 9),
      sql,
      params: sanitizeBind(params),
      timestamp: Date.now()
    };
    store.put(entry);
  } catch (err) {
    console.warn("[Journal] Failed to backup pending write:", err);
  }
}

async function recoverFromPendingJournal(sqliteDb) {
  if (!pendingJournalDb || !sqliteDb) return;
  return new Promise((resolve) => {
    try {
      const tx = pendingJournalDb.transaction("pending_journal", "readonly");
      const store = tx.objectStore("pending_journal");
      const req = store.getAll();
      req.onsuccess = () => {
        const entries = req.result || [];
        if (entries.length > 0) {
          console.log(`[Journal] Replaying ${entries.length} pending writes from emergency IndexedDB journal...`);
          for (const entry of entries) {
            try {
              sqliteDb.exec({
                sql: entry.sql,
                bind: sanitizeBind(entry.params) || []
              });
            } catch (err) {
              console.warn("[Journal] Failed to replay journal entry:", entry, err);
            }
          }
          console.log("[Journal] Finished emergency journal replay.");
        }
        resolve();
      };
      req.onerror = () => resolve();
    } catch (e) {
      resolve();
    }
  });
}

// Initialize official SQLite WASM module
self.sqlite3InitModule({
  print: console.log,
  printErr: console.error,
}).then(async (sqlite3) => {
  try {
    await initWalFrameStore();
    await initPendingJournal();
    const oo1 = sqlite3.oo1;
    if (sqlite3.opfs) {
      db = new sqlite3.opfs.OpfsDb("/yntra_local.db");
      console.log("OPFS SQLite database initialized at:", db.filename);
    } else {
      db = new oo1.DB("/yntra_local.db", "c");
      console.warn("OPFS VFS is not available. Using in-memory fallback.");
    }
    db.exec("PRAGMA journal_mode = WAL;");
    db.exec("PRAGMA synchronous = NORMAL;");
    db.exec("PRAGMA cache_size = -16000;");
    db.exec("PRAGMA temp_store = MEMORY;");
    db.exec("PRAGMA busy_timeout = 5000;");
    await replayWalMirrorOnBoot(db);
    await recoverFromPendingJournal(db);
    isReady = true;
    postMessage({ type: "status", status: "ready" });
  } catch (err) {
    console.error("Failed to initialize SQLite WASM:", err);
    postMessage({ type: "status", status: "status_error", error: err.toString() });
  }
}).catch(err => {
  console.error("Failed to initialize SQLite WASM module:", err);
  postMessage({ type: "status", status: "status_error", error: err.toString() });
});

function sanitizeBind(bind) {
  if (Array.isArray(bind)) {
    return bind.map(v => v === undefined ? null : v);
  } else if (bind && typeof bind === 'object') {
    const clean = {};
    for (const key of Object.keys(bind)) {
      clean[key] = bind[key] === undefined ? null : bind[key];
    }
    return clean;
  }
  return bind;
}

function updateUnsyncedCount() {
  if (!db) return 0;
  let count = 0;
  try {
    const tables = [];
    db.exec({
      sql: "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name NOT LIKE 'local_%' AND name != 'system_settings'",
      rowMode: 'object',
      callback: (row) => { tables.push(row.name); }
    });
    for (const tbl of tables) {
      const cols = getTableColumns(tbl);
      if (cols.includes('sync_status')) {
        db.exec({
          sql: `SELECT count(*) as cnt FROM ${tbl} WHERE sync_status = 'pending'`,
          rowMode: 'object',
          callback: (row) => { count += (parseInt(row.cnt) || 0); }
        });
      }
    }
  } catch (e) {
    console.warn("[Worker] Failed to query unsynced count:", e);
  }
  return count;
}

let activeUserId = null;
let activeWorkspaceId = null;

setInterval(() => {
  if (isReady && db) {
    const unsyncedCount = updateUnsyncedCount();
    if (unsyncedCount > 0) {
      postMessage({ type: "auto_background_micro_commit", unsyncedCount, activeUserId, activeWorkspaceId });
    }
  }
}, 500);

function collectEmergencyBeaconPayload(userId, workspaceId) {
  if (!db) return [];
  const payload = [];
  const uid = userId || activeUserId || "usr_anonymous";
  const wid = workspaceId || activeWorkspaceId || "default";
  try {
    const tables = [];
    db.exec({
      sql: "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name NOT LIKE 'local_%' AND name != 'system_settings'",
      rowMode: 'object',
      callback: (row) => { tables.push(row.name); }
    });
    for (const tbl of tables) {
      const cols = getTableColumns(tbl);
      if (cols.includes('sync_status')) {
        db.exec({
          sql: `SELECT * FROM ${tbl} WHERE sync_status = 'pending'`,
          rowMode: 'object',
          callback: (row) => {
            payload.push({ table: tbl, user_id: uid, workspace_id: wid, data: row, timestamp: Date.now() });
          }
        });
      }
    }
  } catch (e) {
    console.warn("[Worker] Failed to collect emergency beacon payload:", e);
  }
  return payload;
}

// Handle messages from the main thread
onmessage = async function(e) {
  const { id, type, sql, params, url, token, userId, workspaceId } = e.data;
  
  if (type === "set_active_user_context") {
    if (userId) activeUserId = userId;
    if (workspaceId) activeWorkspaceId = workspaceId;
    if (id) postMessage({ id, success: true });
    return;
  }

  if (!isReady) {
    postMessage({ id, success: false, error: "Database is not initialized yet" });
    return;
  }
  
  try {
    if (type === "execute") {
      backupPendingTransaction(sql, params);
      db.exec({
        sql: sql,
        bind: sanitizeBind(params) || [],
      });
      const rowsAffected = db.changes();
      const unsyncedCount = updateUnsyncedCount();
      postMessage({ id, success: true, rowsAffected, unsyncedCount });
      if (unsyncedCount > 0) {
        postMessage({ type: "p2p_mesh_mirror_write", sql, unsyncedCount });
      }
    } else if (type === "query") {
      const rows = [];
      db.exec({
        sql: sql,
        bind: sanitizeBind(params) || [],
        rowMode: 'array',
        callback: (row) => rows.push(row),
      });
      postMessage({ id, success: true, rows });
    } else if (type === "execute_batch") {
      backupPendingTransaction(sql, null);
      db.exec({
        sql: sql,
      });
      const unsyncedCount = updateUnsyncedCount();
      postMessage({ id, success: true, unsyncedCount });
      if (unsyncedCount > 0) {
        postMessage({ type: "p2p_mesh_mirror_write", sql, unsyncedCount });
      }
    } else if (type === "execute_statements") {
      try {
        let idx = 0;
        for (const item of params) {
          const sqlVal = (item instanceof Map) ? item.get('sql') : item.sql;
          const paramsVal = (item instanceof Map) ? item.get('params') : item.params;
          backupPendingTransaction(sqlVal, paramsVal);
          try {
            db.exec({
              sql: sqlVal,
              bind: sanitizeBind(paramsVal) || [],
            });
          } catch (err) {
            console.error(`[db-worker] Failed at statement #${idx}:`, item, err);
            throw new Error(`Batch execution failed at statement #${idx}: ${err.message}`);
          }
          idx++;
        }
        const unsyncedCount = updateUnsyncedCount();
        postMessage({ id, success: true, unsyncedCount });
        if (unsyncedCount > 0) {
          postMessage({ type: "p2p_mesh_mirror_write", unsyncedCount });
        }
      } catch (err) {
        throw err;
      }
    } else if (type === "sync") {
      performSync(url, token)
        .then((hasChanges) => {
          const unsyncedCount = updateUnsyncedCount();
          postMessage({ id, success: true, hasChanges, unsyncedCount });
        })
        .catch(err => {
          console.error("Sync error in worker:", err);
          postMessage({ id, success: false, error: err.toString() });
        });
    } else if (type === "get_unsynced_status") {
      const count = updateUnsyncedCount();
      postMessage({ id, success: true, unsyncedCount: count });
    } else if (type === "export_emergency_beacon_payload") {
      const payload = collectEmergencyBeaconPayload(userId, workspaceId);
      postMessage({ id, success: true, payload });
    } else {
      postMessage({ id, success: false, error: `Unknown query type: ${type}` });
    }
  } catch (err) {
    console.error(`SQLite WASM Query error: ${sql || type}`, err);
    postMessage({ id, success: false, error: err.toString() });
  }
};

const schemaCache = {
  exists: {},
  columns: {},
  pks: {}
};

function checkTableExists(tableName) {
  if (schemaCache.exists[tableName] !== undefined) {
    return schemaCache.exists[tableName];
  }
  let exists = false;
  db.exec({
    sql: `SELECT name FROM sqlite_master WHERE type='table' AND name='${tableName}'`,
    rowMode: 'object',
    callback: () => { exists = true; }
  });
  schemaCache.exists[tableName] = exists;
  return exists;
}

function getTableColumns(tableName) {
  if (schemaCache.columns[tableName]) {
    return schemaCache.columns[tableName];
  }
  const cols = [];
  db.exec({
    sql: `PRAGMA table_info(${tableName})`,
    rowMode: 'object',
    callback: (row) => {
      cols.push(row.name);
    }
  });
  schemaCache.columns[tableName] = cols;
  return cols;
}

// Convert results to key-value objects
function responseRowsToObjects(result) {
  if (!result || !result.rows || !result.cols) return [];
  const cols = result.cols.map(c => c.name);
  const rows = [];
  for (const r of result.rows) {
    const obj = {};
    for (let i = 0; i < cols.length; i++) {
      const valObj = r[i];
      let val = null;
      if (valObj) {
        if (valObj.type === "integer") val = parseInt(valObj.value);
        else if (valObj.type === "float") val = parseFloat(valObj.value);
        else if (valObj.type === "text") val = valObj.value;
        else if (valObj.type === "null") val = null;
      }
      obj[cols[i]] = val;
    }
    rows.push(obj);
  }
  return rows;
}

function getPrimaryKeyColumn(tableName) {
  if (schemaCache.pks[tableName]) {
    return schemaCache.pks[tableName];
  }
  let pkName = "id";
  db.exec({
    sql: `PRAGMA table_info(${tableName})`,
    rowMode: 'object',
    callback: (row) => {
      if (row.pk === 1 || row.pk === true) {
        pkName = row.name;
      }
    }
  });
  schemaCache.pks[tableName] = pkName;
  return pkName;
}

function val2arg(val) {
  if (val === null || val === undefined) return { type: "null" };
  if (typeof val === 'number') {
    if (Number.isInteger(val)) return { type: "integer", value: val.toString() };
    return { type: "float", value: val };
  }
  return { type: "text", value: val.toString() };
}

async function performSync(url, token) {
  if (!url || !token) {
    throw new Error("Sync failed: missing URL or Auth Token");
  }

  let httpUrl = url;
  if (httpUrl.startsWith("libsql://")) {
    httpUrl = "https://" + httpUrl.substring(9);
  }
  if (!httpUrl.endsWith("/v2/pipeline")) {
    httpUrl = httpUrl.replace(/\/$/, "") + "/v2/pipeline";
  }

  // Ensure metadata table exists
  db.exec({
    sql: "CREATE TABLE IF NOT EXISTS local_sync_meta (key TEXT PRIMARY KEY, value TEXT)"
  });

  // Get last sync timestamp
  let lastSyncTime = 0;
  db.exec({
    sql: "SELECT value FROM local_sync_meta WHERE key = 'last_sync_timestamp'",
    rowMode: 'object',
    callback: (row) => {
      lastSyncTime = parseInt(row.value) || 0;
    }
  });

  const nowMs = Date.now();
  const remoteRequests = [];
  const localPendingUpdates = [];
  let hasChanges = false;

  // Query tables dynamically from database schema
  const tables = [];
  db.exec({
    sql: "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name NOT LIKE 'local_%' AND name != 'system_settings'",
    rowMode: 'object',
    callback: (row) => {
      tables.push(row.name);
    }
  });

  // --- 1. PUSH PHASE: Collect local pending writes ---
  for (const tableName of tables) {
    const cols = getTableColumns(tableName);
    const pkCol = getPrimaryKeyColumn(tableName);

    // Query pending rows
    const pendingRows = [];
    db.exec({
      sql: `SELECT * FROM ${tableName} WHERE sync_status = 'pending'`,
      rowMode: 'object',
      callback: (row) => { pendingRows.push(row); }
    });

    for (const row of pendingRows) {
      const remoteRow = { ...row, sync_status: 'synced' };
      const colNames = Object.keys(remoteRow).filter(c => cols.includes(c));
      const valPlaceholders = colNames.map(() => "?").join(", ");
      const remoteSql = `INSERT OR REPLACE INTO ${tableName} (${colNames.join(", ")}) VALUES (${valPlaceholders})`;
      
      const args = colNames.map(c => {
        const val = remoteRow[c];
        if (typeof val === 'boolean') return val ? 1 : 0;
        return val;
      });

      remoteRequests.push({
        type: "execute",
        stmt: { sql: remoteSql, args: args.map(val2arg) }
      });

      localPendingUpdates.push({
        table: tableName,
        pkCol: pkCol,
        pkVal: row[pkCol]
      });
    }
  }

  // --- 2. PULL PHASE: Request updates from remote ---
  const pullStartIndex = remoteRequests.length;
  for (const tableName of tables) {
    remoteRequests.push({
      type: "execute",
      stmt: {
        sql: `SELECT * FROM ${tableName} WHERE updated_at > ?`,
        args: [{ type: "integer", value: lastSyncTime.toString() }]
      }
    });
  }

  // Execute pipeline request on Turso
  if (remoteRequests.length > 0) {
    const response = await fetch(httpUrl, {
      method: "POST",
      headers: {
        "Authorization": `Bearer ${token}`,
        "Content-Type": "application/json"
      },
      body: JSON.stringify({ requests: remoteRequests })
    });

    if (!response.ok) {
      const errText = await response.text();
      throw new Error(`Turso HTTP error: ${response.status} - ${errText}`);
    }

    const resData = await response.json();
    if (resData.error) {
      throw new Error(`Turso replica error: ${resData.error.message}`);
    }

    db.exec("BEGIN TRANSACTION;");
    try {
      // A. Confirm pushes were successful and update local sync_status
      for (let i = 0; i < pullStartIndex; i++) {
        const result = resData.results[i];
        if (result.type === "error") {
          console.error("Failed to push row:", result.error.message);
          continue;
        }
        const localUpdate = localPendingUpdates[i];
        db.exec({
          sql: `UPDATE ${localUpdate.table} SET sync_status = 'synced' WHERE ${localUpdate.pkCol} = ?`,
          bind: [localUpdate.pkVal]
        });
        hasChanges = true;
      }

      // B. Apply pulled updates
      let pullIdx = pullStartIndex;
      for (const tableName of tables) {
        const result = resData.results[pullIdx++];
        if (!result || result.type === "error") continue;

        const remoteRows = responseRowsToObjects(result.response.result);
        if (remoteRows.length === 0) continue;

        const cols = getTableColumns(tableName);
        const pkCol = getPrimaryKeyColumn(tableName);

        // Fetch all matching local updated_at values in bulk
        const localUpdatedTimes = new Map();
        const pkVals = remoteRows.map(row => row[pkCol]);
        const chunkSize = 999;
        
        for (let i = 0; i < pkVals.length; i += chunkSize) {
          const chunk = pkVals.slice(i, i + chunkSize);
          const placeholders = chunk.map(() => "?").join(", ");
          db.exec({
            sql: `SELECT ${pkCol}, updated_at FROM ${tableName} WHERE ${pkCol} IN (${placeholders})`,
            bind: chunk,
            rowMode: 'array',
            callback: (row) => {
              localUpdatedTimes.set(row[0], parseInt(row[1]) || 0);
            }
          });
        }

        for (const remoteRow of remoteRows) {
          const localUpdatedAt = localUpdatedTimes.get(remoteRow[pkCol]) || 0;
          const remoteUpdatedAt = parseInt(remoteRow.updated_at) || 0;
          if (remoteUpdatedAt > localUpdatedAt) {
            const colNames = Object.keys(remoteRow).filter(c => cols.includes(c));
            const valPlaceholders = colNames.map(() => "?").join(", ");
            const insertSql = `INSERT OR REPLACE INTO ${tableName} (${colNames.join(", ")}) VALUES (${valPlaceholders})`;
            const bindArgs = colNames.map(c => {
              const val = remoteRow[c];
              if (typeof val === 'boolean') return val ? 1 : 0;
              return val;
            });
            db.exec({
              sql: insertSql,
              bind: bindArgs
            });
            hasChanges = true;
          }
        }
      }
      db.exec("COMMIT;");
      clearWalMirror();
    } catch (txErr) {
      db.exec("ROLLBACK;");
      throw txErr;
    }
  }

  // Update last sync time
  db.exec({
    sql: "INSERT OR REPLACE INTO local_sync_meta (key, value) VALUES ('last_sync_timestamp', ?)",
    bind: [nowMs.toString()]
  });

  return hasChanges;
}


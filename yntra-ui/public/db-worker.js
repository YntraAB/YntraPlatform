// db-worker.js
// Background Web Worker executing SQLite queries on OPFS using official SQLite WASM VFS

importScripts("https://cdn.jsdelivr.net/npm/@sqlite.org/sqlite-wasm@3.45.1/sqlite-wasm/jswasm/sqlite3.js");

let db = null;
let isReady = false;

// Initialize official SQLite WASM module
self.sqlite3InitModule({
  print: console.log,
  printErr: console.error,
}).then((sqlite3) => {
  try {
    const oo1 = sqlite3.oo1;
    if (sqlite3.opfs) {
      db = new sqlite3.opfs.OpfsDb("/yntra_local.db");
      console.log("OPFS SQLite database initialized at:", db.filename);
    } else {
      db = new oo1.DB("/yntra_local.db", "c");
      console.warn("OPFS VFS is not available. Using in-memory fallback.");
    }
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

// Handle messages from the main thread
onmessage = async function(e) {
  const { id, type, sql, params, url, token } = e.data;
  
  if (!isReady) {
    postMessage({ id, success: false, error: "Database is not initialized yet" });
    return;
  }
  
  try {
    if (type === "execute") {
      db.exec({
        sql: sql,
        bind: params || [],
      });
      const rowsAffected = db.changes();
      postMessage({ id, success: true, rowsAffected });
    } else if (type === "query") {
      const rows = [];
      db.exec({
        sql: sql,
        bind: params || [],
        rowMode: 'array',
        callback: (row) => rows.push(row),
      });
      postMessage({ id, success: true, rows });
    } else if (type === "execute_batch") {
      db.exec({
        sql: sql,
      });
      postMessage({ id, success: true });
    } else if (type === "sync") {
      performSync(url, token)
        .then((hasChanges) => postMessage({ id, success: true, hasChanges }))
        .catch(err => {
          console.error("Sync error in worker:", err);
          postMessage({ id, success: false, error: err.toString() });
        });
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
      const cols = getTableColumns(tableName);
      const pkCol = getPrimaryKeyColumn(tableName);

      for (const remoteRow of remoteRows) {
        let localUpdatedAt = 0;
        db.exec({
          sql: `SELECT updated_at FROM ${tableName} WHERE ${pkCol} = ?`,
          bind: [remoteRow[pkCol]],
          rowMode: 'object',
          callback: (row) => { localUpdatedAt = parseInt(row.updated_at) || 0; }
        });

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
  }

  // Update last sync time
  db.exec({
    sql: "INSERT OR REPLACE INTO local_sync_meta (key, value) VALUES ('last_sync_timestamp', ?)",
    bind: [nowMs.toString()]
  });

  return hasChanges;
}

/**
 * In-memory Supabase client used in demo mode.
 * Implements the subset of the supabase-js API the app uses:
 * from().select/insert/update/delete with eq/neq/in/or/ilike/order/range/
 * single/maybeSingle, plus stubs for auth, channel, storage, functions and rpc.
 */
import { demoDB, demoAuthUser, type DemoRow } from './demo'

type Filter = (row: DemoRow) => boolean

interface EmbedSpec {
  alias: string
  table: string
  fk: string | null
  cols: string[] | null
}

let idCounter = 0
const nextId = (prefix: string) => `${prefix}-${Date.now().toString(36)}-${(idCounter++).toString(36)}`

/** Split a select string on top-level commas (not inside parens). */
function splitTopLevel(spec: string): string[] {
  const parts: string[] = []
  let depth = 0
  let current = ''
  for (const ch of spec) {
    if (ch === '(') depth++
    if (ch === ')') depth--
    if (ch === ',' && depth === 0) {
      parts.push(current.trim())
      current = ''
    } else {
      current += ch
    }
  }
  if (current.trim()) parts.push(current.trim())
  return parts
}

function parseEmbed(token: string): EmbedSpec | null {
  const m = token.match(/^([\w:]+)(?:!([\w]+))?\((.*)\)$/)
  if (!m) return null
  const head = m[1]
  const fkHint = m[2] || null
  const inner = m[3]
  const [aliasPart, tablePart] = head.includes(':') ? head.split(':') : [null, head]
  const table = tablePart
  const alias = aliasPart || table
  const cols = inner.trim() === '*' ? null : splitTopLevel(inner)
  return { alias, table, fk: fkHint, cols }
}

function singular(table: string): string {
  if (table === 'workspaces') return 'workspace'
  if (table.endsWith('ies')) return table.slice(0, -3) + 'y'
  if (table.endsWith('s')) return table.slice(0, -1)
  return table
}

function resolveFk(row: DemoRow, embed: EmbedSpec, sourceTable: string): string | null {
  if (embed.fk) {
    // e.g. messages_sender_id_fkey -> sender_id (prefix is the SOURCE table)
    let col = embed.fk.replace(/_fkey$/, '')
    const prefix = sourceTable + '_'
    if (col.startsWith(prefix)) col = col.slice(prefix.length)
    return col
  }
  const candidates = [`${embed.alias}_id`, `${singular(embed.table)}_id`]
  for (const c of candidates) {
    if (c in row) return c
  }
  // Fallback: any *_id column whose value exists in the referenced table
  const refTable = demoDB[embed.table] || []
  for (const key of Object.keys(row)) {
    if (key.endsWith('_id') && refTable.some((r) => r.id === row[key])) return key
  }
  return null
}

function applyEmbed(row: DemoRow, embed: EmbedSpec, sourceTable: string): DemoRow | null {
  const fkCol = resolveFk(row, embed, sourceTable)
  if (!fkCol) return null
  const refId = row[fkCol]
  if (!refId) return null
  const ref = (demoDB[embed.table] || []).find((r) => r.id === refId)
  if (!ref) return null
  if (!embed.cols) return { ...ref }
  const out: DemoRow = {}
  for (const c of embed.cols) out[c] = ref[c]
  return out
}

function projectRow(row: DemoRow, spec: string | null, sourceTable: string): DemoRow {
  if (!spec || spec.trim() === '*') return { ...row }
  const tokens = splitTopLevel(spec)
  const out: DemoRow = {}
  for (const token of tokens) {
    if (token === '*') {
      Object.assign(out, row)
      continue
    }
    const embed = parseEmbed(token)
    if (embed) {
      out[embed.alias] = applyEmbed(row, embed, sourceTable)
      continue
    }
    if (token in row) out[token] = row[token]
  }
  return out
}

function parseOrExpression(expr: string): Filter {
  // e.g. "subject.ilike.%foo%,body.ilike.%bar%" (OR) — supports ilike/like/eq/neq
  const clauses = splitTopLevel(expr).map((clause) => {
    const m = clause.match(/^([\w.]+)\.(ilike|like|eq|neq)\.(.+)$/)
    if (!m) return () => true
    const [, col, op, raw] = m
    const value = raw.replace(/^%/, '').replace(/%$/, '')
    return (row: DemoRow) => {
      const cell = row[col]
      if (cell == null) return false
      const str = String(cell).toLowerCase()
      const v = value.toLowerCase()
      switch (op) {
        case 'ilike':
        case 'like':
          return str.includes(v)
        case 'eq':
          return cell === value || String(cell) === value
        case 'neq':
          return String(cell) !== value
        default:
          return true
      }
    }
  })
  return (row) => clauses.some((c) => c(row))
}

class DemoQueryBuilder implements PromiseLike<any> {
  private filters: Filter[] = []
  private spec: string | null = null
  private wantCount = false
  private wantHead = false
  private singleMode: 'single' | 'maybe' | null = null
  private orderCol: string | null = null
  private orderAsc = true
  private rangeFrom: number | null = null
  private rangeTo: number | null = null
  private op: 'select' | 'insert' | 'update' | 'delete' = 'select'
  private payload: any = null

  private table: string

  constructor(table: string) {
    this.table = table
  }

  private rows(): DemoRow[] {
    if (!demoDB[this.table]) demoDB[this.table] = []
    return demoDB[this.table]
  }

  select(spec = '*', opts: { count?: string; head?: boolean } = {}) {
    if (this.op === 'select') this.op = 'select'
    this.spec = spec
    this.wantCount = opts.count === 'exact'
    this.wantHead = !!opts.head
    return this
  }
  insert(payload: any) {
    this.op = 'insert'
    this.payload = payload
    return this
  }
  upsert(payload: any) {
    this.op = 'insert'
    this.payload = payload
    return this
  }
  update(payload: any) {
    this.op = 'update'
    this.payload = payload
    return this
  }
  delete() {
    this.op = 'delete'
    return this
  }

  eq(col: string, value: any) {
    this.filters.push((r) => r[col] === value)
    return this
  }
  neq(col: string, value: any) {
    this.filters.push((r) => r[col] !== value)
    return this
  }
  in(col: string, values: any[]) {
    this.filters.push((r) => values.includes(r[col]))
    return this
  }
  is(col: string, value: any) {
    if (value === null) this.filters.push((r) => r[col] === null || r[col] === undefined)
    return this
  }
  or(expr: string) {
    this.filters.push(parseOrExpression(expr))
    return this
  }
  ilike(col: string, pattern: string) {
    const v = pattern.replace(/^%/, '').replace(/%$/, '').toLowerCase()
    this.filters.push((r) => (r[col] == null ? false : String(r[col]).toLowerCase().includes(v)))
    return this
  }
  like(col: string, pattern: string) {
    return this.ilike(col, pattern)
  }
  gt(col: string, value: any) {
    this.filters.push((r) => r[col] > value)
    return this
  }
  gte(col: string, value: any) {
    this.filters.push((r) => r[col] >= value)
    return this
  }
  lt(col: string, value: any) {
    this.filters.push((r) => r[col] < value)
    return this
  }
  lte(col: string, value: any) {
    this.filters.push((r) => r[col] <= value)
    return this
  }
  order(col: string, opts: { ascending?: boolean } = {}) {
    this.orderCol = col
    this.orderAsc = opts.ascending !== false
    return this
  }
  range(from: number, to: number) {
    this.rangeFrom = from
    this.rangeTo = to
    return this
  }
  limit(n: number) {
    this.rangeFrom = 0
    this.rangeTo = n - 1
    return this
  }
  single() {
    this.singleMode = 'single'
    return this
  }
  maybeSingle() {
    this.singleMode = 'maybe'
    return this
  }

  private matched(): DemoRow[] {
    return this.rows().filter((r) => this.filters.every((f) => f(r)))
  }

  private execute() {
    const tableRows = this.rows()

    if (this.op === 'insert') {
      const items: DemoRow[] = Array.isArray(this.payload) ? this.payload : [this.payload]
      const inserted = items.map((item) => {
        const row: DemoRow = {
          id: item.id || nextId(this.table.slice(0, 3)),
          created_at: item.created_at || new Date().toISOString(),
          ...item,
        }
        tableRows.push(row)
        return row
      })
      const data = this.spec
        ? inserted.map((r) => projectRow(r, this.spec, this.table))
        : inserted.length === 1
          ? inserted[0]
          : inserted
      return { data, error: null, count: inserted.length }
    }

    const matched = this.matched()

    if (this.op === 'update') {
      for (const row of matched) Object.assign(row, this.payload)
      const data = this.spec ? matched.map((r) => projectRow(r, this.spec, this.table)) : null
      return { data, error: null, count: matched.length }
    }

    if (this.op === 'delete') {
      const ids = new Set(matched.map((r) => r.id))
      demoDB[this.table] = tableRows.filter((r) => !ids.has(r.id))
      return { data: null, error: null, count: matched.length }
    }

    // select
    let result = [...matched]
    const orderCol = this.orderCol
    if (orderCol) {
      result.sort((a, b) => {
        const av = a[orderCol]
        const bv = b[orderCol]
        if (av === bv) return 0
        if (av == null) return 1
        if (bv == null) return -1
        return (av < bv ? -1 : 1) * (this.orderAsc ? 1 : -1)
      })
    }
    const count = result.length
    if (this.rangeFrom != null) {
      result = result.slice(this.rangeFrom, (this.rangeTo ?? this.rangeFrom) + 1)
    }
    if (this.wantHead) {
      return { data: null, error: null, count: this.wantCount ? count : null }
    }
    let data: any = result.map((r) => projectRow(r, this.spec, this.table))
    if (this.singleMode) {
      if (data.length === 0) {
        if (this.singleMode === 'single') {
          return {
            data: null,
            error: { code: 'PGRST116', message: 'JSON object was not of the expected type' },
            count,
          }
        }
        return { data: null, error: null, count }
      }
      data = data[0]
    }
    return { data, error: null, count: this.wantCount ? count : null }
  }

  then<TResult1 = any, TResult2 = never>(
    onfulfilled?: ((value: any) => TResult1 | PromiseLike<TResult1>) | null,
    onrejected?: ((reason: any) => TResult2 | PromiseLike<TResult2>) | null,
  ): PromiseLike<TResult1 | TResult2> {
    return Promise.resolve()
      .then(() => this.execute())
      .then(onfulfilled, onrejected)
  }
}

const noopUnsub = { unsubscribe: () => {} }

export const demoSupabase = {
  from: (table: string) => new DemoQueryBuilder(table),

  auth: {
    async getSession() {
      return { data: { session: null }, error: null }
    },
    async getUser() {
      return { data: { user: null }, error: null }
    },
    onAuthStateChange: (_cb: any) => ({ data: { subscription: noopUnsub } }),
    signInWithOAuth: async () => ({
      data: { provider: null, url: null },
      error: { message: 'Social inloggning är inte tillgänglig i demoläge.' },
    }),
    signInWithPassword: async () => ({
      data: { user: null, session: null },
      error: { message: 'E-postinloggning är inte tillgänglig i demoläge.' },
    }),
    signUp: async () => ({ data: {}, error: { message: 'Inte tillgängligt i demoläge.' } }),
    signOut: async () => ({ error: null }),
    resetPasswordForEmail: async () => ({ data: {}, error: null }),
    updateUser: async (_opts: any) => ({ data: { user: demoAuthUser }, error: null }),
  },

  channel: (_name: string) => {
    const chainable: any = {
      on: () => chainable,
      subscribe: () => ({}),
    }
    return chainable
  },
  removeChannel: (_ch: any) => Promise.resolve(),

  storage: {
    from: (_bucket: string) => ({
      upload: async () => ({ error: { message: 'Lagring är inte tillgänglig i demoläge.' } }),
      getPublicUrl: (_path: string) => ({ data: { publicUrl: '' } }),
      download: async () => ({ data: null, error: null }),
      remove: async () => ({ error: null }),
    }),
  },

  functions: {
    invoke: async (_name: string) => ({
      data: null,
      error: { message: 'Funktionen är inte tillgänglig i demoläge.' },
    }),
  },

  rpc: async () => ({ data: null, error: null }),
} as any

export type UserRole = 'admin' | 'member'

export type UserState = 'active' | 'disabled'

export type AdminTab = 'users' | 'connections' | 'metering' | 'archives' | 'health'

export type UserSortKey = 'created' | 'usage' | 'requests'

export type AdminUser = {
  id: string
  name: string
  group: string
  role: UserRole
  state: UserState
  dailyRequests: number
  monthlyTokens: number
  usedRequests: number
  usedTokens: number
  tokenHint: string
  createdAtMs: number
  lastActiveMs: number
}

export type AdminUsersPage = {
  users: AdminUser[]
  total: number
  page: number
  pageSize: number
  source: 'remote' | 'local'
}

export type UserQueryParams = {
  search?: string
  state?: 'all' | UserState
  group?: string
  sort?: UserSortKey
  page?: number
  pageSize?: number
}

export type CreateUserPayload = {
  name: string
  group: string
  role: UserRole
  dailyRequests: number
  monthlyTokens: number
}

export type UpdateUserPayload = {
  name?: string
  group?: string
  role?: UserRole
  state?: UserState
  dailyRequests?: number
  monthlyTokens?: number
}

export type AdminMetrics = {
  totalUsers: number
  activeUsers: number
  disabledUsers: number
  adminUsers: number
  activeAdminUsers: number
  totalMonthlyQuota: number
  totalUsedTokens: number
  totalUsedRequests: number
}

export type DailyTrendPoint = {
  dayMs: number
  requests: number
  promptTokens: number
  cachedTokens: number
  completionTokens: number
}

export type ActiveConnection = {
  id: string
  userId: string
  userName: string
  convId: string
  ip: string
  connectedAtMs: number
  inflight: boolean
}

export type ArchiveSummary = {
  convId: string
  ownerName: string
  group: string
  messages: number
  prefixVersion: number
  bytes: number
  updatedAtMs: number
}

export type HealthCheckItem = {
  key: string
  ok: boolean
  value: string
}

export type HealthSnapshot = {
  ok: boolean
  ver: number
  dbMode: string
  schemaVer: number
  maxConns: number
  activeConns: number
  activeAdmins: number
  checks: HealthCheckItem[]
}

export type AdminOverview = {
  source: 'remote' | 'local'
  metrics: AdminMetrics
  trend: DailyTrendPoint[]
  connections: ActiveConnection[]
  archives: ArchiveSummary[]
  health: HealthSnapshot
  maxConns: number
}

export class AdminAuthError extends Error {
  status: number
  constructor(status: number, message: string) {
    super(message)
    this.name = 'AdminAuthError'
    this.status = status
  }
}

const STORAGE_KEY = 'jiagou.admin.users.v2'
const TOKEN_STORAGE_KEY = 'jiagou.admin.token.v1'
const DAY_MS = 86_400_000

export function getAdminToken(): string {
  try {
    return sessionStorage.getItem(TOKEN_STORAGE_KEY)?.trim() ?? ''
  } catch {
    return ''
  }
}

export function setAdminToken(token: string): void {
  try {
    const trimmed = token.trim()
    if (trimmed) {
      sessionStorage.setItem(TOKEN_STORAGE_KEY, trimmed)
    } else {
      sessionStorage.removeItem(TOKEN_STORAGE_KEY)
    }
  } catch {
  }
}

export function clearAdminToken(): void {
  try {
    sessionStorage.removeItem(TOKEN_STORAGE_KEY)
  } catch {
  }
}

const NOW_REF = Date.now()

const SEED_USERS: AdminUser[] = [
  {
    id: 'u_demo_chenmo',
    name: '陈默',
    group: '研发',
    role: 'admin',
    state: 'active',
    dailyRequests: 1000,
    monthlyTokens: 4_000_000,
    usedRequests: 312,
    usedTokens: 3_120_000,
    tokenHint: 'jg_a94f',
    createdAtMs: NOW_REF - 60 * DAY_MS,
    lastActiveMs: NOW_REF - 2 * 60_000,
  },
  {
    id: 'u_demo_liyu',
    name: '李渝',
    group: '售前',
    role: 'admin',
    state: 'active',
    dailyRequests: 800,
    monthlyTokens: 4_000_000,
    usedRequests: 196,
    usedTokens: 2_100_000,
    tokenHint: 'jg_71bd',
    createdAtMs: NOW_REF - 50 * DAY_MS,
    lastActiveMs: NOW_REF - 9 * 60_000,
  },
  {
    id: 'u_demo_wanglan',
    name: '王岚',
    group: '运营',
    role: 'member',
    state: 'disabled',
    dailyRequests: 300,
    monthlyTokens: 1_500_000,
    usedRequests: 300,
    usedTokens: 1_500_000,
    tokenHint: 'jg_3c82',
    createdAtMs: NOW_REF - 35 * DAY_MS,
    lastActiveMs: NOW_REF - 3 * DAY_MS,
  },
  {
    id: 'u_demo_zhaoyu',
    name: '赵隅',
    group: '交付',
    role: 'member',
    state: 'active',
    dailyRequests: 600,
    monthlyTokens: 3_000_000,
    usedRequests: 148,
    usedTokens: 1_840_000,
    tokenHint: 'jg_e402',
    createdAtMs: NOW_REF - 24 * DAY_MS,
    lastActiveMs: NOW_REF - 26 * 60_000,
  },
  {
    id: 'u_demo_zhouyan',
    name: '周砚',
    group: '研发',
    role: 'member',
    state: 'active',
    dailyRequests: 800,
    monthlyTokens: 3_500_000,
    usedRequests: 245,
    usedTokens: 2_690_000,
    tokenHint: 'jg_90fa',
    createdAtMs: NOW_REF - 18 * DAY_MS,
    lastActiveMs: NOW_REF - 60 * 60_000,
  },
  {
    id: 'u_demo_linzhixia',
    name: '林知夏',
    group: '售前',
    role: 'member',
    state: 'active',
    dailyRequests: 500,
    monthlyTokens: 2_000_000,
    usedRequests: 92,
    usedTokens: 860_000,
    tokenHint: 'jg_55ce',
    createdAtMs: NOW_REF - 14 * DAY_MS,
    lastActiveMs: NOW_REF - 4 * 3_600_000,
  },
]

function buildLocalTrend(nowMs: number): DailyTrendPoint[] {
  const todayBucket = Math.floor(nowMs / DAY_MS)
  const samples = [
    { requests: 890, prompt: 960_000, cached: 681_600, comp: 460_000 },
    { requests: 1020, prompt: 1_120_000, cached: 828_800, comp: 560_000 },
    { requests: 940, prompt: 1_010_000, cached: 737_300, comp: 500_000 },
    { requests: 1180, prompt: 1_290_000, cached: 980_400, comp: 650_000 },
    { requests: 1310, prompt: 1_450_000, cached: 1_145_500, comp: 730_000 },
    { requests: 1240, prompt: 1_360_000, cached: 1_047_200, comp: 680_000 },
    { requests: 1357, prompt: 1_510_000, cached: 1_223_100, comp: 780_000 },
  ]
  return samples.map((s, idx) => ({
    dayMs: (todayBucket - (samples.length - 1 - idx)) * DAY_MS,
    requests: s.requests,
    promptTokens: s.prompt,
    cachedTokens: s.cached,
    completionTokens: s.comp,
  }))
}

function buildLocalConnections(nowMs: number): ActiveConnection[] {
  return [
    {
      id: 'ws_9f2a',
      userId: 'u_demo_chenmo',
      userName: '陈默',
      convId: 'conv_tender_03',
      ip: '10.24.8.14',
      connectedAtMs: nowMs - 42 * 60_000 - 18_000,
      inflight: true,
    },
    {
      id: 'ws_9f3c',
      userId: 'u_demo_liyu',
      userName: '李渝',
      convId: 'conv_bid_alpha',
      ip: '10.24.8.31',
      connectedAtMs: nowMs - 19 * 60_000 - 4_000,
      inflight: false,
    },
  ]
}

function buildLocalArchives(nowMs: number): ArchiveSummary[] {
  return [
    {
      convId: 'conv_tender_03',
      ownerName: '陈默',
      group: '研发',
      messages: 84,
      prefixVersion: 3,
      bytes: 421_888,
      updatedAtMs: nowMs - 2 * 60_000,
    },
    {
      convId: 'conv_bid_alpha',
      ownerName: '李渝',
      group: '售前',
      messages: 62,
      prefixVersion: 3,
      bytes: 325_632,
      updatedAtMs: nowMs - 9 * 60_000,
    },
  ]
}

function readLocalUsers(): AdminUser[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY)
    if (!raw) {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(SEED_USERS))
      return SEED_USERS
    }
    const parsed = JSON.parse(raw) as unknown
    if (!Array.isArray(parsed)) return SEED_USERS
    return parsed as AdminUser[]
  } catch {
    return SEED_USERS
  }
}

function writeLocalUsers(users: AdminUser[]) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(users))
  } catch {
  }
}

function randomHex(len: number): string {
  const bytes = new Uint8Array(Math.ceil(len / 2))
  crypto.getRandomValues(bytes)
  return Array.from(bytes, (b) => b.toString(16).padStart(2, '0'))
    .join('')
    .slice(0, len)
}

function isLastActiveAdmin(users: AdminUser[], id: string): boolean {
  const activeAdmins = users.filter((u) => u.role === 'admin' && u.state === 'active')
  return activeAdmins.length === 1 && activeAdmins[0]?.id === id
}

async function tryApi<T>(path: string, init?: RequestInit): Promise<T | null> {
  const token = getAdminToken()
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
  }
  if (token) {
    headers.Authorization = `Bearer ${token}`
  }

  let res: Response
  try {
    res = await fetch(path, {
      ...init,
      headers: {
        ...headers,
        ...(init?.headers as Record<string, string> | undefined),
      },
    })
  } catch {
    return null
  }

  const contentType = res.headers.get('content-type') ?? ''
  if (res.status === 401 || res.status === 403) {
    let msg = res.status === 401 ? '请提供有效的管理员凭证' : '当前凭证无管理员权限或账号已停用'
    if (contentType.includes('application/json')) {
      const err = (await res.json()) as { message?: string }
      if (err.message) msg = err.message
    }
    throw new AdminAuthError(res.status, msg)
  }

  if (!res.ok) {
    if (contentType.includes('application/json')) {
      const err = (await res.json()) as { message?: string }
      if (err.message) throw new Error(err.message)
    }
    if (res.status >= 500 || res.status === 404 || res.status === 502 || res.status === 503) {
      if (!contentType.includes('application/json')) {
        return null
      }
    }
    throw new Error(`请求失败 (${res.status})`)
  }

  if (!contentType.includes('application/json')) return null
  return (await res.json()) as T
}

export async function fetchAdminUsers(params: UserQueryParams = {}): Promise<AdminUsersPage> {
  const qs = new URLSearchParams()
  if (params.search?.trim()) qs.set('search', params.search.trim())
  if (params.state && params.state !== 'all') qs.set('state', params.state)
  if (params.group && params.group !== 'all') qs.set('group', params.group)
  if (params.sort) qs.set('sort', params.sort)
  if (params.page) qs.set('page', String(params.page))
  if (params.pageSize) qs.set('pageSize', String(params.pageSize))

  const suffix = qs.toString() ? `?${qs.toString()}` : ''
  const remote = await tryApi<{
    users: AdminUser[]
    total: number
    page: number
    pageSize: number
  }>(`/api/admin/users${suffix}`)

  if (remote?.users) {
    return {
      users: remote.users,
      total: remote.total,
      page: remote.page,
      pageSize: remote.pageSize,
      source: 'remote',
    }
  }

  const all = readLocalUsers()
  const q = params.search?.trim().toLowerCase() ?? ''
  let list = all.filter((u) => {
    if (params.state && params.state !== 'all' && u.state !== params.state) return false
    if (params.group && params.group !== 'all' && u.group !== params.group) return false
    if (!q) return true
    return (
      u.name.toLowerCase().includes(q) ||
      u.id.toLowerCase().includes(q) ||
      u.group.toLowerCase().includes(q)
    )
  })

  if (params.sort === 'usage') {
    list = [...list].sort((a, b) => {
      const ra = a.monthlyTokens > 0 ? a.usedTokens / a.monthlyTokens : 0
      const rb = b.monthlyTokens > 0 ? b.usedTokens / b.monthlyTokens : 0
      return rb - ra
    })
  } else if (params.sort === 'requests') {
    list = [...list].sort((a, b) => b.usedRequests - a.usedRequests)
  } else {
    list = [...list].sort((a, b) => b.createdAtMs - a.createdAtMs)
  }

  const page = Math.max(1, params.page ?? 1)
  const pageSize = Math.max(1, params.pageSize ?? 50)
  const start = (page - 1) * pageSize
  return {
    users: list.slice(start, start + pageSize),
    total: list.length,
    page,
    pageSize,
    source: 'local',
  }
}

export async function fetchAdminOverview(localUsers?: AdminUser[]): Promise<AdminOverview> {
  const remote = await tryApi<{
    metrics: AdminMetrics
    trend: DailyTrendPoint[]
    connections: ActiveConnection[]
    archives: ArchiveSummary[]
    health: HealthSnapshot
    maxConns: number
  }>('/api/admin/overview')

  if (remote?.health) {
    return {
      source: 'remote',
      metrics: remote.metrics,
      trend: remote.trend ?? [],
      connections: remote.connections ?? [],
      archives: remote.archives ?? [],
      health: remote.health,
      maxConns: remote.maxConns ?? remote.health.maxConns ?? 5000,
    }
  }

  const nowMs = Date.now()
  const users = localUsers ?? readLocalUsers()
  const activeAdmins = users.filter((u) => u.role === 'admin' && u.state === 'active').length
  const activeCount = users.filter((u) => u.state === 'active').length
  const connections = buildLocalConnections(nowMs)

  return {
    source: 'local',
    metrics: {
      totalUsers: users.length,
      activeUsers: activeCount,
      disabledUsers: users.length - activeCount,
      adminUsers: users.filter((u) => u.role === 'admin').length,
      activeAdminUsers: activeAdmins,
      totalMonthlyQuota: users.reduce((s, u) => s + u.monthlyTokens, 0),
      totalUsedTokens: users.reduce((s, u) => s + u.usedTokens, 0),
      totalUsedRequests: users.reduce((s, u) => s + u.usedRequests, 0),
    },
    trend: buildLocalTrend(nowMs),
    connections,
    archives: buildLocalArchives(nowMs),
    maxConns: 5000,
    health: {
      ok: activeAdmins > 0,
      ver: 1,
      dbMode: 'local-preview',
      schemaVer: 2,
      maxConns: 5000,
      activeConns: connections.length,
      activeAdmins,
      checks: [
        { key: 'storage_write', ok: false, value: 'local-preview' },
        { key: 'schema_version', ok: true, value: 'v2/v2' },
        { key: 'active_admins', ok: activeAdmins > 0, value: String(activeAdmins) },
        { key: 'conn_capacity', ok: true, value: `${connections.length}/5000` },
      ],
    },
  }
}

export async function createAdminUser(
  payload: CreateUserPayload,
): Promise<{ user: AdminUser; token: string }> {
  const name = payload.name.trim()
  const group = payload.group.trim()
  if (!name) throw new Error('请输入用户姓名')
  if (!group) throw new Error('请选择或填写所属分组')

  const remote = await tryApi<{ user: AdminUser; token: string }>('/api/admin/users', {
    method: 'POST',
    body: JSON.stringify(payload),
  })
  if (remote?.user) return remote

  const users = readLocalUsers()
  if (users.some((u) => u.name === name && u.group === group)) {
    throw new Error(`「${group}」中已存在同名成员「${name}」`)
  }
  const token = `jg_${randomHex(32)}`
  const user: AdminUser = {
    id: `u_${randomHex(16)}`,
    name,
    group,
    role: payload.role,
    state: 'active',
    dailyRequests: payload.dailyRequests,
    monthlyTokens: payload.monthlyTokens,
    usedRequests: 0,
    usedTokens: 0,
    tokenHint: token.slice(0, 7),
    createdAtMs: Date.now(),
    lastActiveMs: 0,
  }
  writeLocalUsers([user, ...users])
  return { user, token }
}

export async function updateAdminUser(
  id: string,
  payload: UpdateUserPayload,
): Promise<AdminUser> {
  const remote = await tryApi<{ user: AdminUser }>(`/api/admin/users/${id}`, {
    method: 'PATCH',
    body: JSON.stringify(payload),
  })
  if (remote?.user) return remote.user

  const users = readLocalUsers()
  const target = users.find((u) => u.id === id)
  if (!target) throw new Error('用户不存在')

  const nextRole = payload.role ?? target.role
  const nextState = payload.state ?? target.state
  if (isLastActiveAdmin(users, id) && (nextRole !== 'admin' || nextState !== 'active')) {
    throw new Error('至少保留一位启用中的管理员')
  }

  const updated: AdminUser = {
    ...target,
    name: payload.name !== undefined ? payload.name.trim() || target.name : target.name,
    group: payload.group !== undefined ? payload.group.trim() || target.group : target.group,
    role: nextRole,
    state: nextState,
    dailyRequests: payload.dailyRequests ?? target.dailyRequests,
    monthlyTokens: payload.monthlyTokens ?? target.monthlyTokens,
  }
  writeLocalUsers(users.map((u) => (u.id === id ? updated : u)))
  return updated
}

export async function rotateAdminUserToken(
  id: string,
): Promise<{ user: AdminUser; token: string }> {
  const remote = await tryApi<{ user: AdminUser; token: string }>(`/api/admin/users/${id}/token`, {
    method: 'POST',
  })
  if (remote?.user) return remote

  const users = readLocalUsers()
  const target = users.find((u) => u.id === id)
  if (!target) throw new Error('用户不存在')
  const token = `jg_${randomHex(32)}`
  const updated: AdminUser = {
    ...target,
    tokenHint: token.slice(0, 7),
  }
  writeLocalUsers(users.map((u) => (u.id === id ? updated : u)))
  return { user: updated, token }
}

export async function deleteAdminUser(id: string): Promise<void> {
  const remote = await tryApi<{ ok: boolean }>(`/api/admin/users/${id}`, {
    method: 'DELETE',
  })
  if (remote?.ok) return

  const users = readLocalUsers()
  if (isLastActiveAdmin(users, id)) {
    throw new Error('不能删除最后一位启用中的管理员')
  }
  writeLocalUsers(users.filter((u) => u.id !== id))
}

export async function batchSetUsersState(ids: string[], state: UserState): Promise<void> {
  const remote = await tryApi<{ updated: number }>('/api/admin/users/batch-state', {
    method: 'PATCH',
    body: JSON.stringify({ ids, state }),
  })
  if (remote !== null) return

  const users = readLocalUsers()
  const idSet = new Set(ids)
  const remainingActiveAdmins = users.filter(
    (u) => u.role === 'admin' && u.state === 'active' && !(idSet.has(u.id) && state === 'disabled'),
  )
  if (remainingActiveAdmins.length === 0) {
    throw new Error('批量停用后将没有可用的管理员，已拦截')
  }
  const next = users.map((u) => (idSet.has(u.id) ? { ...u, state } : u))
  writeLocalUsers(next)
}

export function calcHitRatePct(point: DailyTrendPoint): number {
  if (point.promptTokens <= 0) return 0
  return Math.min(100, Math.round((point.cachedTokens / point.promptTokens) * 1000) / 10)
}

export function calcTotalTokens(point: DailyTrendPoint): number {
  return point.promptTokens + point.completionTokens
}

export function formatDateMs(ms: number): string {
  if (!ms || ms <= 0) return '-'
  const d = new Date(ms)
  const y = d.getFullYear()
  const m = String(d.getMonth() + 1).padStart(2, '0')
  const day = String(d.getDate()).padStart(2, '0')
  return `${y}-${m}-${day}`
}

export function formatMonthDayMs(ms: number): string {
  if (!ms || ms <= 0) return '--/--'
  const d = new Date(ms)
  const m = String(d.getMonth() + 1).padStart(2, '0')
  const day = String(d.getDate()).padStart(2, '0')
  return `${m}/${day}`
}

export function formatRelativeMs(ms: number, emptyLabel = '尚未活跃'): string {
  if (!ms || ms <= 0) return emptyLabel
  const diffMs = Math.max(0, Date.now() - ms)
  const mins = Math.floor(diffMs / 60_000)
  if (mins < 1) return '刚刚'
  if (mins < 60) return `${mins} 分钟前`
  const hours = Math.floor(mins / 60)
  if (hours < 24) return `${hours} 小时前`
  return formatDateMs(ms)
}

export function formatUptimeMs(connectedAtMs: number): string {
  if (!connectedAtMs || connectedAtMs <= 0) return '00m 00s'
  const totalSecs = Math.max(0, Math.floor((Date.now() - connectedAtMs) / 1000))
  const mins = Math.floor(totalSecs / 60)
  const secs = totalSecs % 60
  if (mins >= 60) {
    const hours = Math.floor(mins / 60)
    const remMins = mins % 60
    return `${hours}h ${String(remMins).padStart(2, '0')}m`
  }
  return `${String(mins).padStart(2, '0')}m ${String(secs).padStart(2, '0')}s`
}

export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${Math.max(1, Math.round(bytes / 1024))} KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
}

export function formatTokens(n: number): string {
  if (n === 0) return '不限'
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(n % 1_000_000 === 0 ? 0 : 2)}M`
  if (n >= 1_000) return `${Math.round(n / 1_000)}K`
  return String(n)
}

export function formatCount(n: number): string {
  return n.toLocaleString('en-US')
}

export function exportUsersCsv(users: AdminUser[]): void {
  const header = ['ID', '姓名', '分组', '角色', '状态', '已用Token', '月度配额', '今日请求', '日请求上限', '创建时间']
  const rows = users.map((u) => [
    u.id,
    u.name,
    u.group,
    u.role === 'admin' ? '管理员' : '成员',
    u.state === 'active' ? '启用' : '停用',
    String(u.usedTokens),
    String(u.monthlyTokens),
    String(u.usedRequests),
    String(u.dailyRequests),
    formatDateMs(u.createdAtMs),
  ])
  const csv = [header, ...rows].map((r) => r.map((v) => `"${v.replaceAll('"', '""')}"`).join(',')).join('\n')
  const blob = new Blob([`\uFEFF${csv}`], { type: 'text/csv;charset=utf-8;' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = `users_${formatDateMs(Date.now())}.csv`
  a.click()
  URL.revokeObjectURL(url)
}

export type UserRole = 'admin' | 'member'

export type UserState = 'active' | 'disabled'

export type AdminTab = 'users' | 'connections' | 'metering' | 'archives' | 'health'

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
  tokenFingerprint: string
  createdAt: string
  lastActiveAt: string
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

export type DailyTrendPoint = {
  day: string
  requests: number
  hitRate: number
  tokensK: number
}

export type ActiveConnection = {
  id: string
  userId: string
  userName: string
  convId: string
  ip: string
  uptime: string
  inflight: boolean
}

export type ArchiveSummary = {
  convId: string
  ownerName: string
  group: string
  messages: number
  prefixVersion: number
  bytesKB: number
  updatedAt: string
}

export type HealthCheckItem = {
  name: string
  detail: string
  ok: boolean
}

export type HealthSnapshot = {
  ok: boolean
  ver: number
  dbMode: string
  schemaVer: number
  maxConns: number
  activeConns: number
  checks: HealthCheckItem[]
}

export type AdminOverview = {
  source: 'remote' | 'local'
  trend: DailyTrendPoint[]
  connections: ActiveConnection[]
  archives: ArchiveSummary[]
  health: HealthSnapshot
  maxConns: number
}

const STORAGE_KEY = 'jiagou.admin.users.v1'

const SEED_USERS: AdminUser[] = [
  {
    id: 'u_01H8QK92',
    name: '陈默',
    group: '研发',
    role: 'admin',
    state: 'active',
    dailyRequests: 1000,
    monthlyTokens: 4_000_000,
    usedRequests: 312,
    usedTokens: 3_120_000,
    tokenFingerprint: 'a94f2c18',
    createdAt: '2026-09-02',
    lastActiveAt: '2 分钟前',
  },
  {
    id: 'u_01H8QN47',
    name: '李渝',
    group: '售前',
    role: 'admin',
    state: 'active',
    dailyRequests: 800,
    monthlyTokens: 4_000_000,
    usedRequests: 196,
    usedTokens: 2_100_000,
    tokenFingerprint: '71bd0e4a',
    createdAt: '2026-09-11',
    lastActiveAt: '9 分钟前',
  },
  {
    id: 'u_01H8QT13',
    name: '王岚',
    group: '运营',
    role: 'member',
    state: 'disabled',
    dailyRequests: 300,
    monthlyTokens: 1_500_000,
    usedRequests: 300,
    usedTokens: 1_500_000,
    tokenFingerprint: '3c82a91f',
    createdAt: '2026-10-04',
    lastActiveAt: '3 天前',
  },
  {
    id: 'u_01H8R205',
    name: '赵隅',
    group: '交付',
    role: 'member',
    state: 'active',
    dailyRequests: 600,
    monthlyTokens: 3_000_000,
    usedRequests: 148,
    usedTokens: 1_840_000,
    tokenFingerprint: 'e402c57b',
    createdAt: '2026-10-22',
    lastActiveAt: '26 分钟前',
  },
  {
    id: 'u_01H8RA81',
    name: '周砚',
    group: '研发',
    role: 'member',
    state: 'active',
    dailyRequests: 800,
    monthlyTokens: 3_500_000,
    usedRequests: 245,
    usedTokens: 2_690_000,
    tokenFingerprint: '90fa6d12',
    createdAt: '2026-10-29',
    lastActiveAt: '1 小时前',
  },
  {
    id: 'u_01H8RF39',
    name: '林知夏',
    group: '售前',
    role: 'member',
    state: 'active',
    dailyRequests: 500,
    monthlyTokens: 2_000_000,
    usedRequests: 92,
    usedTokens: 860_000,
    tokenFingerprint: '55ce108e',
    createdAt: '2026-11-03',
    lastActiveAt: '4 小时前',
  },
  {
    id: 'u_01H8RM64',
    name: '许宁',
    group: '产品',
    role: 'member',
    state: 'active',
    dailyRequests: 500,
    monthlyTokens: 2_000_000,
    usedRequests: 64,
    usedTokens: 540_000,
    tokenFingerprint: '1b90f32c',
    createdAt: '2026-11-10',
    lastActiveAt: '昨天',
  },
  {
    id: 'u_01H8RT08',
    name: '沈远',
    group: '交付',
    role: 'member',
    state: 'disabled',
    dailyRequests: 300,
    monthlyTokens: 1_000_000,
    usedRequests: 0,
    usedTokens: 120_000,
    tokenFingerprint: 'c7d481a9',
    createdAt: '2026-11-15',
    lastActiveAt: '5 天前',
  },
]

export const weeklyTrend: DailyTrendPoint[] = [
  { day: '11/14', requests: 890, hitRate: 71, tokensK: 1420 },
  { day: '11/15', requests: 1020, hitRate: 74, tokensK: 1680 },
  { day: '11/16', requests: 940, hitRate: 73, tokensK: 1510 },
  { day: '11/17', requests: 1180, hitRate: 76, tokensK: 1940 },
  { day: '11/18', requests: 1310, hitRate: 79, tokensK: 2180 },
  { day: '11/19', requests: 1240, hitRate: 77, tokensK: 2040 },
  { day: '11/20', requests: 1357, hitRate: 81, tokensK: 2290 },
]

export const activeConnections: ActiveConnection[] = [
  { id: 'ws_9f2a', userId: 'u_01H8QK92', userName: '陈默', convId: 'conv_tender_03', ip: '10.24.8.14', uptime: '42m 18s', inflight: true },
  { id: 'ws_9f3c', userId: 'u_01H8QN47', userName: '李渝', convId: 'conv_bid_alpha', ip: '10.24.8.31', uptime: '19m 04s', inflight: false },
  { id: 'ws_9f51', userId: 'u_01H8R205', userName: '赵隅', convId: 'conv_delivery_b', ip: '10.24.9.07', uptime: '06m 52s', inflight: true },
]

export const archiveSummaries: ArchiveSummary[] = [
  { convId: 'conv_tender_03', ownerName: '陈默', group: '研发', messages: 84, prefixVersion: 3, bytesKB: 412, updatedAt: '2 分钟前' },
  { convId: 'conv_bid_alpha', ownerName: '李渝', group: '售前', messages: 62, prefixVersion: 3, bytesKB: 318, updatedAt: '9 分钟前' },
  { convId: 'conv_delivery_b', ownerName: '赵隅', group: '交付', messages: 49, prefixVersion: 3, bytesKB: 256, updatedAt: '26 分钟前' },
  { convId: 'conv_weekly_47', ownerName: '周砚', group: '研发', messages: 31, prefixVersion: 3, bytesKB: 148, updatedAt: '1 小时前' },
]

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
  const chars = '0123456789abcdef'
  let out = ''
  for (let i = 0; i < len; i++) {
    out += chars[Math.floor(Math.random() * chars.length)] ?? '0'
  }
  return out
}

function todayStr(): string {
  return new Date().toISOString().slice(0, 10)
}

function isLastActiveAdmin(users: AdminUser[], id: string): boolean {
  const activeAdmins = users.filter((u) => u.role === 'admin' && u.state === 'active')
  return activeAdmins.length === 1 && activeAdmins[0]?.id === id
}

async function tryApi<T>(path: string, init?: RequestInit): Promise<T | null> {
  try {
    const res = await fetch(path, {
      headers: { 'Content-Type': 'application/json' },
      ...init,
    })
    if (!res.ok) {
      const contentType = res.headers.get('content-type') ?? ''
      if (contentType.includes('application/json')) {
        const err = (await res.json()) as { message?: string }
        if (err.message) throw new Error(err.message)
      }
      return null
    }
    const contentType = res.headers.get('content-type') ?? ''
    if (!contentType.includes('application/json')) return null
    return (await res.json()) as T
  } catch (e) {
    if (e instanceof Error && e.message && !e.message.includes('Failed to fetch')) {
      throw e
    }
    return null
  }
}

export async function fetchAdminUsers(): Promise<AdminUser[]> {
  const remote = await tryApi<{ users: AdminUser[] }>('/api/admin/users')
  if (remote?.users) {
    writeLocalUsers(remote.users)
    return remote.users
  }
  return readLocalUsers()
}

export async function fetchAdminOverview(localUsers?: AdminUser[]): Promise<AdminOverview> {
  const remote = await tryApi<{
    trend?: DailyTrendPoint[]
    connections?: ActiveConnection[]
    archives?: ArchiveSummary[]
    health?: HealthSnapshot
    maxConns?: number
  }>('/api/admin/overview')

  if (remote?.health) {
    return {
      source: 'remote',
      trend: remote.trend ?? [],
      connections: remote.connections ?? [],
      archives: remote.archives ?? [],
      health: remote.health,
      maxConns: remote.maxConns ?? remote.health.maxConns ?? 5000,
    }
  }

  const users = localUsers ?? readLocalUsers()
  const activeAdmins = users.filter((u) => u.role === 'admin' && u.state === 'active').length
  return {
    source: 'local',
    trend: weeklyTrend,
    connections: activeConnections,
    archives: archiveSummaries,
    maxConns: 5000,
    health: {
      ok: activeAdmins > 0,
      ver: 1,
      dbMode: 'local-fallback',
      schemaVer: 1,
      maxConns: 5000,
      activeConns: activeConnections.length,
      checks: [
        { name: 'HTTP /healthz 探针', detail: '当前未检测到后端服务 · 使用本地回退数据', ok: false },
        { name: 'SQLite 存储与写通道', detail: '本地浏览器预览模式（未连接引擎）', ok: true },
        { name: 'Schema 版本迁移', detail: 'schema_ver = v1（预期基线）', ok: true },
        { name: '管理员引导与隔离检查', detail: `本地启用管理员 ${activeAdmins} 人`, ok: activeAdmins > 0 },
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
  const token = `jg_${randomHex(24)}`
  const user: AdminUser = {
    id: `u_${randomHex(8).toUpperCase()}`,
    name,
    group,
    role: payload.role,
    state: 'active',
    dailyRequests: payload.dailyRequests,
    monthlyTokens: payload.monthlyTokens,
    usedRequests: 0,
    usedTokens: 0,
    tokenFingerprint: token.slice(3, 11),
    createdAt: todayStr(),
    lastActiveAt: '刚刚创建',
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
  const token = `jg_${randomHex(24)}`
  const updated: AdminUser = {
    ...target,
    tokenFingerprint: token.slice(3, 11),
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

export async function batchSetUsersState(ids: string[], state: UserState): Promise<AdminUser[]> {
  const remote = await tryApi<{ users: AdminUser[] }>('/api/admin/users/batch-state', {
    method: 'PATCH',
    body: JSON.stringify({ ids, state }),
  })
  if (remote?.users) {
    writeLocalUsers(remote.users)
    return remote.users
  }

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
  return next
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
    u.createdAt,
  ])
  const csv = [header, ...rows].map((r) => r.map((v) => `"${v.replaceAll('"', '""')}"`).join(',')).join('\n')
  const blob = new Blob([`\uFEFF${csv}`], { type: 'text/csv;charset=utf-8;' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = `users_${todayStr()}.csv`
  a.click()
  URL.revokeObjectURL(url)
}

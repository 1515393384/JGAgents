import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { Link } from '@tanstack/react-router'
import {
  ArrowLeft,
  Check,
  ChevronLeft,
  ChevronRight,
  Copy,
  Download,
  KeyRound,
  Lock,
  MoreHorizontal,
  Plus,
  RefreshCw,
  Search,
  ShieldAlert,
  UserCheck,
  UserX,
  Users,
  X,
} from 'lucide-react'
import { Avatar, Bar, Button, Stat } from '../../components/ui'
import { ThemeMenu } from '../../components/theme-menu'
import {
  AdminAuthError,
  batchSetUsersState,
  calcHitRatePct,
  clearAdminToken,
  createAdminUser,
  deleteAdminUser,
  exportUsersCsv,
  fetchAdminOverview,
  fetchAdminUsers,
  formatCount,
  formatRelativeMs,
  formatTokens,
  getAdminToken,
  rotateAdminUserToken,
  setAdminToken,
  updateAdminUser,
  type AdminOverview,
  type AdminTab,
  type AdminUser,
  type CreateUserPayload,
  type UpdateUserPayload,
  type UserSortKey,
  type UserState,
} from '../../lib/admin-users'
import { CreateUserDialog } from './create-user-dialog'
import { AdminSubView } from './sub-views'
import { UserCharts } from './user-charts'
import { UserDrawer } from './user-drawer'

const TABS: { id: AdminTab; label: string }[] = [
  { id: 'users', label: '用户' },
  { id: 'connections', label: '连接' },
  { id: 'metering', label: '计量' },
  { id: 'archives', label: '存档' },
  { id: 'health', label: '健康' },
]

const PAGE_SIZE = 20

export function AdminPage() {
  const [tab, setTab] = useState<AdminTab>('users')
  const [users, setUsers] = useState<AdminUser[]>([])
  const [totalUsers, setTotalUsers] = useState(0)
  const [page, setPage] = useState(1)
  const [overview, setOverview] = useState<AdminOverview | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState('')
  const [authRequired, setAuthRequired] = useState(false)
  const [authMessage, setAuthMessage] = useState('')
  const [tokenInput, setTokenInput] = useState('')
  const [authSubmitting, setAuthSubmitting] = useState(false)

  const [query, setQuery] = useState('')
  const [debouncedQuery, setDebouncedQuery] = useState('')
  const [stateFilter, setStateFilter] = useState<'all' | UserState>('all')
  const [groupFilter, setGroupFilter] = useState('all')
  const [sortBy, setSortBy] = useState<UserSortKey>('created')
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set())
  const [activeUserId, setActiveUserId] = useState<string | null>(null)
  const [createOpen, setCreateOpen] = useState(false)
  const [copiedId, setCopiedId] = useState<string | null>(null)
  const searchRef = useRef<HTMLInputElement>(null)

  useEffect(() => {
    const timer = window.setTimeout(() => {
      setDebouncedQuery(query)
      setPage(1)
    }, 220)
    return () => window.clearTimeout(timer)
  }, [query])

  const handleAuthFailure = useCallback((err: unknown): boolean => {
    if (err instanceof AdminAuthError) {
      setAuthRequired(true)
      setAuthMessage(err.message)
      setError('')
      return true
    }
    return false
  }, [])

  const load = useCallback(async () => {
    setLoading(true)
    setError('')
    try {
      const pageRes = await fetchAdminUsers({
        search: debouncedQuery,
        state: stateFilter,
        group: groupFilter,
        sort: sortBy,
        page,
        pageSize: PAGE_SIZE,
      })
      setUsers(pageRes.users)
      setTotalUsers(pageRes.total)

      const ov = await fetchAdminOverview(pageRes.users)
      setOverview(ov)
      setAuthRequired(false)
      setAuthMessage('')
    } catch (err) {
      if (!handleAuthFailure(err)) {
        setError(err instanceof Error ? err.message : '加载用户数据失败')
      }
    } finally {
      setLoading(false)
    }
  }, [debouncedQuery, stateFilter, groupFilter, sortBy, page, handleAuthFailure])

  useEffect(() => {
    void load()
  }, [load])

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (
        e.key === '/' &&
        !authRequired &&
        document.activeElement?.tagName !== 'INPUT' &&
        document.activeElement?.tagName !== 'TEXTAREA' &&
        document.activeElement?.tagName !== 'SELECT'
      ) {
        e.preventDefault()
        searchRef.current?.focus()
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [authRequired])

  const handleUnlock = async (e: React.FormEvent) => {
    e.preventDefault()
    const trimmed = tokenInput.trim()
    if (!trimmed) {
      setAuthMessage('请输入管理员访问凭证')
      return
    }
    setAuthSubmitting(true)
    setAdminToken(trimmed)
    try {
      const pageRes = await fetchAdminUsers({
        search: debouncedQuery,
        state: stateFilter,
        group: groupFilter,
        sort: sortBy,
        page,
        pageSize: PAGE_SIZE,
      })
      const ov = await fetchAdminOverview(pageRes.users)
      setUsers(pageRes.users)
      setTotalUsers(pageRes.total)
      setOverview(ov)
      setAuthRequired(false)
      setAuthMessage('')
      setTokenInput('')
    } catch (err) {
      clearAdminToken()
      if (err instanceof AdminAuthError) {
        setAuthMessage(err.message)
      } else {
        setAuthMessage(err instanceof Error ? err.message : '凭证验证失败')
      }
    } finally {
      setAuthSubmitting(false)
    }
  }

  const handleLockSession = () => {
    clearAdminToken()
    setUsers([])
    setOverview(null)
    setSelectedIds(new Set())
    setActiveUserId(null)
    setAuthRequired(true)
    setAuthMessage('已退出管理凭证，请重新验证')
  }

  const groups = useMemo(() => {
    return Array.from(new Set(users.map((u) => u.group))).sort()
  }, [users])

  const activeUser = useMemo(
    () => users.find((u) => u.id === activeUserId) ?? null,
    [users, activeUserId],
  )

  const summary = useMemo(() => {
    const m = overview?.metrics
    const scaleTotal = m?.totalUsers ?? totalUsers
    const activeCount = m?.activeUsers ?? users.filter((u) => u.state === 'active').length
    const disabledCount = m?.disabledUsers ?? Math.max(0, scaleTotal - activeCount)
    const usedTokens = m?.totalUsedTokens ?? users.reduce((s, u) => s + u.usedTokens, 0)
    const totalQuota = m?.totalMonthlyQuota ?? users.reduce((s, u) => s + u.monthlyTokens, 0)
    const userRequests = m?.totalUsedRequests ?? users.reduce((s, u) => s + u.usedRequests, 0)
    const pct = totalQuota > 0 ? Math.round((usedTokens / totalQuota) * 100) : 0
    const trend = overview?.trend ?? []
    const latestPoint = trend[trend.length - 1]
    const todayRequests =
      latestPoint && overview?.source === 'remote' ? latestPoint.requests : userRequests
    const latestHit = latestPoint ? calcHitRatePct(latestPoint) : 0
    const connCount = overview?.connections.length ?? 0
    const maxConns = overview?.maxConns ?? 5000
    const dbMode =
      overview?.source === 'remote' ? `SQLite ${overview.health.dbMode}` : '本地预览模式'
    return {
      scaleTotal,
      activeCount,
      disabledCount,
      usedTokens,
      totalQuota,
      todayRequests,
      pct,
      latestHit,
      connCount,
      maxConns,
      dbMode,
    }
  }, [users, overview, totalUsers])

  const totalPages = Math.max(1, Math.ceil(totalUsers / PAGE_SIZE))
  const allPageSelected = users.length > 0 && users.every((u) => selectedIds.has(u.id))

  const toggleAll = () => {
    if (allPageSelected) {
      setSelectedIds(new Set())
      return
    }
    setSelectedIds(new Set(users.map((u) => u.id)))
  }

  const toggleRow = (id: string) => {
    setSelectedIds((prev) => {
      const next = new Set(prev)
      if (next.has(id)) next.delete(id)
      else next.add(id)
      return next
    })
  }

  const handleCreate = async (payload: CreateUserPayload) => {
    try {
      const { user, token } = await createAdminUser(payload)
      await load()
      return { token, name: user.name }
    } catch (err) {
      handleAuthFailure(err)
      throw err
    }
  }

  const handleSaveUser = async (id: string, payload: UpdateUserPayload) => {
    try {
      await updateAdminUser(id, payload)
      await load()
    } catch (err) {
      handleAuthFailure(err)
      throw err
    }
  }

  const handleQuickToggleState = async (user: AdminUser) => {
    setError('')
    try {
      const nextState: UserState = user.state === 'active' ? 'disabled' : 'active'
      await updateAdminUser(user.id, { state: nextState })
      await load()
    } catch (err) {
      if (!handleAuthFailure(err)) {
        setError(err instanceof Error ? err.message : '切换状态失败')
      }
    }
  }

  const handleRotateToken = async (id: string) => {
    try {
      const { token } = await rotateAdminUserToken(id)
      await load()
      return token
    } catch (err) {
      handleAuthFailure(err)
      throw err
    }
  }

  const handleDeleteUser = async (id: string) => {
    try {
      await deleteAdminUser(id)
      setSelectedIds((prev) => {
        const next = new Set(prev)
        next.delete(id)
        return next
      })
      await load()
    } catch (err) {
      handleAuthFailure(err)
      throw err
    }
  }

  const handleBatchState = async (state: UserState) => {
    setError('')
    try {
      await batchSetUsersState(Array.from(selectedIds), state)
      setSelectedIds(new Set())
      await load()
    } catch (err) {
      if (!handleAuthFailure(err)) {
        setError(err instanceof Error ? err.message : '批量操作失败')
      }
    }
  }

  const handleCopyId = async (id: string) => {
    await navigator.clipboard.writeText(id)
    setCopiedId(id)
    window.setTimeout(() => setCopiedId((cur) => (cur === id ? null : cur)), 1400)
  }

  const resetFilters = () => {
    setQuery('')
    setDebouncedQuery('')
    setStateFilter('all')
    setGroupFilter('all')
    setPage(1)
  }

  const hasToken = Boolean(getAdminToken())

  return (
    <div className="h-full overflow-y-auto bg-bg text-fg">
      <header className="sticky top-0 z-20 border-b border-surface-border bg-bg/90 backdrop-blur-md">
        <div className="mx-auto flex h-14 max-w-[1400px] items-center justify-between gap-4 px-4 md:px-6">
          <div className="flex min-w-0 items-center gap-3">
            <Link
              to="/"
              className="inline-flex h-8 items-center gap-1.5 rounded-field px-2 text-xs text-fg-muted transition-colors hover:bg-bg-subtle hover:text-fg whitespace-nowrap shrink-0"
            >
              <ArrowLeft size={14} />
              返回工作台
            </Link>
            <span className="text-border-strong" aria-hidden>
              /
            </span>
            <h1 className="truncate text-sm font-medium tracking-tight text-fg whitespace-nowrap">
              中控台
            </h1>
          </div>

          <div className="flex items-center gap-2 shrink-0">
            {hasToken && !authRequired ? (
              <Button
                variant="outline"
                onClick={handleLockSession}
                className="whitespace-nowrap shrink-0"
              >
                <Lock size={13} />
                锁定
              </Button>
            ) : null}
            <Button
              variant="outline"
              onClick={() => void load()}
              disabled={loading}
              aria-label="刷新数据"
              className="whitespace-nowrap shrink-0"
            >
              <RefreshCw size={14} className={loading ? 'animate-spin' : ''} />
              刷新
            </Button>
            <ThemeMenu />
            {!authRequired ? (
              <Button
                variant="primary"
                onClick={() => setCreateOpen(true)}
                className="whitespace-nowrap shrink-0"
              >
                <Plus size={15} />
                新建用户
              </Button>
            ) : null}
          </div>
        </div>
      </header>

      {authRequired ? (
        <main className="mx-auto flex max-w-[440px] flex-col justify-center px-4 py-16">
          <form
            onSubmit={handleUnlock}
            className="rounded-panel border border-surface-border bg-surface p-6 shadow-card"
          >
            <div className="flex items-center gap-2.5 text-sm font-medium text-fg">
              <KeyRound size={16} className="text-brand" />
              管理员凭证验证
            </div>
            <p className="mt-2 text-xs leading-5 text-fg-muted">
              管理接口受 Bearer 鉴权保护。请粘贴服务端首次引导文件{' '}
              <code className="rounded bg-surface-sunken px-1 py-0.5 font-mono text-[11px] text-fg">
                data/.bootstrap-admin-token
              </code>{' '}
              或已签发管理员 Token 解锁控制台。
            </p>

            {authMessage ? (
              <div className="mt-3 flex items-start gap-2 rounded-field border border-danger/35 bg-danger/10 px-3 py-2 text-xs text-danger">
                <ShieldAlert size={14} className="mt-0.5 shrink-0" />
                <span>{authMessage}</span>
              </div>
            ) : null}

            <label className="mt-4 block">
              <span className="text-xs text-fg-muted">管理员 Bearer Token</span>
              <input
                autoFocus
                type="password"
                value={tokenInput}
                onChange={(e) => setTokenInput(e.target.value)}
                placeholder="jg_..."
                className="mt-1.5 h-9 w-full rounded-field border border-border bg-surface-sunken px-3 font-mono text-sm text-fg placeholder:text-fg-subtle focus-visible:border-brand focus-visible:outline-none"
              />
            </label>

            <div className="mt-4 flex justify-end gap-2">
              <Button type="submit" variant="primary" disabled={authSubmitting}>
                {authSubmitting ? '验证中…' : '验证并进入'}
              </Button>
            </div>
          </form>
        </main>
      ) : (
        <main className="mx-auto max-w-[1400px] px-4 py-5 md:px-6 md:py-6">
          <div className="-mx-4 overflow-x-auto px-4 md:mx-0 md:px-0">
            <div className="flex items-center gap-1 border-b border-surface-border">
              {TABS.map((t) => {
                const active = tab === t.id
                return (
                  <button
                    key={t.id}
                    type="button"
                    onClick={() => setTab(t.id)}
                    className={
                      '-mb-px shrink-0 whitespace-nowrap border-b-2 px-3 py-2 text-sm transition-colors ' +
                      (active
                        ? 'border-brand font-medium text-fg'
                        : 'border-transparent text-fg-muted hover:text-fg')
                    }
                  >
                    {t.label}
                    {t.id === 'users' ? (
                      <span className="ml-1.5 font-mono text-xs text-fg-subtle tabular-nums">
                        {summary.scaleTotal}
                      </span>
                    ) : null}
                  </button>
                )
              })}
            </div>
          </div>

          <section className="mt-5 grid grid-cols-2 gap-3 lg:grid-cols-4">
            <Stat
              label="用户规模"
              value={<span className="tabular-nums">{summary.scaleTotal}</span>}
              hint={`启用 ${summary.activeCount} · 停用 ${summary.disabledCount}`}
            />
            <Stat
              label="本月 Token 消耗"
              value={<span className="tabular-nums">{formatTokens(summary.usedTokens)}</span>}
              hint={`总配额 ${formatTokens(summary.totalQuota)} · 已用 ${summary.pct}%`}
            />
            <Stat
              label="今日请求 / 前缀命中"
              value={
                <span className="tabular-nums">
                  {formatCount(summary.todayRequests)}{' '}
                  <span className="text-base font-normal text-ok">{summary.latestHit}%</span>
                </span>
              }
              hint="近 24 小时直通缓存命中率"
            />
            <Stat
              label="活跃连接"
              value={<span className="tabular-nums">{summary.connCount}</span>}
              hint={`${summary.dbMode} · 限额 ${formatCount(summary.maxConns)}`}
            />
          </section>

          {error ? (
            <div className="mt-4 flex items-center justify-between gap-3 rounded-field border border-danger/35 bg-danger/10 px-3.5 py-2.5 text-xs text-danger">
              <span className="flex items-center gap-2">
                <ShieldAlert size={14} className="shrink-0" />
                {error}
              </span>
              <button
                type="button"
                onClick={() => setError('')}
                className="text-danger/80 hover:text-danger"
                aria-label="关闭提示"
              >
                <X size={14} />
              </button>
            </div>
          ) : null}

          {tab === 'users' ? (
            <div className="mt-5 space-y-5">
              <UserCharts
                users={users}
                trend={overview?.trend ?? []}
                selectedGroup={groupFilter}
                onSelectGroup={(g) => {
                  setGroupFilter(g)
                  setPage(1)
                }}
              />

              <section className="rounded-card border border-surface-border bg-surface">
                <div className="flex flex-wrap items-center gap-2.5 border-b border-surface-border px-4 py-3">
                  <div className="relative min-w-[210px] flex-1 max-w-xs">
                    <Search
                      size={14}
                      className="pointer-events-none absolute left-2.5 top-1/2 -translate-y-1/2 text-fg-subtle"
                    />
                    <input
                      ref={searchRef}
                      value={query}
                      onChange={(e) => setQuery(e.target.value)}
                      placeholder="搜索姓名、ID 或分组（按 / 聚焦）"
                      className="h-9 w-full rounded-field border border-border bg-surface-sunken pl-8 pr-8 text-sm text-fg placeholder:text-fg-subtle focus-visible:border-brand focus-visible:outline-none"
                    />
                    {query ? (
                      <button
                        type="button"
                        onClick={() => {
                          setQuery('')
                          setDebouncedQuery('')
                          setPage(1)
                        }}
                        aria-label="清空搜索"
                        className="absolute right-2 top-1/2 -translate-y-1/2 text-fg-subtle hover:text-fg"
                      >
                        <X size={13} />
                      </button>
                    ) : null}
                  </div>

                  <div className="flex items-center gap-0.5 rounded-field bg-bg-subtle p-1">
                    {(
                      [
                        { id: 'all', label: '全部' },
                        { id: 'active', label: '启用' },
                        { id: 'disabled', label: '停用' },
                      ] as const
                    ).map((item) => (
                      <button
                        key={item.id}
                        type="button"
                        onClick={() => {
                          setStateFilter(item.id)
                          setPage(1)
                        }}
                        className={
                          'h-7 rounded-[7px] px-2.5 text-xs transition-colors whitespace-nowrap shrink-0 ' +
                          (stateFilter === item.id
                            ? 'bg-surface font-medium text-fg shadow-card'
                            : 'text-fg-muted hover:text-fg')
                        }
                      >
                        {item.label}
                      </button>
                    ))}
                  </div>

                  <select
                    value={groupFilter}
                    onChange={(e) => {
                      setGroupFilter(e.target.value)
                      setPage(1)
                    }}
                    aria-label="按分组筛选"
                    className="h-9 rounded-field border border-border bg-surface-sunken px-2.5 text-xs text-fg focus-visible:border-brand focus-visible:outline-none"
                  >
                    <option value="all">全部分组</option>
                    {groups.map((g) => (
                      <option key={g} value={g}>
                        {g}
                      </option>
                    ))}
                  </select>

                  <select
                    value={sortBy}
                    onChange={(e) => {
                      setSortBy(e.target.value as UserSortKey)
                      setPage(1)
                    }}
                    aria-label="排序方式"
                    className="h-9 rounded-field border border-border bg-surface-sunken px-2.5 text-xs text-fg focus-visible:border-brand focus-visible:outline-none"
                  >
                    <option value="created">按创建时间</option>
                    <option value="usage">按配额占用率</option>
                    <option value="requests">按今日请求量</option>
                  </select>

                  <div className="ml-auto flex items-center gap-2">
                    <Button
                      variant="outline"
                      onClick={() => exportUsersCsv(users)}
                      disabled={users.length === 0}
                      className="whitespace-nowrap shrink-0"
                    >
                      <Download size={14} />
                      导出当前页 CSV
                    </Button>
                  </div>
                </div>

                {selectedIds.size > 0 ? (
                  <div className="flex flex-wrap items-center justify-between gap-2 border-b border-surface-border bg-brand-soft px-4 py-2 text-xs">
                    <span className="font-medium text-fg tabular-nums">
                      已选择 {selectedIds.size} 位用户
                    </span>
                    <div className="flex items-center gap-1.5">
                      <Button
                        variant="outline"
                        onClick={() => void handleBatchState('active')}
                        className="h-7 px-2.5 text-xs whitespace-nowrap"
                      >
                        <UserCheck size={13} />
                        批量启用
                      </Button>
                      <Button
                        variant="outline"
                        onClick={() => void handleBatchState('disabled')}
                        className="h-7 px-2.5 text-xs whitespace-nowrap"
                      >
                        <UserX size={13} />
                        批量停用
                      </Button>
                      <Button
                        variant="outline"
                        onClick={() =>
                          exportUsersCsv(users.filter((u) => selectedIds.has(u.id)))
                        }
                        className="h-7 px-2.5 text-xs whitespace-nowrap"
                      >
                        导出所选
                      </Button>
                      <Button
                        variant="ghost"
                        onClick={() => setSelectedIds(new Set())}
                        className="h-7 px-2 text-xs whitespace-nowrap"
                      >
                        取消
                      </Button>
                    </div>
                  </div>
                ) : null}

                {loading ? (
                  <div className="divide-y divide-surface-border p-4">
                    {[0, 1, 2, 3, 4].map((row) => (
                      <div key={row} className="flex items-center gap-4 py-3">
                        <div className="h-4 w-4 rounded bg-bg-subtle animate-pulse" />
                        <div className="h-7 w-7 rounded-pill bg-bg-subtle animate-pulse" />
                        <div className="h-3.5 w-24 rounded bg-bg-subtle animate-pulse" />
                        <div className="h-3.5 w-20 rounded bg-bg-subtle animate-pulse" />
                        <div className="ml-auto h-3.5 w-40 rounded bg-bg-subtle animate-pulse" />
                      </div>
                    ))}
                  </div>
                ) : users.length === 0 ? (
                  <div className="flex flex-col items-center justify-center px-4 py-14 text-center">
                    <Users size={24} className="text-fg-subtle" />
                    <p className="mt-3 text-sm font-medium text-fg">
                      {summary.scaleTotal === 0 ? '尚未添加任何用户' : '没有符合筛选条件的用户'}
                    </p>
                    <p className="mt-1 text-xs text-fg-subtle">
                      {summary.scaleTotal === 0
                        ? '创建第一位成员并签发接入凭证，即可开始记录用量与配额。'
                        : '尝试更换关键词、状态或分组条件。'}
                    </p>
                    <div className="mt-4 flex items-center gap-2">
                      {summary.scaleTotal > 0 ? (
                        <Button variant="outline" onClick={resetFilters}>
                          清除筛选
                        </Button>
                      ) : null}
                      <Button variant="primary" onClick={() => setCreateOpen(true)}>
                        <Plus size={14} />
                        新建用户
                      </Button>
                    </div>
                  </div>
                ) : (
                  <>
                    <div className="overflow-x-auto">
                      <table className="w-full min-w-[820px] text-left text-sm">
                        <thead className="border-b border-surface-border bg-surface-sunken text-xs text-fg-muted">
                          <tr>
                            <th className="w-10 px-4 py-2.5">
                              <input
                                type="checkbox"
                                checked={allPageSelected}
                                onChange={toggleAll}
                                aria-label="全选当前列表"
                                className="h-3.5 w-3.5 rounded border-border accent-[var(--brand)]"
                              />
                            </th>
                            <th className="px-3 py-2.5 font-normal">用户</th>
                            <th className="px-3 py-2.5 font-normal">标识 ID</th>
                            <th className="px-3 py-2.5 font-normal">分组</th>
                            <th className="px-3 py-2.5 font-normal">状态</th>
                            <th className="w-[210px] px-3 py-2.5 font-normal">月度配额用量</th>
                            <th className="px-3 py-2.5 text-right font-normal">今日请求</th>
                            <th className="px-3 py-2.5 text-right font-normal">最近活跃</th>
                            <th className="px-4 py-2.5 text-right font-normal">操作</th>
                          </tr>
                        </thead>
                        <tbody>
                          {users.map((u) => {
                            const checked = selectedIds.has(u.id)
                            const pct =
                              u.monthlyTokens > 0
                                ? Math.min(100, Math.round((u.usedTokens / u.monthlyTokens) * 100))
                                : 0
                            const barTone =
                              u.state === 'disabled' ? 'muted' : pct >= 85 ? 'warn' : 'brand'

                            return (
                              <tr
                                key={u.id}
                                onClick={() => setActiveUserId(u.id)}
                                className={
                                  'cursor-pointer border-t border-surface-border transition-colors ' +
                                  (checked ? 'bg-brand-soft/60' : 'hover:bg-bg-subtle/50')
                                }
                              >
                                <td
                                  className="px-4 py-2.5"
                                  onClick={(e) => e.stopPropagation()}
                                >
                                  <input
                                    type="checkbox"
                                    checked={checked}
                                    onChange={() => toggleRow(u.id)}
                                    aria-label={`选择 ${u.name}`}
                                    className="h-3.5 w-3.5 rounded border-border accent-[var(--brand)]"
                                  />
                                </td>

                                <td className="whitespace-nowrap px-3 py-2.5">
                                  <div className="flex items-center gap-2.5">
                                    <Avatar
                                      name={u.name}
                                      size={26}
                                      tone={u.role === 'admin' ? 'brand' : 'muted'}
                                    />
                                    <span className="font-medium text-fg">{u.name}</span>
                                    {u.role === 'admin' ? (
                                      <span className="text-[11px] text-brand">管理员</span>
                                    ) : null}
                                  </div>
                                </td>

                                <td
                                  className="whitespace-nowrap px-3 py-2.5"
                                  onClick={(e) => e.stopPropagation()}
                                >
                                  <button
                                    type="button"
                                    onClick={() => void handleCopyId(u.id)}
                                    title="复制用户 ID"
                                    className="inline-flex items-center gap-1.5 font-mono text-xs text-fg-subtle hover:text-fg"
                                  >
                                    <span>{u.id}</span>
                                    {copiedId === u.id ? (
                                      <Check size={11} className="text-ok" />
                                    ) : (
                                      <Copy size={11} className="opacity-50" />
                                    )}
                                  </button>
                                </td>

                                <td className="whitespace-nowrap px-3 py-2.5 text-fg-muted">
                                  {u.group}
                                </td>

                                <td className="whitespace-nowrap px-3 py-2.5">
                                  <span
                                    className={
                                      'inline-flex items-center gap-1.5 text-xs ' +
                                      (u.state === 'active' ? 'text-ok' : 'text-fg-subtle')
                                    }
                                  >
                                    <span
                                      className={
                                        'h-1.5 w-1.5 rounded-full ' +
                                        (u.state === 'active' ? 'bg-ok' : 'bg-fg-subtle')
                                      }
                                    />
                                    {u.state === 'active' ? '启用' : '停用'}
                                  </span>
                                </td>

                                <td className="px-3 py-2.5">
                                  <div className="flex items-baseline justify-between font-mono text-xs tabular-nums">
                                    <span className="text-fg">{formatTokens(u.usedTokens)}</span>
                                    <span className="text-fg-subtle">
                                      / {formatTokens(u.monthlyTokens)} (
                                      {u.monthlyTokens > 0 ? `${pct}%` : '不限'})
                                    </span>
                                  </div>
                                  <div className="mt-1">
                                    <Bar
                                      value={u.usedTokens}
                                      cap={u.monthlyTokens || Math.max(1, u.usedTokens)}
                                      tone={barTone}
                                    />
                                  </div>
                                </td>

                                <td className="whitespace-nowrap px-3 py-2.5 text-right font-mono text-xs text-fg-muted tabular-nums">
                                  {formatCount(u.usedRequests)} /{' '}
                                  {u.dailyRequests === 0 ? '不限' : formatCount(u.dailyRequests)}
                                </td>

                                <td className="whitespace-nowrap px-3 py-2.5 text-right text-xs text-fg-subtle tabular-nums">
                                  {formatRelativeMs(u.lastActiveMs)}
                                </td>

                                <td
                                  className="whitespace-nowrap px-4 py-2.5 text-right"
                                  onClick={(e) => e.stopPropagation()}
                                >
                                  <div className="inline-flex items-center gap-1">
                                    <button
                                      type="button"
                                      onClick={() => void handleQuickToggleState(u)}
                                      className="rounded-field px-2 py-1 text-xs text-fg-muted transition-colors hover:bg-bg-subtle hover:text-fg"
                                    >
                                      {u.state === 'active' ? '停用' : '启用'}
                                    </button>
                                    <button
                                      type="button"
                                      onClick={() => setActiveUserId(u.id)}
                                      aria-label="编辑用户"
                                      title="详情与配额管理"
                                      className="inline-flex h-7 w-7 items-center justify-center rounded-field text-fg-subtle hover:bg-bg-subtle hover:text-fg"
                                    >
                                      <MoreHorizontal size={14} />
                                    </button>
                                  </div>
                                </td>
                              </tr>
                            )
                          })}
                        </tbody>
                      </table>
                    </div>

                    <div className="flex items-center justify-between border-t border-surface-border px-4 py-2.5 text-xs text-fg-muted">
                      <span className="font-mono tabular-nums">
                        共 {formatCount(totalUsers)} 条 · 第 {page} / {totalPages} 页
                      </span>
                      <div className="flex items-center gap-1.5">
                        <Button
                          variant="outline"
                          disabled={page <= 1}
                          onClick={() => setPage((p) => Math.max(1, p - 1))}
                          className="h-7 px-2 text-xs"
                        >
                          <ChevronLeft size={13} />
                          上一页
                        </Button>
                        <Button
                          variant="outline"
                          disabled={page >= totalPages}
                          onClick={() => setPage((p) => Math.min(totalPages, p + 1))}
                          className="h-7 px-2 text-xs"
                        >
                          下一页
                          <ChevronRight size={13} />
                        </Button>
                      </div>
                    </div>
                  </>
                )}
              </section>
            </div>
          ) : (
            <AdminSubView
              tab={tab}
              users={users}
              overview={overview}
              loading={loading}
              onRefresh={() => void load()}
            />
          )}
        </main>
      )}

      <UserDrawer
        user={activeUser}
        onClose={() => setActiveUserId(null)}
        onSave={handleSaveUser}
        onRotateToken={handleRotateToken}
        onDelete={handleDeleteUser}
      />

      <CreateUserDialog
        open={createOpen}
        onClose={() => setCreateOpen(false)}
        onCreate={handleCreate}
      />
    </div>
  )
}

import { Activity, Archive, Database, RefreshCw } from 'lucide-react'
import {
  formatCount,
  formatTokens,
  type AdminOverview,
  type AdminTab,
  type AdminUser,
} from '../../lib/admin-users'
import { Bar, Button } from '../../components/ui'

export function AdminSubView({
  tab,
  users,
  overview,
  loading,
  onRefresh,
}: {
  tab: Exclude<AdminTab, 'users'>
  users: AdminUser[]
  overview: AdminOverview | null
  loading: boolean
  onRefresh: () => void
}) {
  if (loading && !overview) {
    return (
      <div className="mt-5 rounded-card border border-surface-border bg-surface p-4">
        <div className="space-y-3">
          {[0, 1, 2, 3].map((row) => (
            <div key={row} className="h-9 rounded-field bg-bg-subtle animate-pulse" />
          ))}
        </div>
      </div>
    )
  }

  const connections = overview?.connections ?? []
  const trend = overview?.trend ?? []
  const archives = overview?.archives ?? []
  const health = overview?.health

  if (tab === 'connections') {
    if (connections.length === 0) {
      return (
        <div className="mt-5 flex flex-col items-center justify-center rounded-card border border-surface-border bg-surface px-4 py-14 text-center">
          <Activity size={22} className="text-fg-subtle" />
          <p className="mt-3 text-sm font-medium text-fg">当前暂无在线 WebSocket 控制连接</p>
          <p className="mt-1 text-xs text-fg-subtle">
            客户端建立控制连接并完成鉴权后，将实时出现在此列表。
          </p>
          <Button variant="outline" onClick={onRefresh} className="mt-4">
            <RefreshCw size={13} />
            刷新状态
          </Button>
        </div>
      )
    }

    return (
      <div className="mt-5 overflow-x-auto rounded-card border border-surface-border bg-surface">
        <table className="w-full min-w-[600px] text-left text-sm">
          <thead className="border-b border-surface-border bg-surface-sunken text-xs text-fg-muted">
            <tr>
              <th className="px-4 py-2.5 font-normal">连接 ID</th>
              <th className="px-4 py-2.5 font-normal">用户</th>
              <th className="px-4 py-2.5 font-normal">会话标识</th>
              <th className="px-4 py-2.5 font-normal">来源地址</th>
              <th className="px-4 py-2.5 font-normal">在线时长</th>
              <th className="px-4 py-2.5 text-right font-normal">流状态</th>
            </tr>
          </thead>
          <tbody>
            {connections.map((c) => (
              <tr key={c.id} className="border-t border-surface-border hover:bg-bg-subtle/40">
                <td className="px-4 py-3 font-mono text-xs text-fg-subtle">{c.id}</td>
                <td className="px-4 py-3 text-fg">{c.userName}</td>
                <td className="px-4 py-3 font-mono text-xs text-fg-muted">{c.convId}</td>
                <td className="px-4 py-3 font-mono text-xs text-fg-muted tabular-nums">{c.ip}</td>
                <td className="px-4 py-3 font-mono text-xs text-fg-muted tabular-nums">{c.uptime}</td>
                <td className="px-4 py-3 text-right text-xs">
                  <span className={c.inflight ? 'text-brand' : 'text-fg-subtle'}>
                    {c.inflight ? '传输中' : '空闲'}
                  </span>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    )
  }

  if (tab === 'metering') {
    const totalUsed = users.reduce((s, u) => s + u.usedTokens, 0)
    const totalQuota = users.reduce((s, u) => s + u.monthlyTokens, 0)
    return (
      <div className="mt-5 grid grid-cols-1 gap-3 lg:grid-cols-2">
        <div className="rounded-card border border-surface-border bg-surface p-4">
          <h3 className="text-sm font-medium text-fg">每日用量明细</h3>
          {trend.length === 0 ? (
            <div className="py-10 text-center text-xs text-fg-subtle">暂无每日账本汇总记录</div>
          ) : (
            <div className="mt-3 divide-y divide-surface-border">
              {trend.map((d) => (
                <div key={d.day} className="flex items-center justify-between py-2 text-xs">
                  <span className="font-mono text-fg-muted tabular-nums">{d.day}</span>
                  <span className="font-mono text-fg tabular-nums">{formatCount(d.requests)} 次</span>
                  <span className="font-mono text-ok tabular-nums">命中 {d.hitRate}%</span>
                  <span className="font-mono text-fg-muted tabular-nums">{formatCount(d.tokensK)}K tok</span>
                </div>
              ))}
            </div>
          )}
        </div>

        <div className="rounded-card border border-surface-border bg-surface p-4">
          <div className="flex items-baseline justify-between">
            <h3 className="text-sm font-medium text-fg">全局配额水线</h3>
            <span className="font-mono text-xs text-fg-muted tabular-nums">
              {formatTokens(totalUsed)} / {formatTokens(totalQuota)}
            </span>
          </div>
          <div className="mt-3">
            <Bar value={totalUsed} cap={totalQuota || 1} />
          </div>
          <div className="mt-4 space-y-2.5">
            {[...users]
              .sort((a, b) => b.usedTokens - a.usedTokens)
              .slice(0, 5)
              .map((u) => (
                <div key={u.id} className="flex items-center gap-3 text-xs">
                  <span className="w-16 truncate text-fg">{u.name}</span>
                  <div className="flex-1">
                    <Bar value={u.usedTokens} cap={u.monthlyTokens || Math.max(1, u.usedTokens)} />
                  </div>
                  <span className="w-20 text-right font-mono text-fg-muted tabular-nums">
                    {formatTokens(u.usedTokens)}
                  </span>
                </div>
              ))}
          </div>
        </div>
      </div>
    )
  }

  if (tab === 'archives') {
    if (archives.length === 0) {
      return (
        <div className="mt-5 flex flex-col items-center justify-center rounded-card border border-surface-border bg-surface px-4 py-14 text-center">
          <Archive size={22} className="text-fg-subtle" />
          <p className="mt-3 text-sm font-medium text-fg">当前暂无会话存档记录</p>
          <p className="mt-1 text-xs text-fg-subtle">
            用户产生对话消息并写入 SQLite msgs 表后，将在此展示统计摘要。
          </p>
          <Button variant="outline" onClick={onRefresh} className="mt-4">
            <RefreshCw size={13} />
            刷新存档
          </Button>
        </div>
      )
    }

    return (
      <div className="mt-5 overflow-x-auto rounded-card border border-surface-border bg-surface">
        <table className="w-full min-w-[600px] text-left text-sm">
          <thead className="border-b border-surface-border bg-surface-sunken text-xs text-fg-muted">
            <tr>
              <th className="px-4 py-2.5 font-normal">会话 ID</th>
              <th className="px-4 py-2.5 font-normal">归属人</th>
              <th className="px-4 py-2.5 font-normal">分组</th>
              <th className="px-4 py-2.5 text-right font-normal">消息条数</th>
              <th className="px-4 py-2.5 text-right font-normal">锚点版本</th>
              <th className="px-4 py-2.5 text-right font-normal">体积</th>
              <th className="px-4 py-2.5 text-right font-normal">更新</th>
            </tr>
          </thead>
          <tbody>
            {archives.map((a) => (
              <tr key={a.convId} className="border-t border-surface-border hover:bg-bg-subtle/40">
                <td className="px-4 py-3 font-mono text-xs text-fg-subtle">{a.convId}</td>
                <td className="px-4 py-3 text-fg">{a.ownerName}</td>
                <td className="px-4 py-3 text-fg-muted">{a.group}</td>
                <td className="px-4 py-3 text-right font-mono text-xs text-fg tabular-nums">{a.messages}</td>
                <td className="px-4 py-3 text-right font-mono text-xs text-fg-muted tabular-nums">v{a.prefixVersion}</td>
                <td className="px-4 py-3 text-right font-mono text-xs text-fg-muted tabular-nums">{a.bytesKB} KB</td>
                <td className="px-4 py-3 text-right text-xs text-fg-subtle tabular-nums">{a.updatedAt}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    )
  }

  const checks = health?.checks ?? []
  return (
    <div className="mt-5 space-y-3">
      <div className="flex flex-wrap items-center justify-between gap-2 rounded-card border border-surface-border bg-surface px-4 py-3 text-xs">
        <div className="flex items-center gap-2 text-fg-muted">
          <Database size={14} className="text-brand" />
          <span className="font-mono tabular-nums">
            模式 {health?.dbMode ?? 'unknown'} · schema v{health?.schemaVer ?? 0} · 活跃连接 {health?.activeConns ?? 0}/{formatCount(health?.maxConns ?? 5000)}
          </span>
        </div>
        <Button variant="outline" onClick={onRefresh} className="h-7 px-2.5 text-xs">
          <RefreshCw size={12} />
          重新探测
        </Button>
      </div>

      <div className="grid grid-cols-1 gap-3 md:grid-cols-2">
        {checks.map((item) => (
          <div
            key={item.name}
            className="flex items-center justify-between gap-3 rounded-card border border-surface-border bg-surface p-4"
          >
            <div className="min-w-0">
              <div className="text-sm font-medium text-fg">{item.name}</div>
              <div className="mt-1 truncate font-mono text-xs text-fg-subtle">{item.detail}</div>
            </div>
            <span
              className={
                'inline-flex shrink-0 items-center gap-1.5 text-xs font-medium ' +
                (item.ok ? 'text-ok' : 'text-danger')
              }
            >
              <span className={'h-1.5 w-1.5 rounded-full ' + (item.ok ? 'bg-ok' : 'bg-danger')} />
              {item.ok ? '正常' : '异常'}
            </span>
          </div>
        ))}
      </div>
    </div>
  )
}

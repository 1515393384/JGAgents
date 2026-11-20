import {
  activeConnections,
  archiveSummaries,
  formatCount,
  formatTokens,
  weeklyTrend,
  type AdminTab,
  type AdminUser,
} from '../../lib/admin-users'
import { Bar } from '../../components/ui'

export function AdminSubView({ tab, users }: { tab: Exclude<AdminTab, 'users'>; users: AdminUser[] }) {
  if (tab === 'connections') {
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
            {activeConnections.map((c) => (
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
          <div className="mt-3 divide-y divide-surface-border">
            {weeklyTrend.map((d) => (
              <div key={d.day} className="flex items-center justify-between py-2 text-xs">
                <span className="font-mono text-fg-muted tabular-nums">{d.day}</span>
                <span className="font-mono text-fg tabular-nums">{formatCount(d.requests)} 次</span>
                <span className="font-mono text-ok tabular-nums">命中 {d.hitRate}%</span>
                <span className="font-mono text-fg-muted tabular-nums">{formatCount(d.tokensK)}K tok</span>
              </div>
            ))}
          </div>
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
                  <span className="w-14 truncate text-fg">{u.name}</span>
                  <div className="flex-1">
                    <Bar value={u.usedTokens} cap={u.monthlyTokens || 1} />
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
            {archiveSummaries.map((a) => (
              <tr key={a.convId} className="border-t border-surface-border hover:bg-bg-subtle/40">
                <td className="px-4 py-3 font-mono text-xs text-fg-subtle">{a.convId}</td>
                <td className="px-4 py-3 text-fg">{a.ownerName}</td>
                <td className="px-4 py-3 text-fg-muted">{a.group}</td>
                <td className="px-4 py-3 text-right font-mono text-xs text-fg tabular-nums">{a.messages}</td>
                <td className="px-4 py-3 text-right font-mono text-xs text-fg-muted tabular-nums">v{a.prefixVersion}</td>
                <td className="px-4 py-3 text-right font-mono text-xs text-fg-muted tabular-nums">{a.bytesKB} KB</td>
                <td className="px-4 py-3 text-right text-xs text-fg-subtle">{a.updatedAt}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    )
  }

  const checks = [
    { name: 'HTTP /healthz 探针', detail: 'GET /healthz -> {"ok":true,"ver":1}', status: '正常' },
    { name: 'SQLite WAL 写通道', detail: '单写线程 · busy_timeout 5000ms', status: '正常' },
    { name: '前缀锚点版本', detail: 'prefix_version = 3 · 无迁移阻塞', status: '正常' },
    { name: '管理员引导与隔离检查', detail: '用户分区校验已启用', status: '正常' },
  ]

  return (
    <div className="mt-5 grid grid-cols-1 gap-3 md:grid-cols-2">
      {checks.map((item) => (
        <div
          key={item.name}
          className="flex items-center justify-between rounded-card border border-surface-border bg-surface p-4"
        >
          <div>
            <div className="text-sm font-medium text-fg">{item.name}</div>
            <div className="mt-1 font-mono text-xs text-fg-subtle">{item.detail}</div>
          </div>
          <span className="inline-flex items-center gap-1.5 text-xs font-medium text-ok">
            <span className="h-1.5 w-1.5 rounded-full bg-ok" />
            {item.status}
          </span>
        </div>
      ))}
    </div>
  )
}

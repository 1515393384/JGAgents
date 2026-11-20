import { useMemo, useState } from 'react'
import {
  calcHitRatePct,
  formatMonthDayMs,
  formatTokens,
  type AdminUser,
  type DailyTrendPoint,
} from '../../lib/admin-users'

type GroupStat = {
  group: string
  members: number
  active: number
  used: number
  quota: number
  pct: number
}

export function UserCharts({
  users,
  trend,
  selectedGroup,
  onSelectGroup,
}: {
  users: AdminUser[]
  trend: DailyTrendPoint[]
  selectedGroup: string
  onSelectGroup: (group: string) => void
}) {
  const [hoverIdx, setHoverIdx] = useState<number | null>(null)

  const groupStats = useMemo<GroupStat[]>(() => {
    const map = new Map<string, { members: number; active: number; used: number; quota: number }>()
    for (const u of users) {
      const cur = map.get(u.group) ?? { members: 0, active: 0, used: 0, quota: 0 }
      cur.members += 1
      if (u.state === 'active') cur.active += 1
      cur.used += u.usedTokens
      cur.quota += u.monthlyTokens
      map.set(u.group, cur)
    }
    return Array.from(map.entries())
      .map(([group, s]) => ({
        group,
        ...s,
        pct: s.quota > 0 ? Math.min(100, Math.round((s.used / s.quota) * 100)) : 0,
      }))
      .sort((a, b) => b.used - a.used)
  }, [users])

  const W = 440
  const H = 148
  const PAD_X = 28
  const PAD_Y = 18
  const innerW = W - PAD_X * 2
  const innerH = H - PAD_Y * 2

  const enriched = useMemo(
    () =>
      trend.map((d) => ({
        ...d,
        day: formatMonthDayMs(d.dayMs),
        hitRate: calcHitRatePct(d),
      })),
    [trend],
  )

  const maxReq = Math.max(...enriched.map((d) => d.requests), 10)
  const reqPoints = enriched.map((d, i) => {
    const x = PAD_X + (i / Math.max(1, enriched.length - 1)) * innerW
    const y = PAD_Y + innerH - (d.requests / maxReq) * innerH
    return { x, y, ...d }
  })
  const hitPoints = enriched.map((d, i) => {
    const x = PAD_X + (i / Math.max(1, enriched.length - 1)) * innerW
    const y = PAD_Y + innerH - (d.hitRate / 100) * innerH
    return { x, y }
  })

  const reqLine = reqPoints
    .map((p, i) => `${i === 0 ? 'M' : 'L'}${p.x.toFixed(1)},${p.y.toFixed(1)}`)
    .join(' ')
  const reqArea =
    reqPoints.length > 0
      ? `${reqLine} L${reqPoints[reqPoints.length - 1]?.x.toFixed(1) ?? PAD_X},${PAD_Y + innerH} L${PAD_X},${PAD_Y + innerH} Z`
      : ''
  const hitLine = hitPoints
    .map((p, i) => `${i === 0 ? 'M' : 'L'}${p.x.toFixed(1)},${p.y.toFixed(1)}`)
    .join(' ')

  const activePoint = hoverIdx !== null ? reqPoints[hoverIdx] : reqPoints[reqPoints.length - 1]
  const totalPrompt = trend.reduce((acc, d) => acc + d.promptTokens, 0)
  const totalCached = trend.reduce((acc, d) => acc + d.cachedTokens, 0)
  const avgHit =
    totalPrompt > 0 ? Math.min(100, Math.round((totalCached / totalPrompt) * 1000) / 10) : 0

  return (
    <div className="grid grid-cols-1 gap-3 lg:grid-cols-12">
      <div className="rounded-card border border-surface-border bg-surface p-4 lg:col-span-5">
        <div className="flex items-baseline justify-between gap-2">
          <h2 className="text-sm font-medium text-fg">分组配额消耗</h2>
          {selectedGroup !== 'all' ? (
            <button
              type="button"
              onClick={() => onSelectGroup('all')}
              className="text-xs text-brand hover:underline whitespace-nowrap shrink-0"
            >
              重置筛选 ({selectedGroup})
            </button>
          ) : (
            <span className="text-xs text-fg-subtle whitespace-nowrap">点击分组条联动筛选</span>
          )}
        </div>

        <div className="mt-4 space-y-3">
          {groupStats.map((g) => {
            const isSelected = selectedGroup === g.group
            const toneClass = g.pct >= 85 ? 'bg-warn' : 'bg-brand'
            return (
              <button
                key={g.group}
                type="button"
                onClick={() => onSelectGroup(isSelected ? 'all' : g.group)}
                className={
                  'block w-full rounded-field px-2 py-1.5 text-left transition-colors ' +
                  (isSelected ? 'bg-bg-subtle ring-1 ring-brand/40' : 'hover:bg-bg-subtle/60')
                }
              >
                <div className="flex items-baseline justify-between text-xs">
                  <span className="flex items-center gap-2">
                    <span className="font-medium text-fg">{g.group}</span>
                    <span className="text-fg-subtle tabular-nums">
                      {g.active}/{g.members} 人启用
                    </span>
                  </span>
                  <span className="font-mono text-fg-muted tabular-nums">
                    {formatTokens(g.used)} / {formatTokens(g.quota)} ({g.pct}%)
                  </span>
                </div>
                <div className="mt-1.5 h-1.5 w-full overflow-hidden rounded-pill bg-bg-subtle">
                  <div
                    className={`h-full rounded-pill transition-[width] duration-300 ${toneClass}`}
                    style={{ width: `${g.pct}%` }}
                  />
                </div>
              </button>
            )
          })}
          {groupStats.length === 0 ? (
            <div className="py-6 text-center text-xs text-fg-subtle">暂无分组用量数据</div>
          ) : null}
        </div>
      </div>

      <div className="rounded-card border border-surface-border bg-surface p-4 lg:col-span-7">
        <div className="flex flex-wrap items-baseline justify-between gap-2">
          <div className="flex items-baseline gap-3">
            <h2 className="text-sm font-medium text-fg">7 日请求与前缀命中</h2>
            <div className="flex items-center gap-3 text-xs text-fg-subtle">
              <span className="inline-flex items-center gap-1.5">
                <span className="h-2 w-2 rounded-pill bg-brand" />
                请求量
              </span>
              <span className="inline-flex items-center gap-1.5">
                <span className="h-2 w-2 rounded-pill bg-ok" />
                加权命中率（{avgHit}%）
              </span>
            </div>
          </div>

          {activePoint ? (
            <div className="font-mono text-xs text-fg-muted tabular-nums">
              {activePoint.day} · {activePoint.requests} 次 · 命中 {activePoint.hitRate}%
            </div>
          ) : null}
        </div>

        <div className="mt-2">
          {enriched.length === 0 ? (
            <div className="flex h-[156px] items-center justify-center text-xs text-fg-subtle">
              暂无 7 日用量入账记录
            </div>
          ) : (
            <svg
              viewBox={`0 0 ${W} ${H}`}
              className="h-[156px] w-full overflow-visible select-none"
              onMouseLeave={() => setHoverIdx(null)}
            >
              <defs>
                <linearGradient id="reqAreaGrad" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="0%" stopColor="var(--brand)" stopOpacity="0.22" />
                  <stop offset="100%" stopColor="var(--brand)" stopOpacity="0" />
                </linearGradient>
              </defs>

              {[0.25, 0.5, 0.75, 1].map((ratio) => {
                const y = PAD_Y + innerH * (1 - ratio)
                return (
                  <line
                    key={ratio}
                    x1={PAD_X}
                    x2={W - PAD_X}
                    y1={y}
                    y2={y}
                    stroke="var(--border)"
                    strokeDasharray="3 3"
                    strokeWidth="1"
                  />
                )
              })}

              <path d={reqArea} fill="url(#reqAreaGrad)" />
              <path
                d={reqLine}
                fill="none"
                stroke="var(--brand)"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
              />
              <path
                d={hitLine}
                fill="none"
                stroke="var(--ok)"
                strokeWidth="1.75"
                strokeDasharray="4 3"
                strokeLinecap="round"
                strokeLinejoin="round"
              />

              {reqPoints.map((p, i) => {
                const hp = hitPoints[i]
                const isHover = hoverIdx === i
                return (
                  <g key={`${p.dayMs}-${i}`}>
                    {isHover ? (
                      <line
                        x1={p.x}
                        x2={p.x}
                        y1={PAD_Y}
                        y2={PAD_Y + innerH}
                        stroke="var(--border-strong)"
                        strokeWidth="1"
                      />
                    ) : null}
                    <circle
                      cx={p.x}
                      cy={p.y}
                      r={isHover ? 4 : 2.5}
                      fill="var(--surface)"
                      stroke="var(--brand)"
                      strokeWidth="2"
                    />
                    {hp ? (
                      <circle cx={hp.x} cy={hp.y} r={isHover ? 3.5 : 2} fill="var(--ok)" />
                    ) : null}
                    <text
                      x={p.x}
                      y={H - 2}
                      textAnchor="middle"
                      className="fill-[var(--fg-subtle)] font-mono text-[10px]"
                    >
                      {p.day}
                    </text>
                    <rect
                      x={p.x - innerW / (enriched.length * 2)}
                      y={0}
                      width={innerW / enriched.length}
                      height={H}
                      fill="transparent"
                      onMouseEnter={() => setHoverIdx(i)}
                    />
                  </g>
                )
              })}
            </svg>
          )}
        </div>
      </div>
    </div>
  )
}

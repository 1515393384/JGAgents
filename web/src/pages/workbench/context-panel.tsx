import { Bar, Badge, Card } from '../../components/ui'
import { ctxLayers, ctxSummary } from '../../lib/mock'

export function ContextPanel() {
  const pct = Math.round((ctxSummary.used / ctxSummary.budget) * 100)

  return (
    <aside className="hidden w-80 shrink-0 flex-col border-l border-border xl:flex">
      <div className="border-b border-border px-4 py-2.5 text-sm font-medium">上下文</div>

      <div className="min-h-0 flex-1 space-y-4 overflow-y-auto p-4">
        <Card className="p-3">
          <div className="flex items-baseline justify-between">
            <span className="text-xs text-fg-muted">预算占用</span>
            <span className="font-mono text-xs">
              {ctxSummary.used} / {ctxSummary.budget}
            </span>
          </div>
          <div className="mt-2">
            <Bar value={ctxSummary.used} cap={ctxSummary.budget} tone={pct > 85 ? 'warn' : 'brand'} />
          </div>
          <div className="mt-2 flex items-center gap-1.5">
            <Badge tone="ok">命中 {Math.round(ctxSummary.hitRate * 100)}%</Badge>
            <Badge>缓存 {ctxSummary.cachedTokens.toLocaleString()}</Badge>
          </div>
        </Card>

        <div>
          <div className="mb-2 text-xs text-fg-muted">分层</div>
          <div className="space-y-3">
            {ctxLayers.map((l) => (
              <div key={l.key}>
                <div className="mb-1 flex items-baseline justify-between text-xs">
                  <span>{l.label}</span>
                  <span className="font-mono text-fg-subtle">{l.used}</span>
                </div>
                <Bar value={l.used} cap={l.cap} />
              </div>
            ))}
          </div>
        </div>

        <div className="rounded-field border border-dashed border-border p-3 text-xs leading-relaxed text-fg-subtle">
          服务端算，客户端只显示。这里以后会随每一帧 <span className="font-mono">Ctx</span> 命令刷新。
        </div>
      </div>
    </aside>
  )
}

import { useEffect, useState } from 'react'
import { X } from 'lucide-react'
import { Avatar, Bar } from '../../components/ui'
import { quotas, type Space } from '../../lib/mock'

const tabs = ['成员', '配额', '审计'] as const
type Tab = (typeof tabs)[number]

const compact = (n: number) =>
  n >= 1_000_000 ? `${(n / 1_000_000).toFixed(2)}M` : n >= 1000 ? `${Math.round(n / 1000)}K` : String(n)

/** 空间设置：从左边空间菜单进来，右侧滑出。
 *  个人空间只有"成员"一块——配额和审计是团队才有的事。 */
export function SpaceSettings({
  space,
  open,
  onClose,
}: {
  space: Space
  open: boolean
  onClose: () => void
}) {
  const q = quotas[space.id]
  const team = space.kind === 'team'
  const [tab, setTab] = useState<Tab>('成员')

  useEffect(() => {
    if (!open) return
    setTab('成员')
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose()
    }
    document.addEventListener('keydown', onKey)
    return () => document.removeEventListener('keydown', onKey)
  }, [open, onClose])

  return (
    <>
      <div
        onClick={onClose}
        aria-hidden
        className={
          'fixed inset-0 z-40 bg-black/30 transition-opacity duration-200 ' +
          (open ? 'opacity-100' : 'pointer-events-none opacity-0')
        }
      />
      <aside
        className={
          'fixed inset-y-0 right-0 z-50 flex w-[380px] max-w-[92vw] flex-col border-l border-surface-border bg-surface ' +
          'transition-transform duration-200 ' +
          (open ? 'translate-x-0' : 'translate-x-full')
        }
      >
        <div className="flex h-14 shrink-0 items-center gap-2 border-b border-surface-border px-4">
          <span className="text-[13.5px] font-medium">{space.name}</span>
          <span className="text-[11px] text-fg-subtle">{space.kind === 'personal' ? '个人空间' : '团队空间'}</span>
          <button
            type="button"
            onClick={onClose}
            aria-label="关闭"
            className="ml-auto inline-flex h-8 w-8 items-center justify-center rounded-field text-fg-subtle hover:bg-bg-subtle hover:text-fg"
          >
            <X size={15} />
          </button>
        </div>

        <div className="flex shrink-0 items-center gap-1 border-b border-surface-border px-3">
          {tabs
            .filter((t) => team || t === '成员')
            .map((t) => (
              <button
                key={t}
                type="button"
                onClick={() => setTab(t)}
                className={
                  '-mb-px border-b-2 px-2.5 py-2 text-[12.5px] transition-colors ' +
                  (tab === t
                    ? 'border-brand text-fg'
                    : 'border-transparent text-fg-muted hover:text-fg')
                }
              >
                {t}
              </button>
            ))}
        </div>

        <div className="min-h-0 flex-1 overflow-y-auto p-3.5">
          {tab === '成员' ? (
            <div className="space-y-0.5">
              {(q?.members ?? []).map((m) => (
                <div key={m.name} className="flex items-center gap-2.5 rounded-field px-1.5 py-2">
                  <Avatar name={m.name} size={28} tone={m.role === '管理员' ? 'brand' : 'muted'} />
                  <span className="min-w-0 flex-1">
                    <span className="block truncate text-[13px]">{m.name}</span>
                    <span className="block text-[11px] text-fg-subtle">{m.role}</span>
                  </span>
                  {team ? (
                    <span className="shrink-0 font-mono text-[11px] text-fg-subtle">
                      {compact(m.used)}
                    </span>
                  ) : null}
                </div>
              ))}
              {team ? (
                <button
                  type="button"
                  className="mt-1 w-full rounded-field border border-dashed border-surface-border py-2 text-[12.5px] text-fg-subtle hover:text-fg"
                >
                  邀请成员
                </button>
              ) : (
                <p className="px-1.5 pt-2 text-[11.5px] leading-6 text-fg-subtle">
                  个人空间只有你。文件不进任何团队，配额随你的账号走。
                </p>
              )}
            </div>
          ) : null}

          {tab === '配额' && q ? (
            <div className="space-y-4">
              <div className="rounded-field border border-surface-border bg-surface-sunken p-3">
                <div className="flex items-baseline justify-between text-[12px]">
                  <span className="text-fg-muted">{q.month} token 用量</span>
                  <span className="font-mono">
                    {compact(q.tokens)} / {compact(q.cap)}
                  </span>
                </div>
                <div className="mt-2">
                  <Bar value={q.tokens} cap={q.cap} tone={q.tokens / q.cap > 0.85 ? 'warn' : 'brand'} />
                </div>
                <div className="mt-2 text-[11px] text-fg-subtle">
                  前缀命中 {Math.round(q.hitRate * 100)}% · 超出后按成员上限逐个停
                </div>
              </div>

              <div>
                <div className="mb-1.5 text-[11px] text-fg-subtle">每人用量</div>
                <div className="space-y-2.5">
                  {q.members.map((m) => (
                    <div key={m.name}>
                      <div className="mb-1 flex items-baseline justify-between text-[12px]">
                        <span>{m.name}</span>
                        <span className="font-mono text-[11px] text-fg-subtle">
                          {compact(m.used)} / {compact(m.cap)}
                        </span>
                      </div>
                      <Bar value={m.used} cap={m.cap} />
                    </div>
                  ))}
                </div>
              </div>
            </div>
          ) : null}

          {tab === '审计' && q ? (
            <div className="space-y-0.5">
              {q.audit.map((a, i) => (
                <div key={i} className="flex gap-2.5 rounded-field px-1.5 py-2 text-[12.5px]">
                  <span className="w-10 shrink-0 font-mono text-[11px] text-fg-subtle">{a.at}</span>
                  <span className="min-w-0">
                    <span className="text-fg">{a.who}</span>
                    <span className="ml-1.5 text-fg-muted">{a.what}</span>
                  </span>
                </div>
              ))}
            </div>
          ) : null}
        </div>
      </aside>
    </>
  )
}

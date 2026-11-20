import { useEffect, useState } from 'react'
import { Check, Copy, ShieldAlert, X } from 'lucide-react'
import { Button } from '../../components/ui'
import type { CreateUserPayload, UserRole } from '../../lib/admin-users'

const GROUP_PRESETS = ['研发', '售前', '产品', '交付', '运营']
const QUOTA_PRESETS = [
  { label: '1M', value: 1_000_000 },
  { label: '2M', value: 2_000_000 },
  { label: '4M', value: 4_000_000 },
  { label: '不限', value: 0 },
]

export function CreateUserDialog({
  open,
  onClose,
  onCreate,
}: {
  open: boolean
  onClose: () => void
  onCreate: (payload: CreateUserPayload) => Promise<{ token: string; name: string }>
}) {
  const [name, setName] = useState('')
  const [group, setGroup] = useState('研发')
  const [role, setRole] = useState<UserRole>('member')
  const [monthlyTokens, setMonthlyTokens] = useState(2_000_000)
  const [dailyRequests, setDailyRequests] = useState(500)
  const [submitting, setSubmitting] = useState(false)
  const [error, setError] = useState('')
  const [issuedToken, setIssuedToken] = useState('')
  const [issuedName, setIssuedName] = useState('')
  const [copied, setCopied] = useState(false)

  useEffect(() => {
    if (!open) return
    setName('')
    setGroup('研发')
    setRole('member')
    setMonthlyTokens(2_000_000)
    setDailyRequests(500)
    setError('')
    setIssuedToken('')
    setIssuedName('')
    setCopied(false)

    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose()
    }
    document.addEventListener('keydown', onKey)
    return () => document.removeEventListener('keydown', onKey)
  }, [open, onClose])

  if (!open) return null

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    setSubmitting(true)
    setError('')
    try {
      const res = await onCreate({ name, group, role, dailyRequests, monthlyTokens })
      setIssuedToken(res.token)
      setIssuedName(res.name)
    } catch (err) {
      setError(err instanceof Error ? err.message : '创建用户失败')
    } finally {
      setSubmitting(false)
    }
  }

  const handleCopy = async () => {
    if (!issuedToken) return
    await navigator.clipboard.writeText(issuedToken)
    setCopied(true)
    window.setTimeout(() => setCopied(false), 1800)
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
      <div onClick={onClose} aria-hidden className="fixed inset-0 bg-black/45 backdrop-blur-[2px]" />

      <div className="relative z-10 w-full max-w-[460px] overflow-hidden rounded-panel border border-surface-border bg-surface shadow-float">
        <div className="flex h-13 items-center justify-between border-b border-surface-border px-5">
          <h2 className="text-sm font-medium text-fg">
            {issuedToken ? '用户已创建' : '新建用户'}
          </h2>
          <button
            type="button"
            onClick={onClose}
            aria-label="关闭"
            className="inline-flex h-8 w-8 items-center justify-center rounded-field text-fg-subtle hover:bg-bg-subtle hover:text-fg"
          >
            <X size={15} />
          </button>
        </div>

        {issuedToken ? (
          <div className="space-y-4 p-5">
            <p className="text-xs leading-5 text-fg-muted">
              已为 <span className="font-medium text-fg">{issuedName}</span> 生成接入凭证。服务端仅保存哈希指纹，明文凭证仅在此处展示一次。
            </p>
            <div className="flex items-center gap-2 rounded-field border border-brand/35 bg-brand-soft p-2.5">
              <code className="min-w-0 flex-1 truncate font-mono text-xs text-fg">
                {issuedToken}
              </code>
              <Button variant="primary" onClick={handleCopy} className="h-7 px-2.5 text-xs shrink-0">
                {copied ? <Check size={12} /> : <Copy size={12} />}
                {copied ? '已复制' : '复制凭证'}
              </Button>
            </div>
            <div className="flex justify-end pt-1">
              <Button variant="outline" onClick={onClose}>
                完成
              </Button>
            </div>
          </div>
        ) : (
          <form onSubmit={handleSubmit} className="space-y-4 p-5">
            {error ? (
              <div className="flex items-start gap-2 rounded-field border border-danger/35 bg-danger/10 px-3 py-2 text-xs text-danger">
                <ShieldAlert size={14} className="mt-0.5 shrink-0" />
                <span>{error}</span>
              </div>
            ) : null}

            <div className="grid grid-cols-2 gap-3">
              <label className="block">
                <span className="text-xs text-fg-muted">姓名</span>
                <input
                  autoFocus
                  required
                  placeholder="例如：张衡"
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  className="mt-1.5 h-9 w-full rounded-field border border-border bg-surface-sunken px-2.5 text-sm text-fg placeholder:text-fg-subtle focus-visible:border-brand focus-visible:outline-none"
                />
              </label>

              <label className="block">
                <span className="text-xs text-fg-muted">所属分组</span>
                <input
                  required
                  list="admin-group-presets"
                  value={group}
                  onChange={(e) => setGroup(e.target.value)}
                  className="mt-1.5 h-9 w-full rounded-field border border-border bg-surface-sunken px-2.5 text-sm text-fg focus-visible:border-brand focus-visible:outline-none"
                />
                <datalist id="admin-group-presets">
                  {GROUP_PRESETS.map((g) => (
                    <option key={g} value={g} />
                  ))}
                </datalist>
              </label>
            </div>

            <div>
              <span className="text-xs text-fg-muted">角色</span>
              <div className="mt-1.5 grid grid-cols-2 gap-1 rounded-field bg-bg-subtle p-1">
                {(
                  [
                    { id: 'member', label: '普通成员' },
                    { id: 'admin', label: '管理员' },
                  ] as const
                ).map((item) => (
                  <button
                    key={item.id}
                    type="button"
                    onClick={() => setRole(item.id)}
                    className={
                      'h-8 rounded-[7px] text-xs font-medium transition-colors whitespace-nowrap ' +
                      (role === item.id
                        ? 'bg-surface text-fg shadow-card'
                        : 'text-fg-muted hover:text-fg')
                    }
                  >
                    {item.label}
                  </button>
                ))}
              </div>
            </div>

            <div>
              <div className="flex items-baseline justify-between">
                <span className="text-xs text-fg-muted">月度 Token 配额</span>
                <div className="flex items-center gap-1">
                  {QUOTA_PRESETS.map((q) => (
                    <button
                      key={q.label}
                      type="button"
                      onClick={() => setMonthlyTokens(q.value)}
                      className={
                        'rounded px-1.5 py-0.5 font-mono text-[11px] transition-colors ' +
                        (monthlyTokens === q.value
                          ? 'bg-brand-soft text-brand'
                          : 'text-fg-subtle hover:bg-bg-subtle hover:text-fg')
                      }
                    >
                      {q.label}
                    </button>
                  ))}
                </div>
              </div>
              <div className="mt-1.5 grid grid-cols-2 gap-3">
                <input
                  type="number"
                  min={0}
                  step={100000}
                  value={monthlyTokens}
                  onChange={(e) => setMonthlyTokens(Math.max(0, Number(e.target.value) || 0))}
                  className="h-9 w-full rounded-field border border-border bg-surface-sunken px-2.5 font-mono text-sm text-fg tabular-nums focus-visible:border-brand focus-visible:outline-none"
                />
                <input
                  type="number"
                  min={0}
                  step={50}
                  value={dailyRequests}
                  onChange={(e) => setDailyRequests(Math.max(0, Number(e.target.value) || 0))}
                  placeholder="日请求上限"
                  className="h-9 w-full rounded-field border border-border bg-surface-sunken px-2.5 font-mono text-sm text-fg tabular-nums focus-visible:border-brand focus-visible:outline-none"
                />
              </div>
              <div className="mt-1 flex justify-between text-[11px] text-fg-subtle">
                <span>左：月度 Token 上限（0 不限）</span>
                <span>右：日请求上限</span>
              </div>
            </div>

            <div className="flex items-center justify-end gap-2 pt-2">
              <Button variant="ghost" onClick={onClose}>
                取消
              </Button>
              <Button type="submit" variant="primary" disabled={submitting}>
                {submitting ? '创建中…' : '创建并签发凭证'}
              </Button>
            </div>
          </form>
        )}
      </div>
    </div>
  )
}

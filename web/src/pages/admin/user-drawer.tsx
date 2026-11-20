import { useEffect, useState } from 'react'
import { Check, Copy, KeyRound, ShieldAlert, Trash2, X } from 'lucide-react'
import { Avatar, Bar, Button } from '../../components/ui'
import {
  formatCount,
  formatTokens,
  type AdminUser,
  type UpdateUserPayload,
  type UserRole,
  type UserState,
} from '../../lib/admin-users'

export function UserDrawer({
  user,
  onClose,
  onSave,
  onRotateToken,
  onDelete,
}: {
  user: AdminUser | null
  onClose: () => void
  onSave: (id: string, payload: UpdateUserPayload) => Promise<void>
  onRotateToken: (id: string) => Promise<string>
  onDelete: (id: string) => Promise<void>
}) {
  const [name, setName] = useState('')
  const [group, setGroup] = useState('')
  const [role, setRole] = useState<UserRole>('member')
  const [state, setState] = useState<UserState>('active')
  const [dailyRequests, setDailyRequests] = useState(500)
  const [monthlyTokens, setMonthlyTokens] = useState(2_000_000)
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState('')
  const [newToken, setNewToken] = useState('')
  const [copied, setCopied] = useState(false)
  const [confirmDelete, setConfirmDelete] = useState(false)

  useEffect(() => {
    if (!user) return
    setName(user.name)
    setGroup(user.group)
    setRole(user.role)
    setState(user.state)
    setDailyRequests(user.dailyRequests)
    setMonthlyTokens(user.monthlyTokens)
    setError('')
    setNewToken('')
    setCopied(false)
    setConfirmDelete(false)

    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose()
    }
    document.addEventListener('keydown', onKey)
    return () => document.removeEventListener('keydown', onKey)
  }, [user, onClose])

  if (!user) return null

  const pct =
    monthlyTokens > 0 ? Math.min(100, Math.round((user.usedTokens / monthlyTokens) * 100)) : 0

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault()
    setSaving(true)
    setError('')
    try {
      await onSave(user.id, { name, group, role, state, dailyRequests, monthlyTokens })
      onClose()
    } catch (err) {
      setError(err instanceof Error ? err.message : '保存失败')
    } finally {
      setSaving(false)
    }
  }

  const handleRotate = async () => {
    setError('')
    try {
      const token = await onRotateToken(user.id)
      setNewToken(token)
      setCopied(false)
    } catch (err) {
      setError(err instanceof Error ? err.message : '重置凭证失败')
    }
  }

  const handleCopyToken = async () => {
    if (!newToken) return
    await navigator.clipboard.writeText(newToken)
    setCopied(true)
    window.setTimeout(() => setCopied(false), 1800)
  }

  const handleDelete = async () => {
    setError('')
    setSaving(true)
    try {
      await onDelete(user.id)
      onClose()
    } catch (err) {
      setError(err instanceof Error ? err.message : '删除失败')
      setConfirmDelete(false)
    } finally {
      setSaving(false)
    }
  }

  return (
    <>
      <div
        onClick={onClose}
        aria-hidden
        className="fixed inset-0 z-40 bg-black/35 backdrop-blur-[1px] transition-opacity"
      />

      <aside className="fixed inset-y-0 right-0 z-50 flex w-[420px] max-w-[94vw] flex-col border-l border-surface-border bg-surface shadow-float">
        <div className="flex h-14 shrink-0 items-center gap-3 border-b border-surface-border px-4">
          <Avatar name={user.name} size={30} tone={user.role === 'admin' ? 'brand' : 'muted'} />
          <div className="min-w-0 flex-1">
            <div className="truncate text-sm font-medium text-fg">{user.name}</div>
            <div className="font-mono text-[11px] text-fg-subtle tabular-nums">
              {user.id} · 创建于 {user.createdAt}
            </div>
          </div>
          <button
            type="button"
            onClick={onClose}
            aria-label="关闭详情"
            className="inline-flex h-8 w-8 items-center justify-center rounded-field text-fg-subtle hover:bg-bg-subtle hover:text-fg"
          >
            <X size={15} />
          </button>
        </div>

        <form onSubmit={handleSave} className="flex min-h-0 flex-1 flex-col">
          <div className="min-h-0 flex-1 space-y-5 overflow-y-auto p-4">
            {error ? (
              <div className="flex items-start gap-2 rounded-field border border-danger/35 bg-danger/10 px-3 py-2 text-xs text-danger">
                <ShieldAlert size={14} className="mt-0.5 shrink-0" />
                <span>{error}</span>
              </div>
            ) : null}

            <div className="rounded-field border border-surface-border bg-surface-sunken p-3">
              <div className="flex items-baseline justify-between text-xs">
                <span className="text-fg-muted">本月 Token 消耗</span>
                <span className="font-mono text-fg tabular-nums">
                  {formatTokens(user.usedTokens)} / {formatTokens(monthlyTokens)} ({pct}%)
                </span>
              </div>
              <div className="mt-2">
                <Bar value={user.usedTokens} cap={monthlyTokens || 1} tone={pct >= 85 ? 'warn' : 'brand'} />
              </div>
              <div className="mt-2 flex items-center justify-between text-[11px] text-fg-subtle tabular-nums">
                <span>今日请求 {formatCount(user.usedRequests)} / {dailyRequests === 0 ? '不限' : formatCount(dailyRequests)}</span>
                <span>最近活跃：{user.lastActiveAt}</span>
              </div>
            </div>

            <div className="grid grid-cols-2 gap-3">
              <label className="block">
                <span className="text-xs text-fg-muted">姓名</span>
                <input
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  className="mt-1.5 h-9 w-full rounded-field border border-border bg-surface-sunken px-2.5 text-sm text-fg focus-visible:border-brand focus-visible:outline-none"
                />
              </label>

              <label className="block">
                <span className="text-xs text-fg-muted">所属分组</span>
                <input
                  value={group}
                  onChange={(e) => setGroup(e.target.value)}
                  className="mt-1.5 h-9 w-full rounded-field border border-border bg-surface-sunken px-2.5 text-sm text-fg focus-visible:border-brand focus-visible:outline-none"
                />
              </label>
            </div>

            <div className="grid grid-cols-2 gap-3">
              <div>
                <span className="text-xs text-fg-muted">角色权限</span>
                <div className="mt-1.5 grid grid-cols-2 gap-1 rounded-field bg-bg-subtle p-1">
                  {(
                    [
                      { id: 'member', label: '成员' },
                      { id: 'admin', label: '管理员' },
                    ] as const
                  ).map((item) => (
                    <button
                      key={item.id}
                      type="button"
                      onClick={() => setRole(item.id)}
                      className={
                        'h-7 rounded-[7px] text-xs font-medium transition-colors whitespace-nowrap ' +
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
                <span className="text-xs text-fg-muted">账号状态</span>
                <div className="mt-1.5 grid grid-cols-2 gap-1 rounded-field bg-bg-subtle p-1">
                  {(
                    [
                      { id: 'active', label: '启用' },
                      { id: 'disabled', label: '停用' },
                    ] as const
                  ).map((item) => (
                    <button
                      key={item.id}
                      type="button"
                      onClick={() => setState(item.id)}
                      className={
                        'h-7 rounded-[7px] text-xs font-medium transition-colors whitespace-nowrap ' +
                        (state === item.id
                          ? item.id === 'active'
                            ? 'bg-surface text-ok shadow-card'
                            : 'bg-surface text-danger shadow-card'
                          : 'text-fg-muted hover:text-fg')
                      }
                    >
                      {item.label}
                    </button>
                  ))}
                </div>
              </div>
            </div>

            <div className="grid grid-cols-2 gap-3">
              <label className="block">
                <span className="text-xs text-fg-muted">月度 Token 上限</span>
                <input
                  type="number"
                  min={0}
                  step={100000}
                  value={monthlyTokens}
                  onChange={(e) => setMonthlyTokens(Math.max(0, Number(e.target.value) || 0))}
                  className="mt-1.5 h-9 w-full rounded-field border border-border bg-surface-sunken px-2.5 font-mono text-sm text-fg tabular-nums focus-visible:border-brand focus-visible:outline-none"
                />
                <span className="mt-1 block text-[11px] text-fg-subtle">0 表示不限额度</span>
              </label>

              <label className="block">
                <span className="text-xs text-fg-muted">日请求上限</span>
                <input
                  type="number"
                  min={0}
                  step={50}
                  value={dailyRequests}
                  onChange={(e) => setDailyRequests(Math.max(0, Number(e.target.value) || 0))}
                  className="mt-1.5 h-9 w-full rounded-field border border-border bg-surface-sunken px-2.5 font-mono text-sm text-fg tabular-nums focus-visible:border-brand focus-visible:outline-none"
                />
                <span className="mt-1 block text-[11px] text-fg-subtle">0 表示不限次数</span>
              </label>
            </div>

            <div className="border-t border-surface-border pt-4">
              <div className="flex items-center justify-between gap-2">
                <div>
                  <div className="text-xs font-medium text-fg">访问凭证</div>
                  <div className="mt-0.5 font-mono text-[11px] text-fg-subtle">
                    指纹前缀 {user.tokenFingerprint}····
                  </div>
                </div>
                <Button variant="outline" onClick={handleRotate} className="whitespace-nowrap shrink-0">
                  <KeyRound size={13} />
                  重置凭证
                </Button>
              </div>

              {newToken ? (
                <div className="mt-3 rounded-field border border-brand/35 bg-brand-soft p-2.5">
                  <div className="text-[11px] text-fg-muted">新凭证仅展示这一次，请立即复制保存：</div>
                  <div className="mt-1.5 flex items-center gap-2">
                    <code className="min-w-0 flex-1 truncate rounded bg-surface px-2 py-1 font-mono text-xs text-fg">
                      {newToken}
                    </code>
                    <Button variant="primary" onClick={handleCopyToken} className="h-7 px-2.5 text-xs shrink-0">
                      {copied ? <Check size={12} /> : <Copy size={12} />}
                      {copied ? '已复制' : '复制'}
                    </Button>
                  </div>
                </div>
              ) : null}
            </div>

            <div className="border-t border-surface-border pt-4">
              {confirmDelete ? (
                <div className="flex items-center justify-between gap-2 rounded-field border border-danger/35 bg-danger/10 p-2.5">
                  <span className="text-xs text-danger">确认删除「{user.name}」？历史会话归属将不可恢复。</span>
                  <div className="flex shrink-0 items-center gap-1.5">
                    <Button variant="ghost" onClick={() => setConfirmDelete(false)} className="h-7 px-2 text-xs">
                      取消
                    </Button>
                    <button
                      type="button"
                      onClick={handleDelete}
                      disabled={saving}
                      className="inline-flex h-7 items-center rounded-field bg-danger px-2.5 text-xs font-medium text-white hover:opacity-90 disabled:opacity-50"
                    >
                      确认删除
                    </button>
                  </div>
                </div>
              ) : (
                <button
                  type="button"
                  onClick={() => setConfirmDelete(true)}
                  className="inline-flex items-center gap-1.5 text-xs text-fg-subtle hover:text-danger"
                >
                  <Trash2 size={13} />
                  删除该用户
                </button>
              )}
            </div>
          </div>

          <div className="flex shrink-0 items-center justify-end gap-2 border-t border-surface-border bg-surface-sunken px-4 py-3">
            <Button variant="ghost" onClick={onClose}>
              取消
            </Button>
            <Button type="submit" variant="primary" disabled={saving}>
              {saving ? '保存中…' : '保存更改'}
            </Button>
          </div>
        </form>
      </aside>
    </>
  )
}

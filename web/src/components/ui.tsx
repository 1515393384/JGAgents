import type { ButtonHTMLAttributes, HTMLAttributes, ReactNode } from 'react'

type Tone = 'brand' | 'ok' | 'warn' | 'danger' | 'muted'

const toneText: Record<Tone, string> = {
  brand: 'text-brand',
  ok: 'text-ok',
  warn: 'text-warn',
  danger: 'text-danger',
  muted: 'text-fg-muted',
}

const toneBg: Record<Tone, string> = {
  brand: 'bg-brand',
  ok: 'bg-ok',
  warn: 'bg-warn',
  danger: 'bg-danger',
  muted: 'bg-fg-subtle',
}

function cx(...parts: (string | false | null | undefined)[]) {
  return parts.filter(Boolean).join(' ')
}

export function Button({
  variant = 'ghost',
  pill = false,
  size = 'md',
  className,
  ...rest
}: ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: 'primary' | 'ghost' | 'outline'
  /** 全圆角胶囊：落地页/登录页用，工具栏用方角 */
  pill?: boolean
  size?: 'md' | 'lg'
}) {
  const base = `inline-flex items-center justify-center gap-2 font-medium tracking-tight transition-colors ${
    pill ? 'rounded-pill' : 'rounded-field font-normal'
  } ${size === 'lg' ? 'px-5 py-3 text-sm' : 'h-9 px-3 text-sm'}`
  const styles = {
    primary: 'bg-brand text-brand-fg hover:bg-brand-hover',
    outline: 'border border-border bg-bg-subtle text-fg hover:bg-bg-accent',
    ghost: 'text-fg-muted hover:bg-bg-subtle hover:text-fg',
  }[variant]
  return <button type="button" className={cx(base, styles, className)} {...rest} />
}

export function Card({ className, children, ...rest }: HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      className={cx('rounded-card border border-surface-border bg-surface shadow-card', className)}
      {...rest}
    >
      {children}
    </div>
  )
}

/** 首字头像。企业里认人不认图，首字比随机头像好认 */
export function Avatar({
  name,
  size = 26,
  tone = 'brand',
  ring = false,
}: {
  name: string
  size?: number
  tone?: 'brand' | 'muted'
  /** 叠在别人上面时加一圈背景色描边，才不会糊成一团 */
  ring?: boolean
}) {
  return (
    <span
      style={{ width: size, height: size, fontSize: Math.round(size * 0.42) }}
      className={cx(
        'inline-flex shrink-0 select-none items-center justify-center rounded-pill font-medium',
        tone === 'brand' ? 'bg-brand-soft text-brand' : 'bg-bg-subtle text-fg-muted',
        ring && 'ring-2 ring-bg',
      )}
      title={name}
    >
      {name.slice(0, 1)}
    </span>
  )
}

export function Badge({ tone = 'muted', children }: { tone?: Tone; children: ReactNode }) {
  return (
    <span
      className={cx(
        'inline-flex items-center rounded-pill border border-border px-2 py-0.5 text-xs',
        toneText[tone],
      )}
    >
      {children}
    </span>
  )
}

/** 占用条：value / cap，只做视觉，不做计算 */
export function Bar({
  value,
  cap,
  tone = 'brand',
}: {
  value: number
  cap: number
  tone?: Tone
}) {
  const pct = cap > 0 ? Math.min(100, Math.round((value / cap) * 100)) : 0
  return (
    <div className="h-1.5 w-full overflow-hidden rounded-pill bg-bg-subtle">
      <div className={cx('h-full rounded-pill', toneBg[tone])} style={{ width: `${pct}%` }} />
    </div>
  )
}

export function Stat({
  label,
  value,
  hint,
}: {
  label: string
  value: ReactNode
  hint?: string
}) {
  return (
    <Card className="p-4">
      <div className="text-xs text-fg-muted">{label}</div>
      <div className="mt-1 text-2xl font-medium tracking-tight">{value}</div>
      {hint ? <div className="mt-1 text-xs text-fg-subtle">{hint}</div> : null}
    </Card>
  )
}

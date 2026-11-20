import { useEffect, useRef, useState } from 'react'
import { useNavigate } from '@tanstack/react-router'
import { ArrowRight, Eye, EyeOff } from 'lucide-react'
import { Button } from '../../components/ui'

const strengthOf = (pw: string) => {
  let s = 0
  if (pw.length >= 8) s++
  if (pw.length >= 12) s++
  if (pw.length >= 16) s++
  if (/[^a-zA-Z0-9]/.test(pw)) s++
  if (/[a-z]/.test(pw) && /[A-Z]/.test(pw) && /\d/.test(pw)) s++
  return Math.min(4, s)
}
const strengthLabel = ['太短', '弱', '一般', '强', '很强']
const strengthColor = ['bg-danger', 'bg-danger', 'bg-warn', 'bg-brand', 'bg-ok']

/** 登录 / 注册是同一张卡的正反面：点一下翻过来，不跳页 */
export function AuthCard() {
  const navigate = useNavigate()
  const [flipped, setFlipped] = useState(false)
  const [heights, setHeights] = useState({ front: 0, back: 0 })
  const front = useRef<HTMLDivElement | null>(null)
  const back = useRef<HTMLDivElement | null>(null)
  // 两面都量，取当前朝前那面的高度——只量一面会在翻转瞬间取到旧值
  const height = flipped ? heights.back : heights.front

  useEffect(() => {
    const measure = () => {
      setHeights({
        front: front.current?.offsetHeight ?? 0,
        back: back.current?.offsetHeight ?? 0,
      })
    }
    measure()
    const ro = new ResizeObserver(measure)
    if (front.current) ro.observe(front.current)
    if (back.current) ro.observe(back.current)
    window.addEventListener('resize', measure)
    return () => {
      ro.disconnect()
      window.removeEventListener('resize', measure)
    }
  }, [])

  return (
    <div
      data-flipped={flipped ? '1' : '0'}
      className="mx-auto w-full max-w-sm [perspective:1800px]"
      style={{ height, transition: 'height 420ms cubic-bezier(0.2, 0.8, 0.2, 1)' }}
    >
      <div
        className="relative h-full transition-transform duration-[560ms] ease-[cubic-bezier(0.2,0.8,0.2,1)] [transform-style:preserve-3d]"
        style={{ transform: flipped ? 'rotateY(180deg)' : 'none' }}
      >
        <div
          ref={front}
          data-meteor-target
          inert={flipped}
          className="glass absolute inset-x-0 top-0 rounded-card p-5 [backface-visibility:hidden] md:p-7"
        >
          <LoginFace
            onFlip={() => setFlipped(true)}
            onSubmit={() => void navigate({ to: '/' })}
          />
        </div>

        <div
          ref={back}
          inert={!flipped}
          style={{ transform: 'rotateY(180deg)' }}
          className="glass absolute inset-x-0 top-0 rounded-card p-5 [backface-visibility:hidden] md:p-7"
        >
          <RegisterFace
            onFlip={() => setFlipped(false)}
            onSubmit={() => void navigate({ to: '/' })}
          />
        </div>
      </div>
    </div>
  )
}

function LoginFace({ onFlip, onSubmit }: { onFlip: () => void; onSubmit: () => void }) {
  const [show, setShow] = useState(false)
  return (
    <>
      <h2 className="text-lg font-medium tracking-tight">登录工作台</h2>
      <p className="mt-1 text-xs text-fg-muted">使用分配给你的账号；模型由架阁提供，也可自备。</p>

      <form
        className="mt-5 space-y-3"
        onSubmit={(e) => {
          e.preventDefault()
          onSubmit()
        }}
      >
        <label className="block">
          <span className="mb-1.5 block text-xs text-fg-muted">用户名</span>
          <input
            name="username"
            autoComplete="username"
            defaultValue="chen.mo"
            className="glass-field h-10 w-full rounded-field border border-border px-3 text-sm outline-none transition-colors focus:border-brand"
          />
        </label>

        <label className="block">
          <span className="mb-1.5 block text-xs text-fg-muted">密码</span>
          <div className="relative">
            <input
              name="password"
              type={show ? 'text' : 'password'}
              autoComplete="current-password"
              className="glass-field h-10 w-full rounded-field border border-border px-3 pr-10 text-sm outline-none transition-colors focus:border-brand"
            />
            <button
              type="button"
              onClick={() => setShow((v) => !v)}
              aria-label={show ? '隐藏密码' : '显示密码'}
              className="absolute right-1 top-1 inline-flex h-8 w-8 items-center justify-center rounded-field text-fg-subtle hover:text-fg"
            >
              {show ? <EyeOff size={15} /> : <Eye size={15} />}
            </button>
          </div>
        </label>

        <label className="inline-flex items-center gap-1.5 pt-1 text-xs text-fg-muted">
          <input type="checkbox" defaultChecked className="accent-[var(--brand)]" />
          记住我
        </label>

        <Button type="submit" pill size="lg" variant="primary" className="w-full">
          进入
          <ArrowRight size={15} />
        </Button>
      </form>

      <p className="mt-4 text-center text-xs text-fg-subtle">
        还没有账号？
        <button
          type="button"
          onClick={onFlip}
          className="ml-1 font-medium text-brand hover:underline"
        >
          注册
        </button>
      </p>
    </>
  )
}

function RegisterFace({ onFlip, onSubmit }: { onFlip: () => void; onSubmit: () => void }) {
  const [name, setName] = useState('')
  const [pw, setPw] = useState('')
  const [show, setShow] = useState(false)
  const s = strengthOf(pw)
  const ok = name.trim().length > 0 && pw.length >= 12

  return (
    <>
      <h2 className="text-lg font-medium tracking-tight">创建账号</h2>
      <p className="mt-1 text-xs text-fg-muted">先开号，档位晚点再说：免费直接用，要升级随时升。</p>

      <form
        className="mt-4 space-y-3"
        onSubmit={(e) => {
          e.preventDefault()
          if (ok) onSubmit()
        }}
      >
        <label className="block">
          <span className="mb-1.5 block text-xs text-fg-muted">用户名</span>
          <input
            name="username"
            autoComplete="username"
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="例：chen.mo"
            aria-label="用户名"
            className="glass-field h-10 w-full rounded-field border border-border px-3 text-sm outline-none transition-colors placeholder:text-fg-subtle focus:border-brand"
          />
        </label>

        <label className="block">
          <span className="mb-1.5 flex items-center justify-between text-xs text-fg-muted">
            密码
            <span className="text-[10.5px] text-fg-subtle">至少 12 位</span>
          </span>
          <div className="relative">
            <input
              name="password"
              type={show ? 'text' : 'password'}
              autoComplete="new-password"
              value={pw}
              onChange={(e) => setPw(e.target.value)}
              aria-label="密码"
              className="glass-field h-10 w-full rounded-field border border-border px-3 pr-10 text-sm outline-none transition-colors focus:border-brand"
            />
            <button
              type="button"
              onClick={() => setShow((v) => !v)}
              aria-label={show ? '隐藏密码' : '显示密码'}
              className="absolute right-1 top-1 inline-flex h-8 w-8 items-center justify-center rounded-field text-fg-subtle hover:text-fg"
            >
              {show ? <EyeOff size={15} /> : <Eye size={15} />}
            </button>
          </div>

          <span className="mt-1.5 flex items-center gap-1" aria-label="密码强度">
            {[0, 1, 2, 3].map((i) => (
              <span
                key={i}
                className={
                  'h-1 flex-1 rounded-pill transition-colors ' +
                  (pw.length === 0
                    ? 'bg-bg-accent'
                    : i < s
                      ? strengthColor[s]
                      : 'bg-bg-accent')
                }
              />
            ))}
            <span className="ml-1 w-8 shrink-0 text-right text-[10px] text-fg-subtle">
              {pw.length === 0 ? '' : strengthLabel[s]}
            </span>
          </span>
        </label>

        <Button type="submit" pill size="lg" variant="primary" className="w-full" disabled={!ok}>
          创建账号
          <ArrowRight size={15} />
        </Button>

        <p className="text-[10.5px] leading-5 text-fg-subtle">
          我们会持续加固（TLS、静态加密、最小权限、日志不落正文），但没有任何系统能保证绝对不被攻破；一旦发生泄露，我们承诺 72 小时内告知受影响范围与处置进展。
        </p>
      </form>

      <p className="mt-3 text-center text-xs text-fg-subtle">
        已有账号？
        <button type="button" onClick={onFlip} className="ml-1 font-medium text-brand hover:underline">
          登录
        </button>
      </p>
    </>
  )
}

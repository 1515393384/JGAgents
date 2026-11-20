import { useEffect, useRef, useState, type ReactNode } from 'react'
import { GripHorizontal, Maximize2, Minimize2, MoveDiagonal2, X } from 'lucide-react'

export type WindowRect = { x: number; y: number; w: number; h: number }

const M = 8 // 离视口至少 8px，永远留一条能呼吸的边
const MIN_W = 380
const MIN_H = 300

function clampInto(r: WindowRect): WindowRect {
  const vw = window.innerWidth
  const vh = window.innerHeight
  const w = Math.min(Math.max(r.w, MIN_W), vw - M * 2)
  const h = Math.min(Math.max(r.h, MIN_H), vh - M * 2)
  const x = Math.min(Math.max(r.x, M), Math.max(M, vw - w - M))
  const y = Math.min(Math.max(r.y, M), Math.max(M, vh - h - M))
  return { x, y, w, h }
}

/** 缩放的夹取：只动右下角，左上角钉住不动，最多长到贴边 */
function clampSize(r: WindowRect): WindowRect {
  const maxW = Math.max(0, window.innerWidth - r.x - M)
  const maxH = Math.max(0, window.innerHeight - r.y - M)
  const floorW = Math.min(MIN_W, maxW)
  const floorH = Math.min(MIN_H, maxH)
  return {
    x: r.x,
    y: r.y,
    w: Math.min(Math.max(r.w, floorW), Math.max(floorW, maxW)),
    h: Math.min(Math.max(r.h, floorH), Math.max(floorH, maxH)),
  }
}

/** 微应用的默认落点：桌面 72% 宽、居中偏左让开右边 dock；手机上铺满 */
function defaultRect(): WindowRect {
  const vw = window.innerWidth
  const vh = window.innerHeight
  if (vw < 768) return clampInto({ x: M, y: 44, w: vw - M * 2, h: vh - 96 })
  const w = Math.round(Math.min(vw * 0.72, 980))
  return clampInto({
    x: Math.round(vw / 2 - 28 - w / 2),
    y: Math.round(vh * 0.06),
    w,
    h: Math.round(vh * 0.82),
  })
}

function fullRect(): WindowRect {
  return { x: M, y: M, w: window.innerWidth - M * 2, h: window.innerHeight - M * 2 }
}

/**
 * 微应用统一窗口：拖标题栏移动、拖右下角缩放、双击标题栏或点按钮全屏。
 * 所有微应用（文档、表格、目录窗、空间面板）都用这一层壳，行为一致。
 */
export function MicroWindow({
  open,
  title,
  subtitle,
  icon,
  status,
  footer,
  onClose,
  children,
}: {
  open: boolean
  title: ReactNode
  subtitle?: ReactNode
  icon?: ReactNode
  /** 标题右侧的状态徽标、动作按钮 */
  status?: ReactNode
  /** 贴底的常驻条，不参与缩放计算，也不改窗口高度 */
  footer?: ReactNode
  onClose: () => void
  children: ReactNode
}) {
  const [rect, setRect] = useState<WindowRect>(defaultRect)
  const [full, setFull] = useState(false)
  const [mounted, setMounted] = useState(open)
  const [phase, setPhase] = useState<'in' | 'out'>('out')
  const [morph, setMorph] = useState(false)
  const restore = useRef<WindowRect | null>(null)
  const act = useRef<{ kind: 'move' | 'size'; px: number; py: number; base: WindowRect } | null>(null)

  // 进出场：从右边缘的 dock 图标长出来，关的时候缩回那个图标里
  useEffect(() => {
    if (open) {
      setMounted(true)
      let r2 = 0
      const r1 = requestAnimationFrame(() => {
        r2 = requestAnimationFrame(() => setPhase('in'))
      })
      return () => {
        cancelAnimationFrame(r1)
        cancelAnimationFrame(r2)
      }
    }
    if (!mounted) return
    setPhase('out')
    const t = window.setTimeout(() => {
      setMounted(false)
      // 缩回图标之后，下次打开是一张干净的新窗口
      setFull(false)
      setRect(defaultRect())
    }, 220)
    return () => window.clearTimeout(t)
  }, [open, mounted])

  useEffect(() => {
    const move = (e: PointerEvent) => {
      const a = act.current
      if (!a) return
      const dx = e.clientX - a.px
      const dy = e.clientY - a.py
      setRect(
        a.kind === 'move'
          ? clampInto({ ...a.base, x: a.base.x + dx, y: a.base.y + dy })
          : clampSize({ ...a.base, w: a.base.w + dx, h: a.base.h + dy }),
      )
    }
    const up = () => {
      act.current = null
      document.body.style.userSelect = ''
    }
    const onResize = () => setRect((r) => clampInto(r))
    window.addEventListener('pointermove', move)
    window.addEventListener('pointerup', up)
    window.addEventListener('pointercancel', up)
    window.addEventListener('resize', onResize)
    return () => {
      window.removeEventListener('pointermove', move)
      window.removeEventListener('pointerup', up)
      window.removeEventListener('pointercancel', up)
      window.removeEventListener('resize', onResize)
    }
  }, [])

  const start = (kind: 'move' | 'size') => (e: React.PointerEvent) => {
    if (full) return
    act.current = { kind, px: e.clientX, py: e.clientY, base: rect }
    document.body.style.userSelect = 'none'
  }

  const toggleFull = () => {
    setMorph(true)
    window.setTimeout(() => setMorph(false), 260)
    if (full) {
      setRect(restore.current ? clampInto(restore.current) : defaultRect())
      setFull(false)
    } else {
      restore.current = rect
      setRect(fullRect())
      setFull(true)
    }
  }

  if (!mounted) return null

  // 出场的终点 = 右边缘 dock 那颗图标；进场也从那儿长出来
  const iconX = window.innerWidth - 6
  const iconY = window.innerHeight / 2
  const dx = iconX - (rect.x + rect.w / 2)
  const dy = iconY - (rect.y + rect.h / 2)
  const leaving = phase === 'out'

  return (
    <>
      <div
        style={{ opacity: leaving ? 0 : 1, transition: 'opacity 200ms ease' }}
        className="fixed inset-0 z-30 bg-black/25 backdrop-blur-[2px]"
        onClick={onClose}
        aria-hidden
      />

      <div
        data-probe="micro"
        data-full={full ? '1' : '0'}
        data-phase={phase}
        style={{
          left: rect.x,
          top: rect.y,
          width: rect.w,
          height: rect.h,
          opacity: leaving ? 0 : 1,
          transform: leaving ? `translate(${dx}px, ${dy}px) scale(0.12)` : 'none',
          transformOrigin: '50% 50%',
          transition:
            'opacity 200ms ease, transform 200ms cubic-bezier(0.4, 0, 1, 1)' +
            (morph ? ', left 220ms ease, top 220ms ease, width 220ms ease, height 220ms ease' : ''),
          pointerEvents: leaving ? 'none' : undefined,
        }}
        className="fixed z-40 flex flex-col overflow-hidden rounded-panel border border-surface-border bg-surface shadow-float"
      >
        <div
          onPointerDown={start('move')}
          onDoubleClick={toggleFull}
          className={
            'flex h-11 shrink-0 items-center gap-2 border-b border-surface-border px-2.5 ' +
            (full ? '' : 'cursor-grab active:cursor-grabbing')
          }
        >
          <GripHorizontal size={13} className="shrink-0 text-fg-subtle" />
          {icon ? <span className="shrink-0 text-fg-subtle">{icon}</span> : null}
          <span className="min-w-0 truncate text-[12.5px] font-medium">{title}</span>
          {subtitle ? (
            <span className="shrink-0 rounded-pill bg-bg-subtle px-1.5 py-0.5 font-mono text-[10.5px] text-fg-subtle">
              {subtitle}
            </span>
          ) : null}
          {status}

          <div className="ml-auto flex shrink-0 items-center gap-0.5">
            <button
              type="button"
              onClick={toggleFull}
              aria-label={full ? '还原窗口' : '全屏'}
              title={full ? '还原窗口' : '全屏'}
              className="inline-flex h-7 w-7 items-center justify-center rounded-field text-fg-subtle hover:bg-bg-subtle hover:text-fg"
            >
              {full ? <Minimize2 size={13} /> : <Maximize2 size={13} />}
            </button>
            <button
              type="button"
              onClick={onClose}
              aria-label="关闭窗口"
              title="关闭窗口"
              className="inline-flex h-7 w-7 items-center justify-center rounded-field text-fg-subtle hover:bg-bg-subtle hover:text-fg"
            >
              <X size={14} />
            </button>
          </div>
        </div>

        <div className="flex min-h-0 flex-1 flex-col">{children}</div>

        {footer ? <div className="relative shrink-0">{footer}</div> : null}

        {full ? null : (
          <div
            onPointerDown={start('size')}
            role="presentation"
            aria-label="拖拽缩放"
            title="拖这里改窗口大小"
            className="group/grip absolute bottom-1 right-1 z-10 inline-flex h-5 w-5 cursor-nwse-resize items-center justify-center rounded-[5px] border border-surface-border bg-surface/85 text-fg-subtle/70 backdrop-blur-[2px] transition-colors hover:border-brand/50 hover:bg-brand-soft hover:text-brand"
          >
            <MoveDiagonal2 size={11} strokeWidth={2.2} />
          </div>
        )}
      </div>
    </>
  )
}

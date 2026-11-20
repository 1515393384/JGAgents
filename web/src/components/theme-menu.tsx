import { useEffect, useRef, useState } from 'react'
import { Check, Palette, X } from 'lucide-react'
import { presets, useTheme } from '../app/theme'

export function ThemeMenu() {
  const { preset, setPreset } = useTheme()
  const [open, setOpen] = useState(false)
  const [pos, setPos] = useState<{ left: number; bottom: number } | null>(null)
  const boxRef = useRef<HTMLDivElement>(null)
  const btnRef = useRef<HTMLButtonElement>(null)
  const popRef = useRef<HTMLDivElement>(null)

  const W = 168
  const GAP = 8

  /** 弹层固定在按钮上方，但要夹在视口里：
   * 左栏只有 236px，按按钮右缘对齐会直接跑到屏幕外面去。 */
  useEffect(() => {
    if (!open) return
    const place = () => {
      const r = btnRef.current?.getBoundingClientRect()
      if (!r) return
      const left = Math.min(Math.max(GAP, r.right - W), window.innerWidth - W - GAP)
      setPos({ left, bottom: window.innerHeight - r.top + GAP })
    }
    place()
    window.addEventListener('resize', place)
    return () => window.removeEventListener('resize', place)
  }, [open])

  useEffect(() => {
    if (!open) return
    const onDown = (e: MouseEvent) => {
      const t = e.target as Node
      if (!boxRef.current?.contains(t) && !popRef.current?.contains(t)) setOpen(false)
    }
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setOpen(false)
    }
    document.addEventListener('mousedown', onDown)
    document.addEventListener('keydown', onKey)
    return () => {
      document.removeEventListener('mousedown', onDown)
      document.removeEventListener('keydown', onKey)
    }
  }, [open])

  return (
    <div ref={boxRef} className="relative">
      <button
        ref={btnRef}
        type="button"
        onClick={() => setOpen((v) => !v)}
        aria-label="切换主题"
        aria-expanded={open}
        className={
          'inline-flex h-7 w-7 items-center justify-center rounded-field hover:bg-bg-subtle hover:text-fg ' +
          (open ? 'bg-bg-subtle text-fg' : 'text-fg-subtle')
        }
      >
        <Palette size={14} />
      </button>

      {open ? (
        <div
          ref={popRef}
          style={{
            left: pos?.left ?? GAP,
            bottom: pos?.bottom ?? 0,
            width: W,
            visibility: pos ? 'visible' : 'hidden',
          }}
          className="fixed z-50 overflow-hidden rounded-panel border border-surface-border bg-surface shadow-float"
        >
          <div className="flex items-center justify-between px-2.5 py-2 text-[11px] font-medium text-fg-subtle">
            主题
            <button
              type="button"
              onClick={() => setOpen(false)}
              className="inline-flex h-6 w-6 items-center justify-center rounded-field hover:bg-bg-subtle hover:text-fg"
              aria-label="收起"
            >
              <X size={12} />
            </button>
          </div>

          <div className="p-1.5 pt-0">
            {presets.map((p) => {
              const active = preset === p.id
              return (
                <button
                  key={p.id}
                  type="button"
                  onClick={() => setPreset(p.id)}
                  className={
                    'flex w-full items-center gap-1.5 rounded-field px-1.5 py-1.5 text-left text-[12.5px] ' +
                    (active
                      ? 'bg-bg-subtle text-fg'
                      : 'text-fg-muted hover:bg-bg-subtle/60 hover:text-fg')
                  }
                >
                  <span className="flex shrink-0 items-center gap-0.5">
                    {p.swatch.map((c) => (
                      <span
                        key={c}
                        style={{ backgroundColor: c }}
                        className="h-2.5 w-2.5 rounded-pill ring-1 ring-surface-border"
                      />
                    ))}
                  </span>
                  <span className="flex-1 truncate">{p.label}</span>
                  {active ? <Check size={13} className="shrink-0 text-brand" /> : null}
                </button>
              )
            })}
          </div>
        </div>
      ) : null}
    </div>
  )
}

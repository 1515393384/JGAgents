import { useEffect, useState } from 'react'

type Role = 'user' | 'engine' | 'mate'

type Turn = { role: Role; text: string; who?: string }

/** 一段办公现场：改标书、补材料、提醒，两个人同时在场。四句够看，不堆。 */
const SCRIPT: Turn[] = [
  { role: 'user', text: '第三章按甲方模板改一版' },
  { role: 'engine', text: '改好了，金额小写也核过。' },
  { role: 'mate', who: '王工', text: '资质材料我接着补' },
  { role: 'engine', text: '同一版，不会打架；周五提醒你交。' },
]

const CHAR_MS = 26
const TURN_PAUSE_MS = 460
const HOLD_MS = 4600

const ENDS = SCRIPT.reduce<number[]>((acc, t, i) => {
  acc.push((acc[i - 1] ?? 0) + t.text.length)
  return acc
}, [])
const TOTAL = ENDS[ENDS.length - 1] ?? 0

const BUBBLE = 'rounded-[13px] px-2.5 py-1 text-[11px] leading-[1.45] md:text-[12.5px]'

function Mark({ char, tone }: { char: string; tone: 'engine' | 'mate' }) {
  const skin =
    tone === 'engine'
      ? 'border-brand/40 bg-brand/15 text-brand'
      : 'border-[rgba(139,92,246,.45)] bg-[rgba(139,92,246,.16)] text-[#8B5CF6]'
  return (
    <span
      className={`flex h-6 w-6 shrink-0 items-center justify-center rounded-[7px] border text-[11px] font-medium ${skin}`}
    >
      {char}
    </span>
  )
}

/**
 * 稳住高度：整句用 invisible 占位，逐字出来的那层叠在同一个格子里。
 * 这样气泡从第一帧就是"满行"的高度，打字的字长变化不会把窗口撑高。
 */
function Held({ text, typed, typing }: { text: string; typed: number; typing: boolean }) {
  return (
    <span className="inline-grid">
      <span className="invisible [grid-area:1/1]" aria-hidden>
        {text}
      </span>
      <span className="[grid-area:1/1]">
        {text.slice(0, typed)}
        {typing && (
          <span className="caret-blink ml-0.5 inline-block h-3 w-[3px] translate-y-0.5 bg-current align-baseline" />
        )}
      </span>
    </span>
  )
}

/**
 * 对话预览：一问一答逐字打出来。
 * 所有气泡从第一帧就按满行高度占好了位置，窗口高度从头到尾不变。
 */
export function ChatPreview() {
  const [pos, setPos] = useState(0)
  const [full, setFull] = useState(false)

  useEffect(() => {
    if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
      setFull(true)
      return
    }

    let timer = 0
    let p = 0
    let stopped = false

    const schedule = (ms: number) => {
      timer = window.setTimeout(tick, ms)
    }

    const tick = () => {
      if (stopped) return
      p += 1
      setPos(p)
      if (p >= TOTAL) {
        timer = window.setTimeout(() => {
          if (stopped) return
          p = 0
          setPos(0)
          schedule(TURN_PAUSE_MS)
        }, HOLD_MS)
        return
      }
      schedule(ENDS.includes(p) ? TURN_PAUSE_MS : CHAR_MS)
    }

    schedule(520)
    return () => {
      stopped = true
      window.clearTimeout(timer)
    }
  }, [])

  const activeTurn = ENDS.findIndex((end) => pos <= end)

  return (
    <div className="mx-auto w-full max-w-xl lg:mx-0">
      <div className="glass rounded-card p-3 text-left md:p-4">
        <div className="flex items-center gap-2 border-b border-border pb-2.5">
          <Mark char="阁" tone="engine" />
          <div className="min-w-0">
            <div className="text-xs font-medium leading-tight">阁吏</div>
          </div>
          <span className="ml-auto text-[10px] text-fg-subtle">标书 2024-甲</span>
        </div>

        <div className="space-y-1.5 pt-2.5">
          {SCRIPT.map((t, i) => {
            const start = i === 0 ? 0 : (ENDS[i - 1] ?? 0)
            const shown = full ? t.text.length : Math.max(0, Math.min(pos - start, t.text.length))
            const pending = !full && shown === 0
            const typing = !full && i === activeTurn

            const body = <Held text={t.text} typed={shown} typing={typing} />

            if (t.role === 'engine') {
              return (
                <div key={t.text} className={`flex items-start gap-2 ${pending ? 'invisible' : ''}`}>
                  <Mark char="阁" tone="engine" />
                  <div className={`${BUBBLE} max-w-[85%] rounded-bl-[4px] bg-bg-subtle/70 text-fg`}>{body}</div>
                </div>
              )
            }

            const isMate = t.role === 'mate'
            return (
              <div key={t.text} className={`flex items-start justify-end gap-2 ${pending ? 'invisible' : ''}`}>
                <div
                  className={`${BUBBLE} max-w-[85%] rounded-br-[4px] ${
                    isMate ? 'bg-[rgba(139,92,246,.16)] text-fg' : 'bg-brand/15 text-fg'
                  }`}
                >
                  {body}
                </div>
                {isMate ? <Mark char={t.who?.slice(0, 1) ?? '同'} tone="mate" /> : null}
              </div>
            )
          })}
        </div>
      </div>
    </div>
  )
}

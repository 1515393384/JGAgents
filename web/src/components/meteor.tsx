import { useEffect, useRef } from 'react'
import { useTheme } from '../app/theme'

/**
 * 撞色对：一冷一暖。每颗流星落地时整对取色，
 * 冷色做主喷、暖色做边刺，撞出来才有“对撞”的观感。
 */
const DEFAULT_PAIR: [string, string] = ['#2E9BE8', '#FF7A59']

const CLASH_LIGHT: [string, string][] = [
  ['#2E9BE8', '#FF7A59'],
  ['#7C5CFF', '#FFC24B'],
  ['#12B5A5', '#FF5C8A'],
]
const CLASH_DARK: [string, string][] = [
  ['#6CB6FF', '#FF9C6B'],
  ['#A78BFA', '#FFD166'],
  ['#38E1D0', '#FF6E9C'],
]

type Spark = {
  x: number
  y: number
  vx: number
  vy: number
  age: number
  ttl: number
  size: number
  color: string
}

type Meteor = {
  x: number
  y: number
  vx: number
  vy: number
  /** 命中线：卡片顶沿的 y */
  ty: number
  tail: number
  color: string
}

type Ring = { x: number; y: number; age: number; ttl: number }

const MAX_SPARKS = 420
const MAX_METEORS = 3

/**
 * 流星撞卡片：整页一块透明 canvas，压在内容之上、不接事件。
 * 流星在空白区飞，命中卡片顶沿的那一刻炸开撞色粒子 + 一圈冲击波。
 * 目标用 `data-meteor-target` 标记，每帧重算，窗口缩放/滚动都跟得上。
 */
export function MeteorField({ selector = '[data-meteor-target]' }: { selector?: string }) {
  const ref = useRef<HTMLCanvasElement | null>(null)
  const { theme } = useTheme()
  const themeRef = useRef(theme)
  themeRef.current = theme

  useEffect(() => {
    const canvas = ref.current
    if (!canvas) return
    if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) return

    const ctx = canvas.getContext('2d')
    if (!ctx) return

    let width = 0
    let height = 0
    let dpr = 1

    const resize = () => {
      /* 1.5 封顶：这是一层雾状粒子，1.5 和 2 肉眼分不出来，像素少 44% */
      dpr = Math.min(window.devicePixelRatio || 1, 1.5)
      width = window.innerWidth
      height = window.innerHeight
      canvas.width = Math.round(width * dpr)
      canvas.height = Math.round(height * dpr)
      canvas.style.width = `${width}px`
      canvas.style.height = `${height}px`
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0)
    }
    resize()

    const compact = window.innerWidth < 768
    const meteors: Meteor[] = []
    const sparks: Spark[] = []
    const rings: Ring[] = []

    /* 天幕：暗色是星点，亮色是浮尘。和流星共用一块 canvas、一个 rAF。 */
    type Mote = {
      x: number
      y: number
      vx: number
      vy: number
      r: number
      phase: number
      spin: number
      alpha: number
      color: string
    }
    let motes: Mote[] = []
    let clock = 0

    const buildMotes = (dark: boolean) => {
      const count = compact ? 40 : 96
      const palette = dark ? CLASH_DARK : CLASH_LIGHT
      motes = Array.from({ length: count }, (_, i) => {
        const pair = palette[i % palette.length] ?? DEFAULT_PAIR
        return {
          x: Math.random() * width,
          y: Math.random() * height,
          vx: (Math.random() - 0.5) * 8,
          vy: -2 - Math.random() * 10,
          // 星点要小要硬，浮尘要大要软
          r: dark ? 0.5 + Math.random() * 0.9 : 0.9 + Math.random() * 1.6,
          phase: Math.random() * Math.PI * 2,
          spin: 0.6 + Math.random() * 1.6,
          alpha: dark ? 0.25 + Math.random() * 0.5 : 0.05 + Math.random() * 0.11,
          // 冷暖各半：背景自己也有撞色，不是一片蓝
          color: i % 2 === 0 ? pair[0] : pair[1],
        }
      })
    }
    let lastDark = themeRef.current === 'dark'
    buildMotes(lastDark)

    const burst = (x: number, y: number) => {
      const dark = themeRef.current === 'dark'
      const palette = dark ? CLASH_DARK : CLASH_LIGHT
      const pair = palette[Math.floor(Math.random() * palette.length)] ?? DEFAULT_PAIR
      const [cool, warm] = pair
      const count = compact ? 14 : 24
      for (let i = 0; i < count; i += 1) {
        // 交替取冷/暖，两边数量对齐，混出来才不脏
        const def = i % 2 === 0 ? cool : warm
        const angle = Math.random() * Math.PI * 2
        const speed = 40 + Math.random() * 300
        sparks.push({
          x,
          y,
          vx: Math.cos(angle) * speed,
          vy: Math.sin(angle) * speed - 30,
          age: 0,
          ttl: 0.42 + Math.random() * 0.45,
          size: 1 + Math.random() * 1.9,
          color: def,
        })
      }
      if (sparks.length > MAX_SPARKS) sparks.splice(0, sparks.length - MAX_SPARKS)
      rings.push({ x, y, age: 0, ttl: 0.44 })
      // 命中点留一颗短暂的高光核，亮面压住四周火花
      sparks.push({ x, y, vx: 0, vy: 0, age: 0, ttl: 0.16, size: 3.4, color: dark ? '#FFFFFF' : '#1F6FA8' })
    }

    const spawnMeteor = () => {
      if (meteors.length >= MAX_METEORS) return
      const nodes = Array.from(document.querySelectorAll<HTMLElement>(selector))
      const target = nodes[Math.floor(Math.random() * nodes.length)]
      if (!target) return

      const rect = target.getBoundingClientRect()
      if (rect.width < 40 || rect.bottom < 40 || rect.top > height - 40) return

      // 落点取卡片顶沿：偏中间更密，避免总在边角炸
      const tx = rect.left + rect.width * (0.15 + Math.random() * 0.7)
      const ty = rect.top + 8 + Math.random() * 10

      // 斜着来：与竖直方向 18°~38°，左右随机
      const tilt = (18 + Math.random() * 20) * (Math.PI / 180)
      const dir = Math.random() < 0.5 ? -1 : 1
      const ux = Math.sin(tilt) * dir
      const uy = Math.cos(tilt)

      const speed = 780 + Math.random() * 520
      const startDist = (ty + 80) / uy

      meteors.push({
        x: tx - ux * startDist,
        y: ty - uy * startDist,
        vx: ux * speed,
        vy: uy * speed,
        ty,
        tail: 120 + Math.random() * 140,
        color: themeRef.current === 'dark' ? '#BFE0FF' : '#1F6FA8',
      })
    }

    let raf = 0
    let last = performance.now()
    let nextSpawn = 700
    /* 空闲节流：没有流星/火花/冲击波时，天幕只有慢漂的尘，
       没必要 60fps 全屏重画 —— 隔帧画一次，肉眼一样。 */
    let idleSkip = false

    const frame = (now: number) => {
      const dt = Math.min((now - last) / 1000, 0.05)
      last = now

      if (meteors.length === 0 && sparks.length === 0 && rings.length === 0) {
        idleSkip = !idleSkip
        if (idleSkip) {
          raf = window.requestAnimationFrame(frame)
          return
        }
      }

      // 生成节奏：宽屏密、窄屏疏
      nextSpawn -= dt * 1000
      if (nextSpawn <= 0) {
        spawnMeteor()
        nextSpawn = compact ? 2200 + Math.random() * 1800 : 1000 + Math.random() * 1400
      }

      ctx.clearRect(0, 0, width, height)
      clock += dt

      const isDark = themeRef.current === 'dark'
      if (isDark !== lastDark) {
        lastDark = isDark
        buildMotes(isDark)
      }

      // ── 天幕粒子：慢漂 + 呼吸，永远在最底层 ──
      ctx.globalCompositeOperation = 'source-over'
      for (let i = 0; i < motes.length; i += 1) {
        const mo = motes[i]
        if (!mo) continue
        mo.x += mo.vx * dt
        mo.y += mo.vy * dt
        if (mo.y < -12) {
          mo.y = height + 12
          mo.x = Math.random() * width
        }
        if (mo.x < -12) mo.x = width + 12
        else if (mo.x > width + 12) mo.x = -12

        ctx.globalAlpha = mo.alpha * (0.72 + 0.28 * Math.sin(clock * mo.spin + mo.phase))
        ctx.fillStyle = mo.color
        ctx.beginPath()
        ctx.arc(mo.x, mo.y, mo.r, 0, Math.PI * 2)
        ctx.fill()
      }
      ctx.globalAlpha = 1

      // ── 流星本体 + 拖尾 ──
      ctx.globalCompositeOperation = 'source-over'
      for (let i = meteors.length - 1; i >= 0; i -= 1) {
        const m = meteors[i]
        if (!m) continue
        m.x += m.vx * dt
        m.y += m.vy * dt

        const hit = m.y >= m.ty
        const headX = m.x
        const headY = hit ? m.ty : m.y
        const tailX = headX - (m.vx / Math.hypot(m.vx, m.vy)) * m.tail
        const tailY = headY - (m.vy / Math.hypot(m.vx, m.vy)) * m.tail

        const grad = ctx.createLinearGradient(tailX, tailY, headX, headY)
        grad.addColorStop(0, 'rgba(0,0,0,0)')
        grad.addColorStop(0.7, m.color.startsWith('#') ? `${m.color}66` : m.color)
        grad.addColorStop(1, m.color)
        ctx.strokeStyle = grad
        ctx.lineWidth = 1.6
        ctx.lineCap = 'round'
        ctx.beginPath()
        ctx.moveTo(tailX, tailY)
        ctx.lineTo(headX, headY)
        ctx.stroke()

        ctx.fillStyle = themeRef.current === 'dark' ? '#FFFFFF' : '#0B3E5F'
        ctx.beginPath()
        ctx.arc(headX, headY, 1.7, 0, Math.PI * 2)
        ctx.fill()

        if (hit) {
          burst(headX, m.ty)
          meteors.splice(i, 1)
        } else if (m.y > height + 60 || m.x < -160 || m.x > width + 160) {
          meteors.splice(i, 1)
        }
      }

      // ── 火花：暗色底用加色更亮，亮色底用正常混合才不糊成白 ──
      if (sparks.length > 0) {
        ctx.globalCompositeOperation = themeRef.current === 'dark' ? 'lighter' : 'source-over'
        for (let i = sparks.length - 1; i >= 0; i -= 1) {
          const s = sparks[i]
          if (!s) continue
          s.age += dt
          if (s.age >= s.ttl) {
            sparks.splice(i, 1)
            continue
          }
          s.vy += 620 * dt
          s.vx *= 1 - 1.6 * dt
          s.x += s.vx * dt
          s.y += s.vy * dt

          const k = 1 - s.age / s.ttl
          ctx.globalAlpha = k * k
          ctx.fillStyle = s.color
          ctx.beginPath()
          ctx.arc(s.x, s.y, s.size * (0.5 + k * 0.7), 0, Math.PI * 2)
          ctx.fill()
        }
        ctx.globalAlpha = 1
      }

      // ── 冲击波：一圈细环，快速散掉 ──
      for (let i = rings.length - 1; i >= 0; i -= 1) {
        const r = rings[i]
        if (!r) continue
        r.age += dt
        if (r.age >= r.ttl) {
          rings.splice(i, 1)
          continue
        }
        const k = r.age / r.ttl
        ctx.strokeStyle = themeRef.current === 'dark' ? `rgba(190,224,255,${0.5 * (1 - k)})` : `rgba(31,111,168,${0.42 * (1 - k)})`
        ctx.lineWidth = 1.2 * (1 - k) + 0.4
        ctx.beginPath()
        ctx.arc(r.x, r.y, 6 + k * 86, 0, Math.PI * 2)
        ctx.stroke()
      }

      ctx.globalCompositeOperation = 'source-over'
      raf = window.requestAnimationFrame(frame)
    }

    raf = window.requestAnimationFrame(frame)

    const onResize = () => {
      resize()
    }
    const onVisibility = () => {
      if (document.hidden) {
        window.cancelAnimationFrame(raf)
        raf = 0
        return
      }
      if (!raf) {
        last = performance.now()
        raf = window.requestAnimationFrame(frame)
      }
    }

    window.addEventListener('resize', onResize)
    document.addEventListener('visibilitychange', onVisibility)

    return () => {
      window.cancelAnimationFrame(raf)
      window.removeEventListener('resize', onResize)
      document.removeEventListener('visibilitychange', onVisibility)
    }
  }, [selector])

  return (
    <canvas
      ref={ref}
      aria-hidden="true"
      className="pointer-events-none fixed inset-0 z-[6]"
    />
  )
}

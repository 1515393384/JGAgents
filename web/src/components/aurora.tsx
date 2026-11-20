import { Suspense, lazy, useEffect, useState } from 'react'

/** 着色器单独切一个 chunk：首屏不下载，等浏览器空闲再挂。 */
const GrainGradient = lazy(() =>
  import('@paper-design/shaders-react').then((m) => ({ default: m.GrainGradient })),
)

/* 底层：三团径向渐变。亮暗两套 rgba —— 同一份形状，两种观感。 */
const BG_LIGHT = [
  'radial-gradient(1200px 800px at 20% 15%, rgba(34,211,238,.25), transparent 60%)',
  'radial-gradient(1000px 700px at 70% 25%, rgba(167,139,250,.22), transparent 55%)',
  'radial-gradient(900px 700px at 55% 70%, rgba(249,168,212,.18), transparent 60%)',
].join(',')

const BG_DARK = [
  'radial-gradient(1200px 800px at 20% 15%, rgba(6,182,212,.28), transparent 60%)',
  'radial-gradient(1000px 700px at 70% 25%, rgba(139,92,246,.24), transparent 55%)',
  'radial-gradient(900px 700px at 55% 70%, rgba(236,72,153,.18), transparent 60%)',
].join(',')

/* 上层：着色器配色，与参考站一致 */
const COLORS_LIGHT = ['#7CC4F5', '#93C5FD', '#C4B5FD', '#DBEAFE20']
const COLORS_DARK = ['#0E72BC', '#3B82F6', '#8B5CF6', '#1E3A8A00']

type Tier = {
  delayMs: number
  idleTimeoutMs: number
  intensity: number
  noise: number
  animate: boolean
}

/** 满配：独立显卡/高性能桌面 */
const TIER_FULL: Tier = { delayMs: 250, idleTimeoutMs: 1000, intensity: 0.9, noise: 0.5, animate: true }
/** 降配：小屏 / 低内存 / 少核 */
const TIER_MID: Tier = { delayMs: 550, idleTimeoutMs: 1600, intensity: 0.7, noise: 0.28, animate: true }
/** 关闭：用户要求减少动效，只留静态渐变 */
const TIER_OFF: Tier = { delayMs: 0, idleTimeoutMs: 0, intensity: 0.55, noise: 0.22, animate: false }

function pickTier(): Tier {
  if (typeof window === 'undefined') return TIER_OFF
  if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) return TIER_OFF

  const nav = navigator as Navigator & { deviceMemory?: number }
  const mem = nav.deviceMemory ?? 8
  const cores = nav.hardwareConcurrency ?? 8
  if (mem <= 4 || cores <= 4 || window.matchMedia('(max-width: 1023px)').matches) return TIER_MID
  return TIER_FULL
}

/**
 * 极光背景：静态渐变永远在，着色器（会流动的细颗粒）等空闲才上。
 * 不上着色器也不难看——底下的渐变就是完整的成品。
 */
export function Aurora({ theme }: { theme: 'light' | 'dark' }) {
  const [tier] = useState<Tier>(pickTier)
  const [ready, setReady] = useState(false)

  useEffect(() => {
    if (!tier.animate) return

    let idleId = 0
    let cancelled = false
    const mount = () => {
      if (!cancelled) setReady(true)
    }

    const timer = window.setTimeout(() => {
      if (cancelled) return
      const ric = window.requestIdleCallback?.bind(window)
      if (ric) idleId = ric(mount, { timeout: tier.idleTimeoutMs })
      else mount()
    }, tier.delayMs)

    return () => {
      cancelled = true
      window.clearTimeout(timer)
      window.cancelIdleCallback?.(idleId)
    }
  }, [tier])

  const dark = theme === 'dark'

  return (
    <div className="pointer-events-none absolute inset-0 overflow-hidden">
      <div className="absolute inset-0" style={{ background: dark ? BG_DARK : BG_LIGHT }} />

      <div
        className="absolute inset-0 transition-opacity duration-700"
        style={{ opacity: ready ? 1 : 0 }}
      >
        {ready ? (
          <Suspense fallback={null}>
            <GrainGradient
              className="size-full"
              colors={dark ? COLORS_DARK : COLORS_LIGHT}
              colorBack="#00000000"
              softness={1}
              intensity={tier.intensity}
              noise={tier.noise}
              shape="corners"
            />
          </Suspense>
        ) : null}
      </div>
    </div>
  )
}

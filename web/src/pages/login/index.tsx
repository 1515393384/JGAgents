import { useEffect, useRef, useState } from 'react'
import { Moon, Sun } from 'lucide-react'
import { planPrice, routeOptions } from '../../lib/route-copy'
import { PricingDialog } from '../../components/pricing-dialog'
import { Aurora } from '../../components/aurora'
import { MeteorField } from '../../components/meteor'
import { ChatPreview } from '../../components/chat-preview'
import { EdgeSweep } from '../../components/edge-sweep'
import { useTheme } from '../../app/theme'
import { AuthCard } from './auth-card'

export function LoginPage() {
  const { theme, toggle } = useTheme()
  const [pricing, setPricing] = useState(false)
  const layerRef = useRef<HTMLDivElement | null>(null)
  const gridRef = useRef<HTMLDivElement | null>(null)
  const panelRef = useRef<HTMLDivElement | null>(null)

  /* 方格纸挖洞：读出面板的真实矩形，用两层 mask + exclude 把中间那块抠掉。
     网格永远只在留白里出现，不会透过玻璃透到卡片上。 */
  useEffect(() => {
    const grid = gridRef.current
    const panel = panelRef.current
    const host = grid?.parentElement
    if (!grid || !panel || !host) return

    const apply = () => {
      const p = panel.getBoundingClientRect()
      const h = host.getBoundingClientRect()
      const GRID = 32

      const px0 = p.left - h.left
      const py0 = p.top - h.top
      const pw = p.width
      const ph = p.height

      /* 1) 把网格原点挪到卡片左上角的模数上 —— 卡片边线正好落在一条网格线上 */
      const gx = ((px0 % GRID) + GRID) % GRID
      const gy = ((py0 % GRID) + GRID) % GRID
      grid.style.backgroundPosition = `${gx}px ${gy}px`

      /* 2) 洞就是卡片本身：网格线一路顶到卡片边线为止，不在旁边留半截"刺"。
            尺寸向上补齐到整格，保证右/下两条洞边也压在网格线上 */
      const holeW = Math.ceil(pw / GRID) * GRID
      const holeH = Math.ceil(ph / GRID) * GRID

      grid.style.setProperty('--hole-x', `${px0.toFixed(2)}px`)
      grid.style.setProperty('--hole-y', `${py0.toFixed(2)}px`)
      grid.style.setProperty('--hole-w', `${holeW}px`)
      grid.style.setProperty('--hole-h', `${holeH}px`)

      /* 同一个洞也给点阵层用（变量写在共用的父层上，子层继承） */
      host.style.setProperty('--hole-x', `${px0.toFixed(2)}px`)
      host.style.setProperty('--hole-y', `${py0.toFixed(2)}px`)
      host.style.setProperty('--hole-w', `${holeW}px`)
      host.style.setProperty('--hole-h', `${holeH}px`)
    }

    apply()
    const ro = new ResizeObserver(apply)
    ro.observe(panel)
    window.addEventListener('resize', apply)
    return () => {
      ro.disconnect()
      window.removeEventListener('resize', apply)
    }
  }, [])

  /* 视差：只写 --par-x/--par-y 两个变量，光晕用 transform 跟，不碰布局。
     用 rAF 追一个缓动目标，指针停下来就自然收工。 */
  useEffect(() => {
    const el = layerRef.current
    if (!el) return
    if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) return
    if (window.matchMedia('(hover: none)').matches) return

    let raf = 0
    let tx = 0
    let ty = 0
    let cx = 0
    let cy = 0

    function tick() {
      raf = 0
      cx += (tx - cx) * 0.08
      cy += (ty - cy) * 0.08
      el?.style.setProperty('--par-x', `${cx.toFixed(2)}px`)
      el?.style.setProperty('--par-y', `${cy.toFixed(2)}px`)
      if (Math.abs(tx - cx) + Math.abs(ty - cy) > 0.2) raf = window.requestAnimationFrame(tick)
    }

    const onMove = (e: PointerEvent) => {
      tx = (e.clientX / window.innerWidth - 0.5) * 26
      ty = (e.clientY / window.innerHeight - 0.5) * 20
      if (!raf) raf = window.requestAnimationFrame(tick)
    }

    window.addEventListener('pointermove', onMove, { passive: true })
    return () => {
      window.removeEventListener('pointermove', onMove)
      if (raf) window.cancelAnimationFrame(raf)
    }
  }, [])

  return (
    <div className="relative h-full overflow-y-auto bg-bg">
      <MeteorField />
      {/* 空白背景的设计层：细点阵 + 两团极弱光晕，四周淡出。纯装饰，不接事件。 */}
      <div
        ref={layerRef}
        aria-hidden
        className="pointer-events-none absolute inset-0 overflow-hidden [--par-x:0px] [--par-y:0px]"
      >
        {/* 方格纸：按面板实际矩形挖洞，只在留白里出现，绝不透到卡片上 */}
        <div ref={gridRef} className="absolute inset-0 grid-paper" />
        {/* 点阵：外层按同一个洞裁掉，内层自己 transform 慢漂（合成器干活，不重绘） */}
        <div className="dots-cut absolute inset-0 opacity-40">
          <div className="dots dots-slow absolute -inset-4" />
        </div>

        {/* 四边扫光：只跑在面板之外的留白里 */}
        <EdgeSweep panelRef={panelRef} />
        {/* 三团撞色雾：冷（左上）/ 暖（右下）/ 紫纱（右上）。
            每团一个视差系数，前后拉开层次；亮暗两套 alpha 在 tokens 里。 */}
        <div className="absolute -top-40 -left-32 h-[520px] w-[520px] rounded-full bg-[radial-gradient(closest-side,var(--glow-cool),transparent)] blur-[120px] [transform:translate3d(calc(var(--par-x)*1.4),calc(var(--par-y)*1.4),0)]" />
        <div className="absolute -right-40 -bottom-48 h-[560px] w-[560px] rounded-full bg-[radial-gradient(closest-side,var(--glow-warm),transparent)] blur-[130px] [transform:translate3d(calc(var(--par-x)*2),calc(var(--par-y)*2),0)]" />
        <div className="absolute -top-28 right-6 h-[360px] w-[360px] rounded-full bg-[radial-gradient(closest-side,var(--glow-veil),transparent)] blur-[110px] [transform:translate3d(calc(var(--par-x)*-0.8),calc(var(--par-y)*-0.8),0)]" />

        {/* 四周暗角：把视线收进面板，空白边缘不空但也不抢 */}
        <div className="absolute inset-0 bg-[radial-gradient(120%_100%_at_50%_50%,transparent_52%,var(--edge-vignette)_100%)]" />
      </div>

      {/* 内容在页面里居中，面板仍是浮起来的一块，四周留白 */}
      <div className="relative z-[1] mx-auto flex min-h-full w-full max-w-[1200px] items-center px-4 py-6 md:px-6 md:py-10">
        <div ref={panelRef} className="relative w-full overflow-hidden rounded-card border border-border">
          <Aurora theme={theme} />

          {/* 右上：定价入口 + 主题按钮，浮在面板上，不占一行 */}
          <div className="absolute right-3 top-3 z-[3] flex items-center gap-1.5">
            <button
              type="button"
              onClick={() => setPricing(true)}
              className="inline-flex h-9 items-center rounded-pill border border-border bg-bg/40 px-3 text-xs font-medium text-fg-muted backdrop-blur-sm hover:border-brand/40 hover:text-fg"
            >
              定价
            </button>
            <button
              type="button"
              onClick={toggle}
              aria-label="切换主题"
              className="inline-flex h-9 w-9 items-center justify-center rounded-pill border border-border bg-bg/40 text-fg-muted backdrop-blur-sm hover:bg-bg-subtle hover:text-fg"
            >
              {theme === 'dark' ? <Sun size={15} /> : <Moon size={15} />}
            </button>
          </div>

          <div className="relative z-[2] grid w-full grid-cols-1 items-center justify-items-center gap-8 px-5 py-12 text-center md:min-h-[560px] md:p-12 lg:grid-cols-[minmax(0,1fr)_24rem] lg:justify-items-stretch lg:gap-16 lg:p-16 lg:text-left">
          {/* 左：通栏排版，不套卡片。窄屏居中堆叠，≥lg 才分栏 */}
          <div>
            <p className="mx-auto w-fit rounded-pill border border-brand/50 px-3 py-1.5 text-xs font-medium text-brand lg:mx-0">
              架阁 · 上下文不重算
            </p>

            <h1 className="mx-auto my-6 max-w-3xl text-3xl font-medium leading-[1.15] tracking-tight md:my-8 md:text-4xl xl:text-5xl lg:mx-0">
              一份标书，<span className="text-brand">几个人接着写</span>，不用互相复述。
            </h1>

            <p className="mx-auto max-w-xl text-[15px] leading-relaxed text-fg-muted md:text-lg lg:mx-0">
              改 Word、写标书、盯提醒，几个人同时在场也只有一个版本；谁打开都是最新那一份。
            </p>

            <p className="mx-auto mt-4 w-full max-w-xl border-l-2 border-brand pl-3 text-left text-[13px] font-medium leading-relaxed text-fg md:text-sm lg:mx-0">
              说过的话，只该存在一次。
            </p>

            {/* 价格条：登录页先给一眼价，点任一枚开定价弹窗 */}
            <div className="mt-5 flex flex-wrap items-center justify-center gap-1.5 lg:justify-start">
              {routeOptions.map((r) => (
                <button
                  key={r.id}
                  type="button"
                  onClick={() => setPricing(true)}
                  className="inline-flex items-center gap-1 rounded-pill border border-border bg-bg/30 px-2.5 py-1 text-[11px] text-fg-muted backdrop-blur-sm hover:border-brand/40 hover:text-fg"
                >
                  {r.tag}
                  <span className="font-medium text-fg">{planPrice[r.id].price}</span>
                </button>
              ))}
              <button
                type="button"
                onClick={() => setPricing(true)}
                className="px-1 text-[11px] text-brand hover:underline"
              >
                看全部档位
              </button>
            </div>

            <div className="mt-6 md:mt-7">
              <ChatPreview />
            </div>
          </div>

          {/* 右：登录卡，浮在极光上 */}
          <AuthCard />
          </div>
        </div>
      </div>

      <PricingDialog open={pricing} onClose={() => setPricing(false)} />
    </div>
  )
}


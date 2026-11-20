import { useEffect, useRef, type CSSProperties, type RefObject } from 'react'

/**
 * 边缘对撞：一个领头的在正中线上跑，两个跟班跟在它后面、左右各一（严格对称）。
 * 领头每到拐角撞一下、亮一下；跟班一开始落后一点，一路越拖越远，撞墙那一下被追回来 ——
 * 所以每撞一次，后面的就对齐前进一份。只跑在「视口边 ↔ 卡片」的留白里，层级在内容之下。
 */
const PATH = 'inset(var(--lane, 40px) round 20px)'

export function EdgeSweep({ panelRef }: { panelRef: RefObject<HTMLDivElement | null> }) {
  const boxRef = useRef<HTMLDivElement | null>(null)

  useEffect(() => {
    const box = boxRef.current
    const panel = panelRef.current
    if (!box || !panel) return

    const apply = () => {
      const p = panel.getBoundingClientRect()
      const vw = window.innerWidth
      const vh = window.innerHeight
      // 四边留白带各取一半 → 中线正好居中，左右对称
      const top = Math.max(12, p.top) / 2
      const right = Math.max(12, vw - (p.left + p.width)) / 2
      const bottom = Math.max(12, vh - (p.top + p.height)) / 2
      const left = Math.max(12, p.left) / 2

      box.style.setProperty(
        '--lane',
        `${top.toFixed(1)}px ${right.toFixed(1)}px ${bottom.toFixed(1)}px ${left.toFixed(1)}px`,
      )
    }

    apply()
    const ro = new ResizeObserver(apply)
    ro.observe(panel)
    window.addEventListener('resize', apply)
    return () => {
      ro.disconnect()
      window.removeEventListener('resize', apply)
    }
  }, [panelRef])

  return (
    <div
      ref={boxRef}
      aria-hidden
      className="pointer-events-none absolute inset-0 hidden overflow-hidden xl:block"
    >
      {/* 跟班：左 A、右 B。前半圈各走各的，半圈处一起追到领头身后，后半圈齐头并进 */}
      {[
        { offset: -7, anim: 'sweep-trail-a' },
        { offset: 7, anim: 'sweep-trail-b' },
      ].map(({ offset, anim }) => (
        <span
          key={anim}
          className="sweep-trail"
          style={
            {
              offsetPath: PATH,
              offsetRotate: 'auto 180deg',
              transform: `translateY(${offset}px)`,
              ['--anim' as string]: anim,
              ['--c' as string]: 'var(--strand)',
              ['--ch' as string]: 'var(--strand-hot)',
            } as CSSProperties
          }
        />
      ))}

      {/* 领头：最亮，撞墙闪一下 */}
      <span
        className="sweep-run"
        style={
          {
            offsetPath: PATH,
            offsetRotate: 'auto 180deg',
            ['--dir' as string]: 'reverse',
            ['--c' as string]: 'var(--strand)',
            ['--ch' as string]: 'var(--strand-hot)',
          } as CSSProperties
        }
      />
    </div>
  )
}

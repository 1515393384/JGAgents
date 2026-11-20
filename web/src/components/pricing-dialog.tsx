import { useEffect } from 'react'
import { Check, Minus, X } from 'lucide-react'
import { planGroups, planIncluded, planPrice, routeOptions } from '../lib/route-copy'
import type { PlanCell, PlanId } from '../lib/route-copy'
import { Button } from './ui'

const COLS = 'grid-cols-[minmax(0,1.05fr)_repeat(4,minmax(0,1fr))]'

/** 一格：true 打勾，false 划横杠，字符串写口径 */
function Cell({ value, hot }: { value: PlanCell; hot?: boolean }) {
  if (value === true)
    return <Check size={13} strokeWidth={2.5} className={hot ? 'text-brand' : 'text-ok'} />
  if (value === false) return <Minus size={13} className="text-fg-subtle/50" />
  return <span className="text-[11px] leading-4 text-fg-muted">{value}</span>
}

/** 定价弹窗：宽屏功能对照表（按能力分组），窄屏按档纵向列 */
export function PricingDialog({ open, onClose }: { open: boolean; onClose: () => void }) {
  useEffect(() => {
    if (!open) return
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose()
    }
    document.addEventListener('keydown', onKey)
    return () => document.removeEventListener('keydown', onKey)
  }, [open, onClose])

  if (!open) return null

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-[rgba(10,12,16,0.45)] px-3 py-6 backdrop-blur-[3px]"
      onClick={onClose}
      role="dialog"
      aria-modal="true"
      aria-label="定价"
    >
      <div
        data-pricing-dialog
        onClick={(e) => e.stopPropagation()}
        className="max-h-full w-full max-w-[1000px] overflow-y-auto rounded-card border border-border bg-bg-elevated p-4 shadow-card md:p-6"
      >
        <div className="flex items-start gap-3">
          <div>
            <h2 className="text-lg font-medium tracking-tight">方案与价格</h2>
            <p className="mt-1 text-xs text-fg-muted">
              免费版密钥由架阁托管，用户无需输入，以最小权限与审计记录约束；付费档密钥由用户自行保管，服务端不具备解密能力。全部档位均不读取内容，亦不用于模型训练。
            </p>
          </div>
          <button
            type="button"
            onClick={onClose}
            aria-label="关闭"
            className="ml-auto inline-flex h-7 w-7 shrink-0 items-center justify-center rounded-field text-fg-subtle hover:bg-bg-subtle hover:text-fg"
          >
            <X size={14} />
          </button>
        </div>

        {/* 宽屏：对照表 */}
        <div className="mt-4 hidden overflow-hidden rounded-field border border-border md:block">
          <div className={'grid ' + COLS}>
            {/* 表头：列 = 方案 */}
            <div className="border-b border-border bg-bg-subtle/40 px-3 py-2" />
            {routeOptions.map((r) => (
              <div
                key={r.id}
                className={
                  'border-b border-l border-border px-3 py-2.5 ' +
                  (r.id === 'svip' ? 'bg-brand-soft/50' : 'bg-bg-subtle/40')
                }
              >
                <div className="flex items-center gap-1">
                  <span className="text-[12px] font-medium text-fg">{r.title}</span>
                  {r.id === 'svip' ? (
                    <span className="rounded-pill bg-brand px-1.5 text-[9px] font-medium leading-4 text-brand-fg">
                      推荐
                    </span>
                  ) : null}
                </div>
                <div className="mt-1.5 flex items-baseline gap-1">
                  <span className="text-[18px] font-medium leading-none tracking-tight text-fg">
                    {planPrice[r.id].price}
                  </span>
                  <span className="text-[9.5px] text-fg-subtle">{planPrice[r.id].unit}</span>
                </div>
                <div className="mt-1 text-[10px] leading-4 text-fg-subtle">{planPrice[r.id].head}</div>
              </div>
            ))}

            {/* 分组 + 行 */}
            {planGroups.map((g) => (
              <div key={g.group} className="contents">
                <div className="col-span-5 border-b border-border bg-bg-subtle/50 px-3 py-1.5">
                  <span className="text-[10px] font-medium tracking-wide text-fg-subtle">{g.group}</span>
                </div>
                {g.rows.map((row, i) => (
                  <div key={row.label} className="contents">
                    <div
                      className={
                        'px-3 py-2 text-[11px] text-fg-muted ' +
                        (i > 0 ? 'border-t border-border' : '')
                      }
                    >
                      {row.label}
                    </div>
                    {routeOptions.map((r) => (
                      <div
                        key={r.id}
                        className={
                          'flex items-center border-l border-border px-3 py-2 ' +
                          (i > 0 ? 'border-t' : '') +
                          (r.id === 'svip' ? ' bg-brand-soft/30' : '')
                        }
                      >
                        <Cell value={row.cells[r.id as PlanId]} hot={r.id === 'svip'} />
                      </div>
                    ))}
                  </div>
                ))}
              </div>
            ))}
          </div>
        </div>

        {/* 窄屏：一档一块，按分组列有口径的项 */}
        <div className="mt-4 space-y-2 md:hidden">
          {routeOptions.map((r) => (
            <div
              key={r.id}
              className={
                'rounded-field border px-3 py-2.5 ' +
                (r.id === 'svip' ? 'border-brand/50 bg-brand-soft/40' : 'border-border')
              }
            >
              <div className="flex items-baseline gap-1.5">
                <span className="text-[12.5px] font-medium text-fg">{r.title}</span>
                <span className="text-[10px] text-fg-subtle">{r.tag}</span>
                <span className="ml-auto text-[15px] font-medium text-fg">
                  {planPrice[r.id].price}
                </span>
                <span className="text-[9.5px] text-fg-subtle">{planPrice[r.id].unit}</span>
              </div>
              {planGroups.map((g) => {
                const visible = g.rows.filter((row) => row.cells[r.id as PlanId] !== false)
                if (visible.length === 0) return null
                return (
                  <div key={g.group} className="mt-2">
                    <div className="text-[9.5px] tracking-wide text-fg-subtle">{g.group}</div>
                    <ul className="mt-1 space-y-1">
                      {visible.map((row) => {
                        const v = row.cells[r.id as PlanId]
                        return (
                          <li key={row.label} className="flex items-start gap-1.5 text-[10.5px] leading-4">
                            <Check
                              size={11}
                              strokeWidth={2.5}
                              className={
                                'mt-[2px] shrink-0 ' + (r.id === 'svip' ? 'text-brand' : 'text-ok')
                              }
                            />
                            <span className="text-fg-muted">
                              {row.label}
                              {typeof v === 'string' ? (
                                <span className="text-fg-subtle"> · {v}</span>
                              ) : null}
                            </span>
                          </li>
                        )
                      })}
                    </ul>
                  </div>
                )
              })}
            </div>
          ))}
        </div>

        {/* 全档一致：安全底线不当卖点，单独一条横条 */}
        <div
          className="mt-3 rounded-field border border-border bg-bg-subtle/50 px-3 py-2.5"
          data-included
        >
          <div className="text-[10px] font-medium tracking-wide text-fg-subtle">
            全部档位共同承诺
          </div>
          <ul className="mt-1 grid gap-1 md:grid-cols-2">
            {planIncluded.map((line) => (
              <li key={line} className="flex items-start gap-1.5 text-[10.5px] leading-4 text-fg-muted">
                <Check size={11} strokeWidth={2.5} className="mt-[2px] shrink-0 text-ok" />
                <span>{line}</span>
              </li>
            ))}
          </ul>
        </div>

        <p className="mt-3 text-[10.5px] leading-[1.7] text-fg-subtle">
          按席位月付，不按 token 抽成；模型用量按成本价直出；可按订单开具发票。以上为示意价，最终以合同与账单为准；企业私有部署另议。
        </p>

        <Button onClick={onClose} pill size="lg" variant="outline" className="mt-4 w-full">
          返回
        </Button>
      </div>
    </div>
  )
}

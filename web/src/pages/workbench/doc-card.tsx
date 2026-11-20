import { useState } from 'react'
import { ArrowUp, FileText, Lock } from 'lucide-react'
import { MicroWindow } from '../../components/micro-window'
import { Avatar } from '../../components/ui'
import { docParagraphs } from '../../lib/mock'

/** 文档微应用：盖在会话之上，不挤压会话宽度。
 *  文档随时可编辑，diff 交给版本记录，"锁"只当状态提示，不当闸门。
 *  窗口的一切行为（拖、缩、全屏、关）都由 MicroWindow 统一管。 */
export function DocCard({ open, onClose }: { open: boolean; onClose: () => void }) {
  const [chatOpen, setChatOpen] = useState(false)
  const [draft, setDraft] = useState('')

  return (
    <MicroWindow
      open={open}
      onClose={onClose}
      icon={<FileText size={13} />}
      title="技术标_甲方案_v3.docx"
      subtitle="v12 → v15"
      status={
        <>
          <span className="hidden shrink-0 items-center gap-1 rounded-pill bg-brand-soft px-1.5 py-0.5 text-[10.5px] text-brand sm:flex">
            <Lock size={9} />
            阁吏 正在改 3 段
          </span>
          <span className="hidden shrink-0 font-mono text-[10.5px] text-fg-subtle sm:inline">2/5</span>
        </>
      }
      footer={
        <div
          data-probe="chatline"
          className="relative shrink-0 border-t border-surface-border bg-surface"
        >
          <div
            className={
              'absolute bottom-full left-0 right-0 overflow-hidden border-t border-surface-border bg-surface shadow-[0_-8px_20px_-12px_rgba(0,0,0,0.28)] transition-[max-height,opacity] duration-200 ' +
              (chatOpen ? 'max-h-40 opacity-100' : 'max-h-0 opacity-0')
            }
          >
            <div className="space-y-2 px-3 py-2.5 text-[12.5px] leading-6">
              <p className="text-fg-muted">
                <span className="mr-1.5 text-[11px] text-fg-subtle">你 09:41</span>
                3.4 这段别把班次下调写死，加个"需甲方书面确认"。
              </p>
              <p className="text-fg-muted">
                <span className="mr-1.5 text-[11px] text-brand">阁吏 09:41</span>
                改着呢。这句补在段尾，改完 3.5 一并给你看，前缀没动。
              </p>
            </div>
          </div>

          <form
            onSubmit={(e) => {
              e.preventDefault()
              setDraft('')
            }}
            className="flex h-11 items-center gap-2 pl-2.5 pr-7"
          >
            <Avatar name="阁" size={20} />
            <span className="hidden shrink-0 font-mono text-[10.5px] text-fg-subtle sm:inline">
              3.4 · 2/5
            </span>
            <input
              value={draft}
              onChange={(e) => setDraft(e.target.value)}
              onFocus={() => setChatOpen(true)}
              onBlur={() => setChatOpen(false)}
              placeholder="跟阁吏说一句…"
              aria-label="跟阁吏说一句"
              className="min-w-0 flex-1 bg-transparent text-[12.5px] text-fg outline-none placeholder:text-fg-subtle"
            />
            <button
              type="submit"
              disabled={!draft.trim()}
              aria-label="发送"
              className="inline-flex h-7 w-7 items-center justify-center rounded-pill bg-brand text-brand-fg transition-opacity disabled:opacity-30"
            >
              <ArrowUp size={14} />
            </button>
          </form>
        </div>
      }
    >
      {/* 进度条：阁吏改到第几处 */}
      <div className="h-0.5 w-full shrink-0 bg-bg-subtle">
        <div className="h-full w-[40%] bg-brand transition-all duration-300" />
      </div>

      {/* 纸张：默认就是编辑态，光标、选区都正常，改动走版本记录 */}
      <div className="min-h-0 flex-1 overflow-y-auto bg-surface-sunken px-3 py-4 md:px-6">
        <div className="mx-auto max-w-[640px] rounded-field bg-surface px-5 py-6 shadow-card md:px-9 md:py-8">
          <h2 className="text-[15px] font-medium tracking-tight">第三章 运维服务方案</h2>
          <p className="mt-3 text-[13.5px] leading-7 text-fg-muted">
            本章描述运维服务的响应机制、值班安排与故障处理流程，供评标委员会审阅。
          </p>

          <div className="mt-4 space-y-3">
            {docParagraphs.map((p) => {
              const cursor = p.state === 'editing'
              return (
                <div key={p.id} className="relative">
                  <div className="mb-1 flex items-center gap-1.5 text-[10.5px] text-fg-subtle">
                    <span className="font-mono">{p.id}</span>
                    {p.state === 'changed' ? (
                      <span className="rounded-pill bg-warn/10 px-1.5 text-warn">已改</span>
                    ) : null}
                    {cursor ? (
                      <span className="rounded-pill bg-brand-soft px-1.5 text-brand">正在改</span>
                    ) : null}
                  </div>

                  <div
                    className={
                      'relative overflow-hidden rounded-[6px] px-2 py-1.5 text-[13.5px] leading-7 ' +
                      (cursor
                        ? 'bg-brand-soft/60 text-fg'
                        : p.state === 'changed'
                          ? 'bg-warn/[0.07] text-fg'
                          : 'text-fg-muted')
                    }
                  >
                    {p.text}
                    {cursor ? <span className="scan-sweep absolute inset-0" /> : null}
                  </div>
                </div>
              )
            })}
          </div>
        </div>
      </div>
    </MicroWindow>
  )
}

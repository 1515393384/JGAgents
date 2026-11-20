import { ArrowUp, AtSign, ChevronDown, Paperclip, Sparkles, Square } from 'lucide-react'

const toolBtn =
  'inline-flex h-8 items-center gap-1 rounded-pill px-2 text-[12px] text-fg-muted hover:bg-bg-subtle hover:text-fg'

export function Composer() {
  return (
    <div className="shrink-0 px-3 pb-[max(0.75rem,env(safe-area-inset-bottom))] pt-1 md:px-6 md:pb-4">
      <div className="mx-auto w-full max-w-[760px]">
        {/* 全页只留这一张浮起的卡：输入框是唯一天然该"浮"的东西 */}
        <div className="rounded-panel border border-surface-border bg-surface p-2 shadow-card focus-within:border-border-strong">
          <textarea
            rows={1}
            placeholder="说说要改哪一节，或者直接丢个文件进来"
            className="max-h-40 w-full resize-none bg-transparent px-2.5 py-1.5 text-[13.5px] leading-6 outline-none placeholder:text-fg-subtle"
          />

          <div className="mt-1 flex items-center gap-1">
            <button type="button" className={toolBtn} aria-label="附件">
              <Paperclip size={14} />
            </button>
            <button type="button" className={toolBtn} aria-label="提及参与人">
              <AtSign size={14} />
            </button>
            <button type="button" className={toolBtn}>
              <Sparkles size={13} className="text-brand" />
              阁吏
              <ChevronDown size={12} className="text-fg-subtle" />
            </button>

            <span className="ml-auto flex items-center gap-1.5 pl-2">
              <button
                type="button"
                aria-label="停止"
                className="inline-flex h-8 w-8 items-center justify-center rounded-pill text-fg-subtle hover:bg-bg-subtle hover:text-fg"
              >
                <Square size={12} />
              </button>
              <button
                type="button"
                aria-label="发送"
                className="inline-flex h-8 w-8 items-center justify-center rounded-pill bg-brand text-brand-fg transition-colors hover:bg-brand-hover"
              >
                <ArrowUp size={16} />
              </button>
            </span>
          </div>
        </div>

        <div className="mt-1.5 hidden items-center justify-center gap-1 text-[11px] text-fg-subtle sm:flex">
          Enter 发送 · Shift+Enter 换行
        </div>
      </div>
    </div>
  )
}

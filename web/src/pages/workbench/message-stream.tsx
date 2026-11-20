import { CalendarClock, FileText, Lock, Menu, MoreHorizontal, Terminal } from 'lucide-react'
import { Avatar } from '../../components/ui'
import { docChange, members, messages, threadOf, type MsgRow, type Space } from '../../lib/mock'
import { DiffFold, DiffRevisions, DiffSheet } from './doc-diff'

/** 工具与改动都不做成卡片：左边一条细线加等宽小字就够了，
 * 卡片一多，正文就被切成一片片白块，读起来像表格。 */
function Aside({ children }: { children: React.ReactNode }) {
  return (
    <div className="mt-2.5 border-l-2 border-surface-border pl-2.5 font-mono text-[11.5px] leading-6 text-fg-subtle">
      {children}
    </div>
  )
}

function ToolLine({ tool }: { tool: NonNullable<MsgRow['tool']> }) {
  return (
    <Aside>
      <span className="inline-flex items-center gap-1.5">
        <Terminal size={11} className="shrink-0" />
        <span className="text-fg-muted">{tool.name}</span>
        <span className="truncate">{tool.args}</span>
        <span className="text-fg-subtle">· {tool.state === 'run' ? '执行中' : '完成'}</span>
      </span>
    </Aside>
  )
}

function DiffLine({ diff }: { diff: NonNullable<MsgRow['diff']> }) {
  return (
    <Aside>
      <span className="inline-flex items-center gap-1.5">
        <FileText size={11} className="shrink-0" />
        <span className="truncate text-fg-muted">{diff.file}</span>
        <span className="text-ok">+{diff.add}</span>
        <span className="text-danger">−{diff.del}</span>
      </span>
    </Aside>
  )
}

export function MessageStream({ space, onOpenNav }: { space: Space; onOpenNav: () => void }) {
  const shared = space.kind === 'team'
  const thread = threadOf(space.id)
  return (
    <div className="flex min-h-0 flex-1 flex-col">
      {/* 会话头：只留一条发丝线，不抢正文 */}
      <div className="sticky top-0 z-20 shrink-0 border-b border-surface-border bg-bg/85 backdrop-blur-xl">
        <div className="flex h-13 items-center gap-2 px-3 py-2.5 md:px-5">
          <button
            type="button"
            onClick={onOpenNav}
            aria-label="打开列表"
            className="-ml-1 inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-field text-fg-muted hover:bg-bg-subtle hover:text-fg lg:hidden"
          >
            <Menu size={17} />
          </button>

          <span className="min-w-0">
            <span className="block truncate text-[13.5px] font-medium leading-tight">{thread.title}</span>
            <span className="mt-0.5 flex items-center gap-1.5 text-[11px] text-fg-subtle">
              <FileText size={10} className="shrink-0" />
              <span className="truncate">{thread.doc}</span>
              <span className="hidden md:inline">· {thread.deadline}</span>
            </span>
          </span>

          <span className="ml-auto flex shrink-0 items-center gap-2.5">
            {shared ? (
              <>
                <span className="hidden items-center gap-1 text-[11px] text-fg-subtle md:flex">
                  <CalendarClock size={12} />
                  2 人在场
                </span>
                <span className="flex -space-x-1.5">
                  {members.map((m) => (
                    <Avatar key={m.name} name={m.name} size={22} tone="muted" ring />
                  ))}
                </span>
              </>
            ) : (
              <span className="flex items-center gap-1 text-[11px] text-fg-subtle">
                <Lock size={11} />
                仅自己可见
              </span>
            )}
            <button
              type="button"
              aria-label="更多"
              className="inline-flex h-8 w-8 items-center justify-center rounded-field text-fg-muted hover:bg-bg-subtle hover:text-fg"
            >
              <MoreHorizontal size={16} />
            </button>
          </span>
        </div>
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto">
        <div className="mx-auto w-full max-w-[760px] px-4 py-6 md:px-6 md:py-9">
          <div className="mb-6 flex items-center gap-3">
            <span className="h-px flex-1 bg-surface-border" />
            <span className="text-[11px] text-fg-subtle">今天 · 11 月 20 日</span>
            <span className="h-px flex-1 bg-surface-border" />
          </div>

          <div className="space-y-7 md:space-y-8">
            {messages.map((m) =>
              m.who === 'you' ? (
                /* 自己：右对齐一条浅底，不做成卡片（卡片会跟助手消息抢层级） */
                <div key={m.id} className="flex flex-col items-end">
                  <div className="mb-1.5 flex items-center gap-1.5 text-[11px] text-fg-subtle">
                    <span>{m.at}</span>
                    <span className="text-[12.5px] font-medium text-fg-muted">{m.name}</span>
                  </div>
                  <div className="max-w-[86%] rounded-panel rounded-tr-field bg-bg-subtle px-3.5 py-2.5 text-[13.5px] leading-7 md:max-w-[76%]">
                    <p className="whitespace-pre-wrap">{m.text}</p>
                  </div>
                </div>
              ) : (
                <div key={m.id} className="flex gap-2.5 md:gap-3">
                  <Avatar
                    name={m.who === 'assistant' ? '阁' : m.name}
                    size={24}
                    tone={m.who === 'assistant' ? 'brand' : 'muted'}
                  />
                  <div className="min-w-0 flex-1">
                    <div className="flex items-baseline gap-1.5">
                      <span className="text-[12.5px] font-medium">
                        {m.who === 'assistant' ? '阁吏' : m.name}
                      </span>
                      <span className="text-[11px] text-fg-subtle">{m.at}</span>
                    </div>
                    <p className="mt-1.5 whitespace-pre-wrap text-[13.5px] leading-7 text-fg">{m.text}</p>
                    {m.tool ? <ToolLine tool={m.tool} /> : null}
                    {m.diff ? <DiffLine diff={m.diff} /> : null}
                  </div>
                </div>
              ),
            )}
          </div>

          <div className="mt-9 text-[11px] text-fg-subtle">三人看到的是同一份，前缀没重算</div>

          {/* 临时对比区：三种"AI 改文档"的画法，挑完删掉这一段 */}
          <div className="mt-8 space-y-5 rounded-panel border border-dashed border-surface-border p-3.5">
            <div className="text-[11px] text-fg-subtle">AI 改文档 / 表格 · 三种画法，挑一个</div>
            <div>
              <div className="mb-1 text-[11px] text-brand">A · 折叠条：默认只占一行，点开看逐条改动</div>
              <DiffFold d={docChange} />
            </div>
            <div>
              <div className="mb-1 text-[11px] text-brand">B · 修订对：原文划掉、新文跟在下面（像 Word 比较）</div>
              <DiffRevisions d={docChange} />
            </div>
            <div>
              <div className="mb-1 text-[11px] text-brand">C · 单元格：表格里改了哪几格，旧值划掉新值跟上</div>
              <DiffSheet d={docChange} />
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}

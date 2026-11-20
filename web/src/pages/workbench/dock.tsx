import { useState } from 'react'
import {
  Bell,
  CalendarDays,
  Eye,
  EyeOff,
  FileSpreadsheet,
  FileText,
  HardDrive,
  NotebookPen,
  Plus,
} from 'lucide-react'
import { Avatar } from '../../components/ui'

const templates = [
  { id: 't1', label: '标书', icon: FileText },
  { id: 't2', label: '周报', icon: CalendarDays },
  { id: 't3', label: '纪要', icon: NotebookPen },
  { id: 't4', label: '表格', icon: FileSpreadsheet },
  { id: 't5', label: '提醒', icon: Bell },
] as const

/** 右边缘的 dock：默认只剩一条发光细边，鼠标靠过去才展开。
 *  左边留给文件列表，右边是状态与工具，正文一点不被占。 */
export function Dock({
  onOpenDoc,
  onOpenRoot,
  onOpenSpace,
  onOpenTemplate,
}: {
  onOpenDoc: () => void
  onOpenRoot: () => void
  onOpenSpace: () => void
  onOpenTemplate: (label: string) => void
}) {
  const [track, setTrack] = useState(true)

  return (
    <div
      data-probe="dock"
      className="group/dock fixed inset-y-0 right-0 z-50 flex flex-row-reverse"
    >
      {/* 触发带：一条可见细边 + 一段呼吸光，代表 AI 正在动 */}
      <div className="relative w-2.5 shrink-0">
        <span className="absolute inset-y-0 right-0 w-px bg-surface-border/70" />
        <span className="dock-breathe absolute right-0 top-1/2 h-28 w-[2px] -translate-y-1/2 rounded-pill bg-brand/80" />
      </div>

      <aside className="w-0 overflow-hidden border-l border-surface-border bg-surface-sunken/95 backdrop-blur-xl transition-[width] duration-200 ease-out group-hover/dock:w-[56px]">
        <div className="flex w-14 flex-col items-center gap-1 py-3 opacity-0 transition-opacity duration-200 group-hover/dock:opacity-100">
          <button
            type="button"
            onClick={onOpenSpace}
            className="mb-1"
            aria-label="空间设置与新建空间"
            title="空间 · 新建 / 设置"
          >
            <Avatar name="售" size={28} />
          </button>

          <span className="my-1 h-px w-6 bg-surface-border" />

          {/* 根目录：不是 APP，是这个空间的家底，管上传与增删改查 */}
          <button
            type="button"
            onClick={onOpenRoot}
            aria-label="空间根目录"
            title="空间根目录 · 上传 / 改名 / 删除"
            className="inline-flex h-9 w-9 items-center justify-center rounded-field text-fg-muted transition-colors hover:bg-bg-subtle hover:text-fg"
          >
            <HardDrive size={15} />
          </button>

          <span className="my-1 h-px w-6 bg-surface-border" />

          {/* 当前文件：外圈呼吸环 = 阁吏正在改 */}
          <button
            type="button"
            onClick={onOpenDoc}
            aria-label="当前文件"
            title="当前文件 · 阁吏在改"
            className="relative inline-flex h-9 w-9 items-center justify-center rounded-field bg-brand-soft text-brand ring-1 ring-brand/45"
          >
            <FileText size={15} />
            <span className="dock-ring absolute inset-0 rounded-field" />
          </button>

          <span className="my-1 h-px w-6 bg-surface-border" />

          {templates.map((t) => {
            const Icon = t.icon
            return (
              <button
                key={t.id}
                type="button"
                onClick={() => onOpenTemplate(t.label)}
                title={`新建${t.label}`}
                aria-label={`新建${t.label}`}
                className="inline-flex h-9 w-9 items-center justify-center rounded-field text-fg-subtle transition-colors hover:bg-bg-subtle hover:text-fg"
              >
                <Icon size={15} />
              </button>
            )
          })}

          <button
            type="button"
            aria-label="更多"
            className="mt-1 inline-flex h-9 w-9 items-center justify-center rounded-field border border-dashed border-surface-border text-fg-subtle hover:text-fg"
          >
            <Plus size={15} />
          </button>

          <button
            type="button"
            onClick={() => setTrack((v) => !v)}
            aria-label={track ? '关闭自动跟随' : '开启自动跟随'}
            title={track ? '自动跟随 AI 改动位置' : '不自动跟随'}
            className={
              'mt-auto inline-flex h-9 w-9 items-center justify-center rounded-field ' +
              (track ? 'text-brand' : 'text-fg-subtle hover:text-fg')
            }
          >
            {track ? <Eye size={15} /> : <EyeOff size={15} />}
          </button>
        </div>
      </aside>
    </div>
  )
}

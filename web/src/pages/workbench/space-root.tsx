import {
  ArrowUpFromLine,
  FileCode2,
  FileSpreadsheet,
  FileText,
  Folder,
  FolderPlus,
  HardDrive,
  MoreHorizontal,
  Search,
} from 'lucide-react'
import { MicroWindow } from '../../components/micro-window'
import { rootFiles } from '../../lib/mock'

const iconOf = {
  doc: FileText,
  sheet: FileSpreadsheet,
  folder: Folder,
  code: FileCode2,
} as const

/**
 * 空间根目录：一个空间的全部家底都在这儿。
 * 它和 dock 上的"程序·模板"是两件事——模板是起点，根目录是仓库；
 * 这里不绑定任何单个文件，上传、改名、复制、删除都在这一层做。
 */
export function SpaceRoot({ open, onClose }: { open: boolean; onClose: () => void }) {
  return (
    <MicroWindow
      open={open}
      onClose={onClose}
      icon={<HardDrive size={13} />}
      title="我的空间"
      subtitle="根目录"
      status={
        <span className="hidden shrink-0 font-mono text-[10.5px] text-fg-subtle sm:inline">
          {rootFiles.length} 项
        </span>
      }
      footer={
        <div className="flex h-10 items-center gap-3 border-t border-surface-border px-3 text-[10.5px] text-fg-subtle">
          <span>勾选后可批量删除或移动</span>
          <span className="ml-auto font-mono">配额 1.8 / 5 GB</span>
        </div>
      }
    >
      {/* 工具条：上传、新建文件夹，作用域永远是当前这一层 */}
      <div className="flex shrink-0 flex-wrap items-center gap-2 border-b border-surface-border px-2.5 py-2">
        <button
          type="button"
          className="inline-flex h-7 items-center gap-1.5 rounded-field bg-brand px-2.5 text-[11.5px] font-medium text-brand-fg hover:bg-brand-hover"
        >
          <ArrowUpFromLine size={12} />
          上传
        </button>
        <button
          type="button"
          className="inline-flex h-7 items-center gap-1.5 rounded-field border border-surface-border px-2.5 text-[11.5px] text-fg-muted hover:bg-bg-subtle hover:text-fg"
        >
          <FolderPlus size={12} />
          新建文件夹
        </button>

        <div className="ml-auto flex h-7 items-center gap-1.5 rounded-field bg-bg-subtle px-2 text-fg-subtle">
          <Search size={12} />
          <input
            aria-label="在根目录里找"
            placeholder="在根目录里找…"
            className="w-24 bg-transparent text-[11.5px] text-fg outline-none placeholder:text-fg-subtle sm:w-40"
          />
        </div>
      </div>

      {/* 文件表：名称 / 大小 / 更新时间 / 谁在用 */}
      <div className="min-h-0 flex-1 overflow-y-auto">
        <table className="w-full table-fixed border-collapse text-[12.5px]">
          <thead className="sticky top-0 z-10 bg-surface-sunken/95 backdrop-blur">
            <tr className="text-left text-[10.5px] font-normal text-fg-subtle">
              <th className="w-8 py-1.5 pl-3" />
              <th className="py-1.5">名称</th>
              <th className="hidden w-20 py-1.5 sm:table-cell">大小</th>
              <th className="hidden w-28 py-1.5 md:table-cell">更新时间</th>
              <th className="w-24 py-1.5">谁在用</th>
              <th className="w-8 py-1.5" />
            </tr>
          </thead>
          <tbody>
            {rootFiles.map((f) => {
              const Icon = iconOf[f.kind]
              return (
                <tr
                  key={f.id}
                  className="group/row border-t border-surface-border/60 hover:bg-bg-subtle/60"
                >
                  <td className="py-2 pl-3 align-middle">
                    <input
                      type="checkbox"
                      aria-label={`选中${f.name}`}
                      className="h-3 w-3 accent-[var(--brand)]"
                    />
                  </td>
                  <td className="min-w-0 py-2 pr-2 align-middle">
                    <span className="flex min-w-0 items-center gap-1.5">
                      <Icon size={13} className="shrink-0 text-fg-subtle" />
                      <span className="truncate text-fg">{f.name}</span>
                    </span>
                  </td>
                  <td className="hidden py-2 font-mono text-[11px] text-fg-subtle sm:table-cell">
                    {f.size}
                  </td>
                  <td className="hidden py-2 text-[11px] text-fg-subtle md:table-cell">{f.at}</td>
                  <td className="py-2 text-[11px] text-fg-muted">{f.who}</td>
                  <td className="py-2 pr-2 text-right">
                    <button
                      type="button"
                      aria-label={`${f.name} 更多操作`}
                      className="inline-flex h-6 w-6 items-center justify-center rounded-field text-fg-subtle opacity-0 transition-opacity hover:bg-bg-subtle hover:text-fg group-hover/row:opacity-100"
                    >
                      <MoreHorizontal size={13} />
                    </button>
                  </td>
                </tr>
              )
            })}
          </tbody>
        </table>

        <p className="px-3 py-3 text-[10.5px] leading-5 text-fg-subtle">
          拖文件到窗口任意位置也能上传，落到当前这一层。文件夹只归类，不影响上下文缓存。
        </p>
      </div>
    </MicroWindow>
  )
}

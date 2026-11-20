import { useState } from 'react'
import { ChevronRight, FileSpreadsheet, FileText } from 'lucide-react'

export type Fold = { removed: string; added: string }

export type CellChange = { cell: string; before: string; after: string }

export type DocChange = {
  file: string
  kind: 'docx' | 'xlsx'
  add: number
  del: number
  summary: string
  /** A 折叠：逐条改动摘要 */
  folds: Fold[]
  /** B 修订：成对的原文/新文 */
  pairs: { before: string; after: string }[]
  /** C 表格：单元格级 */
  sheet: { name: string; cells: CellChange[] }
}

/** A · 折叠条：默认只占一行，点开看逐条改动。
 *  适合"改了一堆但你只想确认他没乱动"。 */
export function DiffFold({ d }: { d: DocChange }) {
  const [open, setOpen] = useState(false)
  const Icon = d.kind === 'xlsx' ? FileSpreadsheet : FileText

  return (
    <div className="mt-2.5 overflow-hidden rounded-field border border-surface-border bg-surface-sunken">
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        className="flex w-full items-center gap-2 px-2.5 py-2 text-left text-[11.5px] text-fg-muted hover:bg-bg-subtle/50"
      >
        <ChevronRight
          size={12}
          className={'shrink-0 text-fg-subtle transition-transform ' + (open ? 'rotate-90' : '')}
        />
        <Icon size={12} className="shrink-0 text-fg-subtle" />
        <span className="truncate">{d.file}</span>
        <span className="shrink-0 font-mono text-ok">+{d.add}</span>
        <span className="shrink-0 font-mono text-danger">−{d.del}</span>
      </button>

      {open ? (
        <div className="space-y-1.5 border-t border-surface-border px-2.5 py-2">
          {d.folds.map((f, i) => (
            <div key={i} className="text-[12px] leading-5">
              <div className="text-fg-subtle">
                <span className="mr-1 font-mono text-danger">−</span>
                <span className="line-through decoration-danger/40">{f.removed}</span>
              </div>
              <div className="text-fg-muted">
                <span className="mr-1 font-mono text-ok">+</span>
                {f.added}
              </div>
            </div>
          ))}
        </div>
      ) : null}
    </div>
  )
}

/** B · 修订对：原文一行、新文一行，像 Word 的"比较并合并"。
 *  适合"只改了两三处，要逐字看清"。 */
export function DiffRevisions({ d }: { d: DocChange }) {
  return (
    <div className="mt-2.5 space-y-2 rounded-field border border-surface-border bg-surface-sunken p-2.5">
      {d.pairs.map((p, i) => (
        <div key={i} className="space-y-1 text-[12.5px] leading-6">
          <div className="flex gap-2">
            <span className="mt-0.5 w-3 shrink-0 text-center font-mono text-[11px] text-danger">
              −
            </span>
            <span className="border-l-2 border-danger/40 pl-2 text-fg-subtle line-through decoration-danger/30">
              {p.before}
            </span>
          </div>
          <div className="flex gap-2">
            <span className="mt-0.5 w-3 shrink-0 text-center font-mono text-[11px] text-ok">+</span>
            <span className="border-l-2 border-ok/40 pl-2 text-fg">{p.after}</span>
          </div>
        </div>
      ))}
      <div className="pt-0.5 text-[11px] text-fg-subtle">
        共 {d.pairs.length} 处改动，其余段落原样没动
      </div>
    </div>
  )
}

/** C · 单元格：表格里改了哪几格，旧值划掉、新值跟上。
 *  适合 Excel 类改动，比段落 diff 好读。 */
export function DiffSheet({ d }: { d: DocChange }) {
  return (
    <div className="mt-2.5 overflow-hidden rounded-field border border-surface-border bg-surface-sunken">
      <div className="flex items-center gap-2 border-b border-surface-border px-2.5 py-1.5 text-[11px] text-fg-subtle">
        <FileSpreadsheet size={11} />
        <span className="text-fg-muted">{d.sheet.name}</span>
        <span className="ml-auto font-mono">{d.sheet.cells.length} 格</span>
      </div>
      <table className="w-full border-collapse text-[12px]">
        <tbody>
          {d.sheet.cells.map((c) => (
            <tr key={c.cell} className="border-b border-surface-border/60 last:border-0">
              <td className="w-14 px-2.5 py-1.5 font-mono text-[11px] text-fg-subtle">{c.cell}</td>
              <td className="px-2 py-1.5 text-fg-subtle">
                <span className="line-through decoration-danger/40">{c.before}</span>
              </td>
              <td className="px-2 py-1.5 text-fg">{c.after}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}

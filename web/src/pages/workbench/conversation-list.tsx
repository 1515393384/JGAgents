import { useEffect, useRef, useState } from 'react'
import { Link } from '@tanstack/react-router'
import {
  Bell,
  Check,
  ChevronDown,
  ChevronRight,
  Clock,
  FileSpreadsheet,
  FileText,
  Folder,
  LogOut,
  PanelLeftClose,
  Plus,
  Search,
  Settings2,
  X,
} from 'lucide-react'
import { Avatar } from '../../components/ui'
import { ThemeMenu } from '../../components/theme-menu'
import { SpaceSettings } from './space-settings'
import { folderTrees, spaceGroups, spaces, type FolderNode, type Space, type WorkItem } from '../../lib/mock'

const kindIcon = {
  标书: FileText,
  Word: FileText,
  表格: FileSpreadsheet,
  提醒: Bell,
} as const

type Props = {
  space: Space
  /** 窄屏抽屉是否打开（lg 以上恒为常驻侧栏，此参数无效） */
  open: boolean
  onClose: () => void
}

function FileRow({
  item,
  shared,
  inset = 0,
}: {
  item: WorkItem
  shared: boolean
  inset?: number
}) {
  const Icon = kindIcon[item.kind]
  return (
    <button
      type="button"
      style={{ paddingLeft: 10 + inset }}
      className={
        'group relative flex w-full items-start gap-2.5 rounded-field py-2 pr-2.5 text-left transition-colors ' +
        (item.active ? 'bg-bg-subtle' : 'hover:bg-bg-subtle/60')
      }
    >
      {item.active ? (
        <span className="absolute inset-y-1.5 left-0 w-[2px] rounded-pill bg-brand" />
      ) : null}

      <span
        className={
          'mt-0.5 inline-flex h-5 w-5 shrink-0 items-center justify-center ' +
          (item.active ? 'text-brand' : 'text-fg-subtle group-hover:text-fg-muted')
        }
      >
        <Icon size={13} />
      </span>

      <span className="min-w-0 flex-1">
        <span
          className={
            'block truncate text-[13px] leading-5 ' +
            (item.active ? 'font-medium text-fg' : 'text-fg-muted group-hover:text-fg')
          }
        >
          {item.title}
        </span>
        <span className="mt-0.5 flex items-center gap-1.5">
          {/* 个人空间没有别人，画头像只会误导 */}
          {shared && item.members.length > 0 ? (
            <span className="flex -space-x-1">
              {item.members.slice(0, 3).map((m) => (
                <Avatar key={m} name={m} size={14} tone="muted" ring />
              ))}
            </span>
          ) : null}
          <span className="text-[11px] text-fg-subtle">{item.at}</span>
        </span>
      </span>
    </button>
  )
}

function FolderRow({
  node,
  depth,
  shared,
  openSet,
  toggle,
}: {
  node: FolderNode
  depth: number
  shared: boolean
  openSet: Set<string>
  toggle: (id: string) => void
}) {
  const kids = node.children ?? []
  const files = node.items ?? []
  const hasKids = kids.length > 0
  const isOpen = openSet.has(node.id)

  return (
    <div>
      <button
        type="button"
        onClick={() => hasKids && toggle(node.id)}
        style={{ paddingLeft: 6 + depth * 12 }}
        className="flex w-full items-center gap-1.5 rounded-field py-1.5 pr-2 text-left text-[12.5px] text-fg-muted hover:bg-bg-subtle/60 hover:text-fg"
      >
        {hasKids ? (
          <ChevronRight
            size={12}
            className={'shrink-0 text-fg-subtle transition-transform ' + (isOpen ? 'rotate-90' : '')}
          />
        ) : (
          <span className="w-3 shrink-0" />
        )}
        <Folder size={13} className="shrink-0 text-fg-subtle" />
        <span className="truncate">{node.name}</span>
        <span className="ml-auto shrink-0 font-mono text-[10.5px] text-fg-subtle">
          {files.length + kids.reduce((n, k) => n + (k.items?.length ?? 0), 0) || ''}
        </span>
      </button>

      {files.map((f) => (
        <FileRow key={f.id} item={f} shared={shared} inset={18 + depth * 12} />
      ))}

      {isOpen
        ? kids.map((k) => (
            <FolderRow
              key={k.id}
              node={k}
              depth={depth + 1}
              shared={shared}
              openSet={openSet}
              toggle={toggle}
            />
          ))
        : null}
    </div>
  )
}

/** 空间切换器：企业里"我在哪个空间"比"我在哪个会话"重要，所以放最上面 */
function SpaceSwitcher({ space, onOpenSettings }: { space: Space; onOpenSettings: () => void }) {
  const [open, setOpen] = useState(false)
  const ref = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (!open) return
    const onDown = (e: MouseEvent) => {
      if (!ref.current?.contains(e.target as Node)) setOpen(false)
    }
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setOpen(false)
    }
    document.addEventListener('mousedown', onDown)
    document.addEventListener('keydown', onKey)
    return () => {
      document.removeEventListener('mousedown', onDown)
      document.removeEventListener('keydown', onKey)
    }
  }, [open])

  return (
    <div ref={ref} className="relative min-w-0">
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        aria-expanded={open}
        className="flex min-w-0 items-center gap-1.5 rounded-field py-1 pl-1 pr-1.5 hover:bg-bg-subtle"
      >
        <span
          className={
            'inline-flex h-6 w-6 shrink-0 items-center justify-center rounded-field text-[11px] font-semibold ' +
            (space.kind === 'personal' ? 'bg-bg-subtle text-fg-muted' : 'bg-brand text-brand-fg')
          }
        >
          {space.kind === 'personal' ? '个' : space.name.slice(0, 1)}
        </span>
        <span className="truncate text-[13px] font-medium">{space.name}</span>
        <ChevronDown size={13} className="shrink-0 text-fg-subtle" />
      </button>

      {open ? (
        <div className="absolute left-0 top-full z-50 mt-1 w-[212px] overflow-hidden rounded-panel border border-surface-border bg-surface shadow-float">
          <div className="px-2.5 py-1.5 text-[11px] text-fg-subtle">空间</div>
          <div className="p-1 pt-0">
            {spaces.map((s) => (
              <Link
                key={s.id}
                to="/s/$spaceId"
                params={{ spaceId: s.id }}
                onClick={() => setOpen(false)}
                className={
                  'flex items-center gap-2 rounded-field px-1.5 py-1.5 text-[12.5px] ' +
                  (s.id === space.id
                    ? 'bg-bg-subtle text-fg'
                    : 'text-fg-muted hover:bg-bg-subtle/60 hover:text-fg')
                }
              >
                <span
                  className={
                    'inline-flex h-5 w-5 shrink-0 items-center justify-center rounded-field text-[10px] font-semibold ' +
                    (s.kind === 'personal' ? 'bg-bg-subtle text-fg-muted' : 'bg-brand-soft text-brand')
                  }
                >
                  {s.kind === 'personal' ? '个' : s.name.slice(0, 1)}
                </span>
                <span className="min-w-0 flex-1">
                  <span className="block truncate">{s.name}</span>
                  <span className="block truncate text-[10.5px] text-fg-subtle">{s.sub}</span>
                </span>
                {s.id === space.id ? <Check size={13} className="shrink-0 text-brand" /> : null}
              </Link>
            ))}
          </div>

          <div className="border-t border-surface-border p-1">
            <button
              type="button"
              onClick={() => {
                setOpen(false)
                onOpenSettings()
              }}
              className="flex w-full items-center gap-2 rounded-field px-1.5 py-1.5 text-left text-[12px] text-fg-muted hover:bg-bg-subtle/60 hover:text-fg"
            >
              <Settings2 size={12} />
              成员与配额
            </button>
          </div>
        </div>
      ) : null}
    </div>
  )
}

export function ConversationList({ space, open, onClose }: Props) {
  const shared = space.kind === 'team'
  const [view, setView] = useState<'recent' | 'folders'>('recent')
  const [openSet, setOpenSet] = useState<Set<string>>(() => new Set(['f-p1']))
  const [settingsOpen, setSettingsOpen] = useState(false)

  const toggle = (id: string) =>
    setOpenSet((prev) => {
      const next = new Set(prev)
      if (next.has(id)) next.delete(id)
      else next.add(id)
      return next
    })

  return (
    <>
      {/* 窄屏遮罩：只在抽屉打开时可点 */}
      <div
        onClick={onClose}
        aria-hidden
        className={
          'fixed inset-0 z-30 bg-black/40 backdrop-blur-sm transition-opacity duration-200 lg:hidden ' +
          (open ? 'opacity-100' : 'pointer-events-none opacity-0')
        }
      />

      <aside
        className={
          'fixed inset-y-0 left-0 z-40 flex w-[264px] max-w-[86vw] flex-col border-r border-surface-border bg-surface-sunken ' +
          'transition-transform duration-200 lg:static lg:z-auto lg:w-[236px] lg:max-w-none lg:translate-x-0 ' +
          (open ? 'translate-x-0' : '-translate-x-full')
        }
      >
        <div className="flex h-14 shrink-0 items-center gap-1 px-2">
          <SpaceSwitcher space={space} onOpenSettings={() => setSettingsOpen(true)} />
          <span className="ml-auto flex items-center gap-0.5">
            <button
              type="button"
              aria-label="收起列表"
              className="hidden h-8 w-8 items-center justify-center rounded-field text-fg-subtle hover:bg-bg-subtle hover:text-fg lg:inline-flex"
            >
              <PanelLeftClose size={15} />
            </button>
            <button
              type="button"
              onClick={onClose}
              aria-label="关闭列表"
              className="inline-flex h-8 w-8 items-center justify-center rounded-field text-fg-subtle hover:bg-bg-subtle hover:text-fg lg:hidden"
            >
              <X size={16} />
            </button>
          </span>
        </div>

        <div className="px-3 pb-1">
          <button
            type="button"
            className="flex w-full items-center gap-2 rounded-field bg-bg-subtle px-2.5 py-2 text-[13px] font-medium text-fg-muted transition-colors hover:text-fg"
          >
            <Plus size={14} />
            新建
          </button>
        </div>

        <div className="px-3 py-2">
          <div className="flex items-center gap-1.5 px-2 pb-2 text-xs text-fg-subtle">
            <Search size={13} />
            搜索文件、标书、纪要
          </div>

          {/* 最近 / 文件夹：同一批活的两种视图，不拆成两个页面 */}
          <div className="flex items-center gap-0.5 rounded-field bg-bg-subtle/70 p-0.5">
            {(
              [
                { id: 'recent', label: '最近', icon: Clock },
                { id: 'folders', label: '文件夹', icon: Folder },
              ] as const
            ).map((v) => {
              const Icon = v.icon
              const on = view === v.id
              return (
                <button
                  key={v.id}
                  type="button"
                  onClick={() => setView(v.id)}
                  className={
                    'flex flex-1 items-center justify-center gap-1 rounded-[7px] py-1 text-[12px] transition-colors ' +
                    (on ? 'bg-surface text-fg shadow-card' : 'text-fg-subtle hover:text-fg-muted')
                  }
                >
                  <Icon size={12} />
                  {v.label}
                </button>
              )
            })}
          </div>
        </div>

        <div className="min-h-0 flex-1 overflow-y-auto px-2 pb-4">
          {view === 'recent'
            ? (spaceGroups[space.id] ?? []).map((g) => (
                <div key={g.label} className="mb-3">
                  <div className="px-2.5 pb-1.5 text-[11px] font-medium tracking-wide text-fg-subtle">
                    {g.label}
                  </div>
                  <div className="space-y-0.5">
                    {g.items.map((it) => (
                      <FileRow key={it.id} item={it} shared={shared} />
                    ))}
                  </div>
                </div>
              ))
            : (
                <div className="space-y-1 pt-0.5">
                  {(folderTrees[space.id] ?? []).map((n) => (
                    <FolderRow
                      key={n.id}
                      node={n}
                      depth={0}
                      shared={shared}
                      openSet={openSet}
                      toggle={toggle}
                    />
                  ))}
                </div>
              )}
        </div>

        <div className="flex shrink-0 items-center gap-2 border-t border-surface-border px-3 py-2.5">
          <Avatar name="陈默" size={26} />
          <span className="min-w-0 flex-1">
            <span className="block truncate text-[13px]">陈默</span>
            <span className="block text-[11px] text-fg-subtle">
              {space.kind === 'personal' ? '个人空间' : '研发 · 本月 312 次'}
            </span>
          </span>
          <ThemeMenu />
          <Link
            to="/login"
            aria-label="退出"
            className="inline-flex h-7 w-7 items-center justify-center rounded-field text-fg-subtle hover:bg-bg-subtle hover:text-fg"
          >
            <LogOut size={14} />
          </Link>
        </div>
      </aside>

      <SpaceSettings space={space} open={settingsOpen} onClose={() => setSettingsOpen(false)} />
    </>
  )
}

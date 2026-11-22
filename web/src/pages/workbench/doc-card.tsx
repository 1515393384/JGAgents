import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import {
  AlertTriangle,
  Check,
  Download,
  FileDiff,
  FileText,
  FileUp,
  ListTree,
  Lock,
  Pencil,
  Search,
  Unlock,
  X,
} from 'lucide-react'
import { MicroWindow } from '../../components/micro-window'
import {
  DocClientError,
  downloadDocRevision,
  fetchDocDiff,
  fetchDocRender,
  getDocDetail,
  listDocs,
  patchDocEdits,
  searchDoc,
  toggleNodeLock,
  uploadDocFile,
  uploadDocRevisionFile,
  type DocDetail,
  type DocDiffReport,
  type DocNodeRecord,
  type DocRecord,
  type DocSearchHit,
  type ExportMode,
  type PageSetup,
  type ParaFmt,
  type RPara,
  type RenderBlock,
  type RenderSlice,
  type RTable,
  type RunFmt,
} from '../../lib/docs'

type ViewTab = 'body' | 'outline' | 'diff'

/** 文档微应用：盖在会话之上，承载纯 .docx 原包解析、AST 分片检索、条款锁定、精准 Diff 与双模式无损导出。 */
export function DocCard({
  open,
  spaceId,
  onClose,
}: {
  open: boolean
  spaceId: string
  onClose: () => void
}) {
  const [docs, setDocs] = useState<DocRecord[]>([])
  const [activeDocId, setActiveDocId] = useState<string>('')
  const [detail, setDetail] = useState<DocDetail | null>(null)
  const [tab, setTab] = useState<ViewTab>('body')
  const [loading, setLoading] = useState(false)
  const [busyMsg, setBusyMsg] = useState<string | null>(null)
  const [errorBanner, setErrorBanner] = useState<{
    message: string
    legacyDocHint: boolean
  } | null>(null)

  const [searchText, setSearchText] = useState('')
  const [searchHits, setSearchHits] = useState<DocSearchHit[] | null>(null)
  const [searching, setSearching] = useState(false)

  const [editingAnchor, setEditingAnchor] = useState<string | null>(null)
  const [editDraft, setEditDraft] = useState('')
  const [editMode, setEditMode] = useState<'redline' | 'clean'>('redline')

  const [diffFromRev, setDiffFromRev] = useState<number>(1)
  const [diffToRev, setDiffToRev] = useState<number>(1)
  const [diffReport, setDiffReport] = useState<DocDiffReport | null>(null)
  const [onlyChangedDiff, setOnlyChangedDiff] = useState(true)

  const [focusedAnchor, setFocusedAnchor] = useState<string | null>(null)
  const [render, setRender] = useState<RenderSlice | null>(null)
  const [renderLoading, setRenderLoading] = useState(false)
  const [renderMissing, setRenderMissing] = useState(false)

  const uploadInputRef = useRef<HTMLInputElement | null>(null)
  const revUploadInputRef = useRef<HTMLInputElement | null>(null)
  const nodeRefs = useRef<Record<string, HTMLDivElement | null>>({})

  const loadWorkspaceDocs = useCallback(async () => {
    setLoading(true)
    setErrorBanner(null)
    try {
      const list = await listDocs(spaceId)
      setDocs(list)
      const first = list[0]
      if (first) {
        setActiveDocId(first.id)
        const d = await getDocDetail(first.id)
        setDetail(d)
        const cur = d.doc.currentRev
        setDiffToRev(cur)
        setDiffFromRev(Math.max(1, cur - 1))
      } else {
        setActiveDocId('')
        setDetail(null)
      }
    } catch (err) {
      setErrorBanner({
        message: err instanceof Error ? err.message : '加载文档失败',
        legacyDocHint: err instanceof DocClientError && err.needsHumanSaveAsDocx,
      })
    } finally {
      setLoading(false)
    }
  }, [spaceId])

  useEffect(() => {
    if (open) {
      void loadWorkspaceDocs()
    }
  }, [open, loadWorkspaceDocs])

  const selectDoc = async (docId: string) => {
    setActiveDocId(docId)
    setSearchHits(null)
    setSearchText('')
    setEditingAnchor(null)
    setLoading(true)
    setErrorBanner(null)
    try {
      const d = await getDocDetail(docId)
      setDetail(d)
      const cur = d.doc.currentRev
      setDiffToRev(cur)
      setDiffFromRev(Math.max(1, cur - 1))
    } catch (err) {
      setErrorBanner({
        message: err instanceof Error ? err.message : '切换文档失败',
        legacyDocHint: false,
      })
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    if (!detail || tab !== 'diff') return
    if (detail.revisions.length === 0) return
    let cancelled = false
    const run = async () => {
      try {
        const rep = await fetchDocDiff(detail.doc.id, diffFromRev, diffToRev)
        if (!cancelled) setDiffReport(rep)
      } catch (err) {
        if (!cancelled) {
          setErrorBanner({
            message: err instanceof Error ? err.message : '计算版本 Diff 失败',
            legacyDocHint: false,
          })
        }
      }
    }
    void run()
    return () => {
      cancelled = true
    }
  }, [detail, tab, diffFromRev, diffToRev])

  useEffect(() => {
    const docId = detail?.doc.id
    const rev = detail?.doc.currentRev
    if (!docId || rev === undefined || tab !== 'body') return
    let cancelled = false
    setRenderLoading(true)
    setRenderMissing(false)
    void (async () => {
      try {
        const slice = await fetchDocRender(docId, { rev })
        if (cancelled) return
        setRender(slice)
        setRenderMissing(slice === null)
      } catch {
        if (cancelled) return
        setRender(null)
        setRenderMissing(true)
      } finally {
        if (!cancelled) setRenderLoading(false)
      }
    })()
    return () => {
      cancelled = true
    }
  }, [detail?.doc.id, detail?.doc.currentRev, tab])

  const handleUploadNewDoc = async (file: File | undefined) => {
    if (!file) return
    setErrorBanner(null)
    setBusyMsg('正在校验并解析 .docx 结构树…')
    try {
      const created = await uploadDocFile(file, spaceId)
      const list = await listDocs(spaceId)
      setDocs(list)
      setActiveDocId(created.doc.id)
      setDetail(created)
      setDiffFromRev(1)
      setDiffToRev(created.doc.currentRev)
      setTab('body')
      setSearchHits(null)
      setSearchText('')
    } catch (err) {
      setErrorBanner({
        message: err instanceof Error ? err.message : '上传文档失败',
        legacyDocHint: err instanceof DocClientError && err.needsHumanSaveAsDocx,
      })
    } finally {
      setBusyMsg(null)
      if (uploadInputRef.current) uploadInputRef.current.value = ''
    }
  }

  const handleUploadRevision = async (file: File | undefined) => {
    if (!file || !detail) return
    setErrorBanner(null)
    setBusyMsg('正在比对回传 .docx 并对齐四级锚点…')
    try {
      const prevRev = detail.doc.currentRev
      const res = await uploadDocRevisionFile(detail.doc.id, file)
      setDetail(res.detail)
      setDiffFromRev(prevRev)
      setDiffToRev(res.detail.doc.currentRev)
      setDiffReport(res.diff)
      setTab('diff')
    } catch (err) {
      setErrorBanner({
        message: err instanceof Error ? err.message : '上传修订版失败',
        legacyDocHint: err instanceof DocClientError && err.needsHumanSaveAsDocx,
      })
    } finally {
      setBusyMsg(null)
      if (revUploadInputRef.current) revUploadInputRef.current.value = ''
    }
  }

  const handleSearch = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!detail) return
    const q = searchText.trim()
    if (!q) {
      setSearchHits(null)
      return
    }
    setSearching(true)
    setErrorBanner(null)
    try {
      const hits = await searchDoc(detail.doc.id, q)
      setSearchHits(hits)
      setTab('body')
    } catch (err) {
      setErrorBanner({
        message: err instanceof Error ? err.message : '检索失败',
        legacyDocHint: false,
      })
    } finally {
      setSearching(false)
    }
  }

  const handleToggleLock = async (node: DocNodeRecord) => {
    if (!detail) return
    setErrorBanner(null)
    try {
      const updated = await toggleNodeLock(detail.doc.id, node.anchorId, !node.locked)
      setDetail((prev) =>
        prev
          ? {
              ...prev,
              nodes: prev.nodes.map((n) => (n.anchorId === updated.anchorId ? updated : n)),
            }
          : prev,
      )
    } catch (err) {
      setErrorBanner({
        message: err instanceof Error ? err.message : '切换锁定状态失败',
        legacyDocHint: false,
      })
    }
  }

  const handleCommitEdit = async (node: DocNodeRecord) => {
    if (!detail) return
    const nextText = editDraft.trim()
    if (!nextText || nextText === node.textContent) {
      setEditingAnchor(null)
      return
    }
    setErrorBanner(null)
    setBusyMsg('正在原位补丁写回 word/document.xml…')
    try {
      const prevRev = detail.doc.currentRev
      const res = await patchDocEdits(
        detail.doc.id,
        [{ anchorId: node.anchorId, newText: nextText }],
        editMode,
      )
      setDetail(res.detail)
      setEditingAnchor(null)
      setDiffFromRev(prevRev)
      setDiffToRev(res.detail.doc.currentRev)
      if (res.diff) setDiffReport(res.diff)
    } catch (err) {
      setErrorBanner({
        message: err instanceof Error ? err.message : '提交修订失败',
        legacyDocHint: false,
      })
    } finally {
      setBusyMsg(null)
    }
  }

  const handleDownload = async (mode: ExportMode) => {
    if (!detail) return
    setErrorBanner(null)
    setBusyMsg(`正在生成${mode === 'redline' ? '修订红线版' : '净稿版'} .docx…`)
    try {
      await downloadDocRevision(
        detail.doc.id,
        detail.doc.currentRev,
        mode,
        detail.doc.filename,
      )
    } catch (err) {
      setErrorBanner({
        message: err instanceof Error ? err.message : '导出文档失败',
        legacyDocHint: false,
      })
    } finally {
      setBusyMsg(null)
    }
  }

  const jumpToAnchor = (anchorId: string) => {
    setTab('body')
    setFocusedAnchor(anchorId)
    window.setTimeout(() => {
      nodeRefs.current[anchorId]?.scrollIntoView({ behavior: 'smooth', block: 'center' })
    }, 60)
    window.setTimeout(() => setFocusedAnchor(null), 2200)
  }

  const primaryNodes = useMemo(
    () => (detail?.nodes ?? []).filter((n) => !n.isSearchChunk),
    [detail],
  )

  const headings = useMemo(
    () => primaryNodes.filter((n) => n.nodeKind === 'heading'),
    [primaryNodes],
  )

  const mandatoryNodes = useMemo(
    () => primaryNodes.filter((n) => n.mandatoryTags.length > 0),
    [primaryNodes],
  )

  const refsByAnchor = useMemo(() => {
    const map = new Map<string, string[]>()
    for (const r of detail?.refs ?? []) {
      const list = map.get(r.sourceAnchor) ?? []
      list.push(r.targetHint)
      map.set(r.sourceAnchor, list)
    }
    return map
  }, [detail])

  const latestRevMeta = detail?.revisions[0]
  const revSubtitle = detail
    ? detail.doc.currentRev > 1
      ? `v1 → v${detail.doc.currentRev}`
      : `v${detail.doc.currentRev}`
    : undefined

  const statusText = detail
    ? `${latestRevMeta?.nodeCount ?? primaryNodes.length} 节点 · ${latestRevMeta?.tableCount ?? 0} 表格 · ${latestRevMeta?.mandatoryCount ?? mandatoryNodes.length} 硬约束`
    : undefined

  const nodeByAnchor = useMemo(() => {
    const map = new Map<string, DocNodeRecord>()
    for (const n of detail?.nodes ?? []) map.set(n.anchorId, n)
    return map
  }, [detail])

  const blockCtx: BlockCtx = {
    nodeByAnchor,
    refsByAnchor,
    focusedAnchor,
    editingAnchor,
    editDraft,
    editMode,
    onDraftChange: setEditDraft,
    onModeChange: setEditMode,
    onStartEdit: (node) => {
      if (node.locked) {
        setErrorBanner({
          message: `条款 ${node.anchorId} 已锁定，请先解除保护后再修改。`,
          legacyDocHint: false,
        })
        return
      }
      setEditingAnchor(node.anchorId)
      setEditDraft(node.textContent)
    },
    onCancelEdit: () => setEditingAnchor(null),
    onCommitEdit: (node) => void handleCommitEdit(node),
    onToggleLock: (node) => void handleToggleLock(node),
    registerRef: (anchor, el) => {
      nodeRefs.current[anchor] = el
    },
  }

  return (
    <MicroWindow
      open={open}
      onClose={onClose}
      icon={<FileText size={13} />}
      title={detail ? detail.doc.filename : '标书文档工作台'}
      subtitle={revSubtitle}
      status={
        statusText ? (
          <span className="hidden shrink-0 font-mono text-[10.5px] tabular-nums text-fg-subtle sm:inline">
            {statusText}
          </span>
        ) : null
      }
      footer={
        <div className="flex flex-wrap items-center justify-between gap-2 border-t border-surface-border bg-surface px-3 py-2 text-[11.5px] text-fg-subtle">
          <span className="truncate">
            仅处理 .docx 原包；若下游必交老 .doc，请导出净稿后用 Word/WPS 另存一次
          </span>
          <div className="flex shrink-0 items-center gap-1.5">
            <button
              type="button"
              disabled={!detail || !!busyMsg}
              onClick={() => void handleDownload('redline')}
              className="inline-flex h-7 items-center gap-1 rounded-field border border-border bg-bg-subtle px-2.5 text-[11.5px] text-fg transition-colors hover:bg-bg-accent disabled:opacity-40"
            >
              <Download size={12} />
              导出修订红线版
            </button>
            <button
              type="button"
              disabled={!detail || !!busyMsg}
              onClick={() => void handleDownload('clean')}
              className="inline-flex h-7 items-center gap-1 rounded-field bg-brand px-2.5 text-[11.5px] font-medium text-brand-fg transition-colors hover:bg-brand-hover disabled:opacity-40"
            >
              <Download size={12} />
              导出净稿版
            </button>
          </div>
        </div>
      }
    >
      {/* 顶部工具栏：文档切换 / 上传 / 分段视图 / FTS5 检索 */}
      <div className="flex flex-wrap items-center gap-2 border-b border-surface-border bg-surface px-3 py-2">
        {docs.length > 0 ? (
          <select
            aria-label="切换文档"
            value={activeDocId}
            onChange={(e) => void selectDoc(e.target.value)}
            className="h-7 max-w-[170px] rounded-field border border-border bg-surface-sunken px-2 text-[12px] text-fg outline-none focus:border-brand"
          >
            {docs.map((d) => (
              <option key={d.id} value={d.id}>
                {d.filename} (v{d.currentRev})
              </option>
            ))}
          </select>
        ) : null}

        <div className="inline-flex rounded-field border border-border bg-surface-sunken p-0.5 text-[11.5px]">
          <button
            type="button"
            onClick={() => setTab('body')}
            className={
              'inline-flex h-6 items-center gap-1 rounded-[7px] px-2 transition-colors ' +
              (tab === 'body' ? 'bg-surface font-medium text-fg shadow-xs' : 'text-fg-muted hover:text-fg')
            }
          >
            <FileText size={11} />
            正文
          </button>
          <button
            type="button"
            onClick={() => setTab('outline')}
            className={
              'inline-flex h-6 items-center gap-1 rounded-[7px] px-2 transition-colors ' +
              (tab === 'outline' ? 'bg-surface font-medium text-fg shadow-xs' : 'text-fg-muted hover:text-fg')
            }
          >
            <ListTree size={11} />
            目录与硬约束
            {mandatoryNodes.length > 0 ? (
              <span className="font-mono text-[10px] tabular-nums text-warn">
                {mandatoryNodes.length}
              </span>
            ) : null}
          </button>
          <button
            type="button"
            onClick={() => setTab('diff')}
            className={
              'inline-flex h-6 items-center gap-1 rounded-[7px] px-2 transition-colors ' +
              (tab === 'diff' ? 'bg-surface font-medium text-fg shadow-xs' : 'text-fg-muted hover:text-fg')
            }
          >
            <FileDiff size={11} />
            版本 Diff
          </button>
        </div>

        <form onSubmit={(e) => void handleSearch(e)} className="relative flex min-w-[140px] flex-1 items-center">
          <Search size={12} className="pointer-events-none absolute left-2 text-fg-subtle" />
          <input
            value={searchText}
            onChange={(e) => {
              setSearchText(e.target.value)
              if (!e.target.value.trim()) setSearchHits(null)
            }}
            placeholder="检索条款 / 表号 / 参数…"
            aria-label="检索文档内容"
            className="h-7 w-full rounded-field border border-border bg-surface-sunken pl-7 pr-6 text-[12px] text-fg outline-none placeholder:text-fg-subtle focus:border-brand"
          />
          {searchText ? (
            <button
              type="button"
              onClick={() => {
                setSearchText('')
                setSearchHits(null)
              }}
              aria-label="清空搜索"
              className="absolute right-1.5 text-fg-subtle hover:text-fg"
            >
              <X size={12} />
            </button>
          ) : null}
        </form>

        <div className="ml-auto flex shrink-0 items-center gap-1.5">
          <input
            ref={uploadInputRef}
            type="file"
            accept=".docx,.doc,.wps"
            className="hidden"
            onChange={(e) => void handleUploadNewDoc(e.target.files?.[0])}
          />
          <input
            ref={revUploadInputRef}
            type="file"
            accept=".docx,.doc,.wps"
            className="hidden"
            onChange={(e) => void handleUploadRevision(e.target.files?.[0])}
          />
          <button
            type="button"
            disabled={!!busyMsg}
            onClick={() => uploadInputRef.current?.click()}
            className="inline-flex h-7 items-center gap-1 rounded-field border border-border bg-surface-sunken px-2 text-[11.5px] text-fg-muted transition-colors hover:text-fg disabled:opacity-40"
          >
            <FileUp size={12} />
            上传 .docx
          </button>
          {detail ? (
            <button
              type="button"
              disabled={!!busyMsg}
              onClick={() => revUploadInputRef.current?.click()}
              title="上传同事用 WPS/Word 改完回传的 .docx，自动生成精确 Diff"
              className="inline-flex h-7 items-center gap-1 rounded-field border border-border bg-surface-sunken px-2 text-[11.5px] text-fg-muted transition-colors hover:text-fg disabled:opacity-40"
            >
              <FileDiff size={12} />
              回传新版本
            </button>
          ) : null}
        </div>
      </div>

      {/* 状态与错误提示区（含 .doc 人手转换引导） */}
      {busyMsg ? (
        <div className="border-b border-brand/20 bg-brand-soft/60 px-3 py-1.5 text-[11.5px] text-brand">
          {busyMsg}
        </div>
      ) : null}

      {errorBanner ? (
        <div
          role="alert"
          className={
            'flex items-start gap-2 border-b px-3 py-2 text-[12px] leading-5 ' +
            (errorBanner.legacyDocHint
              ? 'border-warn/30 bg-warn/10 text-warn'
              : 'border-danger/30 bg-danger/10 text-danger')
          }
        >
          <AlertTriangle size={14} className="mt-0.5 shrink-0" />
          <div className="min-w-0 flex-1">
            <p className="font-medium">
              {errorBanner.legacyDocHint ? '需要先另存为 .docx 再入库' : '操作未完成'}
            </p>
            <p className="mt-0.5 text-fg-muted">{errorBanner.message}</p>
          </div>
          <button
            type="button"
            onClick={() => setErrorBanner(null)}
            aria-label="关闭提示"
            className="shrink-0 text-fg-subtle hover:text-fg"
          >
            <X size={13} />
          </button>
        </div>
      ) : null}

      {/* 主纸面区域 */}
      <div className="min-h-0 flex-1 overflow-y-auto bg-surface-sunken px-3 py-4 md:px-6">
        {loading ? (
          <div className="mx-auto max-w-[680px] rounded-field bg-surface px-6 py-12 text-center text-[13px] text-fg-subtle shadow-card">
            正在解析文档结构树…
          </div>
        ) : !detail ? (
          <div className="mx-auto max-w-[680px] rounded-field bg-surface px-6 py-12 text-center shadow-card">
            <p className="text-[14px] font-medium text-fg">当前空间暂无标书文档</p>
            <p className="mt-1.5 text-[12.5px] text-fg-muted">
              上传标准 .docx 文件即可提取章节目录、完整表格、★/▲ 硬约束与交叉引用；若原件是老 .doc，请先用 Word/WPS 另存为 .docx。
            </p>
            <button
              type="button"
              onClick={() => uploadInputRef.current?.click()}
              className="mt-4 inline-flex h-8 items-center gap-1.5 rounded-field bg-brand px-3.5 text-[12.5px] font-medium text-brand-fg hover:bg-brand-hover"
            >
              <FileUp size={13} />
              选择 .docx 上传
            </button>
          </div>
        ) : (
          <div className="mx-auto max-w-[680px] rounded-field bg-surface px-5 py-6 shadow-card md:px-8 md:py-7">
            {/* FTS5 检索结果条 */}
            {searchHits !== null && tab === 'body' ? (
              <div className="mb-5 border-b border-surface-border pb-4">
                <div className="mb-2 flex items-center justify-between text-[11.5px] text-fg-subtle">
                  <span>
                    {searching
                      ? '正在检索…'
                      : `命中 ${searchHits.length} 处（已自动关联父级章节与交叉引用）`}
                  </span>
                  <button
                    type="button"
                    onClick={() => {
                      setSearchHits(null)
                      setSearchText('')
                    }}
                    className="text-brand hover:underline"
                  >
                    返回全文
                  </button>
                </div>
                {searchHits.length === 0 ? (
                  <p className="text-[12.5px] text-fg-muted">未找到匹配条款。</p>
                ) : (
                  <div className="space-y-2">
                    {searchHits.map((hit) => {
                      const target = hit.parentNode?.anchorId ?? hit.hitAnchorId
                      return (
                        <button
                          key={hit.hitAnchorId}
                          type="button"
                          onClick={() => jumpToAnchor(target)}
                          className="block w-full rounded-[6px] border border-surface-border bg-surface-sunken px-3 py-2 text-left transition-colors hover:border-brand/50"
                        >
                          <div className="text-[11px] text-fg-subtle">
                            {hit.headingPath}
                            {hit.relatedRefs.length > 0
                              ? ` · 引用 ${hit.relatedRefs.map((r) => r.targetHint).join('、')}`
                              : ''}
                          </div>
                          <div className="mt-0.5 line-clamp-2 text-[12.5px] text-fg">
                            {hit.parentNode?.textContent ?? hit.hitText}
                          </div>
                        </button>
                      )
                    })}
                  </div>
                )}
              </div>
            ) : null}

            {/* 视图 1：正文页 —— 版式渲染，人看的唯一视图 */}
            {tab === 'body' ? (
              <DocBody
                slice={render}
                loading={renderLoading}
                missing={renderMissing}
                ctx={blockCtx}
              />
            ) : null}

            {/* 视图 2：目录与硬约束 */}
            {tab === 'outline' ? (
              <div className="space-y-6">
                <section>
                  <h3 className="text-[12px] font-medium text-fg-subtle">
                    硬约束与废标红线（{mandatoryNodes.length} 处）
                  </h3>
                  {mandatoryNodes.length === 0 ? (
                    <p className="mt-2 text-[12.5px] text-fg-muted">
                      暂未检测到带 ★ / ▲ / 废标 等关键词的硬性要求条款。
                    </p>
                  ) : (
                    <div className="mt-2 divide-y divide-surface-border">
                      {mandatoryNodes.map((n) => (
                        <button
                          key={n.anchorId}
                          type="button"
                          onClick={() => jumpToAnchor(n.anchorId)}
                          className="block w-full py-2.5 text-left transition-colors hover:bg-surface-sunken/70"
                        >
                          <div className="flex items-center gap-2 text-[11px] text-fg-subtle">
                            <span className="font-mono">{n.anchorId}</span>
                            <span>·</span>
                            <span className="font-medium text-warn">{n.mandatoryTags.join(' / ')}</span>
                            <span>·</span>
                            <span className="truncate">{n.headingPath}</span>
                          </div>
                          <div className="mt-1 text-[13px] leading-6 text-fg">{n.textContent}</div>
                        </button>
                      ))}
                    </div>
                  )}
                </section>

                <section className="border-t border-surface-border pt-5">
                  <h3 className="text-[12px] font-medium text-fg-subtle">
                    章节目录树（{headings.length} 节）
                  </h3>
                  <div className="mt-2 space-y-1">
                    {headings.map((h) => (
                      <button
                        key={h.anchorId}
                        type="button"
                        onClick={() => jumpToAnchor(h.anchorId)}
                        className="flex w-full items-center justify-between rounded-[6px] px-2 py-1.5 text-left text-[13px] text-fg hover:bg-surface-sunken"
                      >
                        <span className="truncate">{h.textContent}</span>
                        <span className="shrink-0 font-mono text-[11px] tabular-nums text-fg-subtle">
                          {h.anchorId}
                        </span>
                      </button>
                    ))}
                  </div>
                </section>

                {detail.refs.length > 0 ? (
                  <section className="border-t border-surface-border pt-5">
                    <h3 className="text-[12px] font-medium text-fg-subtle">
                      文内交叉引用（{detail.refs.length} 条）
                    </h3>
                    <div className="mt-2 space-y-1 text-[12.5px] text-fg-muted">
                      {detail.refs.map((r, i) => (
                        <button
                          key={`${r.sourceAnchor}-${r.targetHint}-${i}`}
                          type="button"
                          onClick={() => jumpToAnchor(r.sourceAnchor)}
                          className="flex w-full items-center gap-2 rounded-[6px] px-2 py-1 text-left hover:bg-surface-sunken"
                        >
                          <span className="font-mono text-[11px] text-fg-subtle">
                            {r.sourceAnchor}
                          </span>
                          <span>→</span>
                          <span className="text-brand">{r.targetHint}</span>
                        </button>
                      ))}
                    </div>
                  </section>
                ) : null}
              </div>
            ) : null}

            {/* 视图 3：四级混合锚点精准 Diff */}
            {tab === 'diff' ? (
              <div className="space-y-4">
                <div className="flex flex-wrap items-center gap-2 border-b border-surface-border pb-3 text-[12px]">
                  <label className="inline-flex items-center gap-1 text-fg-muted">
                    基线
                    <select
                      value={diffFromRev}
                      onChange={(e) => setDiffFromRev(Number(e.target.value))}
                      className="h-7 rounded-field border border-border bg-surface-sunken px-2 font-mono text-[11.5px] text-fg"
                    >
                      {detail.revisions.map((r) => (
                        <option key={r.rev} value={r.rev}>
                          v{r.rev}
                        </option>
                      ))}
                    </select>
                  </label>
                  <span className="text-fg-subtle">→</span>
                  <label className="inline-flex items-center gap-1 text-fg-muted">
                    对比
                    <select
                      value={diffToRev}
                      onChange={(e) => setDiffToRev(Number(e.target.value))}
                      className="h-7 rounded-field border border-border bg-surface-sunken px-2 font-mono text-[11.5px] text-fg"
                    >
                      {detail.revisions.map((r) => (
                        <option key={r.rev} value={r.rev}>
                          v{r.rev}
                        </option>
                      ))}
                    </select>
                  </label>

                  {diffReport ? (
                    <span className="font-mono text-[11.5px] tabular-nums text-fg-subtle">
                      <span className="text-ok">+{diffReport.addedChars}</span>
                      {' · '}
                      <span className="text-danger">−{diffReport.deletedChars}</span>
                      {' · '}
                      {diffReport.changedBlocks} 处变更
                    </span>
                  ) : null}

                  <label className="ml-auto inline-flex items-center gap-1.5 text-[11.5px] text-fg-muted">
                    <input
                      type="checkbox"
                      checked={onlyChangedDiff}
                      onChange={(e) => setOnlyChangedDiff(e.target.checked)}
                      className="accent-[var(--brand)]"
                    />
                    仅显示变更段落
                  </label>
                </div>

                {diffReport && diffReport.lockedConflicts.length > 0 ? (
                  <div className="flex items-start gap-2 rounded-field border border-warn/30 bg-warn/10 px-3 py-2 text-[12px] text-warn">
                    <AlertTriangle size={14} className="mt-0.5 shrink-0" />
                    <span>
                      以下已锁定关键条款在对比版本中被改动：{diffReport.lockedConflicts.join('、')}
                    </span>
                  </div>
                ) : null}

                {diffReport ? (
                  <div className="divide-y divide-surface-border">
                    {diffReport.blocks
                      .filter((b) => !onlyChangedDiff || b.diffKind !== 'equal')
                      .map((blk, idx) => (
                        <div key={`${blk.anchorId}-${idx}`} className="py-3 first:pt-0 last:pb-0">
                          <div className="mb-1 flex flex-wrap items-center gap-2 text-[11px] text-fg-subtle">
                            <span className="font-mono">{blk.anchorId}</span>
                            <span>·</span>
                            <span>{blk.headingPath}</span>
                            <span>·</span>
                            <span
                              className={
                                blk.diffKind === 'inserted'
                                  ? 'text-ok'
                                  : blk.diffKind === 'deleted'
                                    ? 'text-danger'
                                    : blk.diffKind === 'modified'
                                      ? 'text-warn'
                                      : 'text-fg-subtle'
                              }
                            >
                              {blk.diffKind === 'inserted'
                                ? '新增'
                                : blk.diffKind === 'deleted'
                                  ? '删除'
                                  : blk.diffKind === 'modified'
                                    ? `修改 (+${blk.addedChars}/−${blk.deletedChars})`
                                    : blk.diffKind === 'moved'
                                      ? '移动'
                                      : '未改'}
                            </span>
                            {blk.locked ? (
                              <>
                                <span>·</span>
                                <span className="text-warn">锁定条款</span>
                              </>
                            ) : null}
                          </div>

                          <div className="text-[13.5px] leading-7 text-fg">
                            {blk.spans.map((sp, sIdx) => {
                              if (sp.op === 'delete') {
                                return (
                                  <del
                                    key={sIdx}
                                    className="rounded-[3px] bg-danger/12 px-0.5 text-danger line-through decoration-danger/50"
                                  >
                                    {sp.text}
                                  </del>
                                )
                              }
                              if (sp.op === 'insert') {
                                return (
                                  <ins
                                    key={sIdx}
                                    className="rounded-[3px] bg-ok/12 px-0.5 text-ok no-underline"
                                  >
                                    {sp.text}
                                  </ins>
                                )
                              }
                              return <span key={sIdx}>{sp.text}</span>
                            })}
                          </div>

                          {blk.tableRowDiffs.length > 0 ? (
                            <div className="mt-2 rounded-[6px] border border-surface-border bg-surface-sunken px-2.5 py-2 text-[12px]">
                              <div className="mb-1 text-[11px] text-fg-subtle">
                                表格行级 Key 对齐差异
                              </div>
                              {blk.tableRowDiffs.map((rd, rIdx) => (
                                <div key={rIdx} className="font-mono text-[11.5px] leading-6">
                                  {rd.beforeRow ? (
                                    <div className="text-danger line-through">− {rd.beforeRow}</div>
                                  ) : null}
                                  {rd.afterRow ? (
                                    <div className="text-ok">+ {rd.afterRow}</div>
                                  ) : null}
                                </div>
                              ))}
                            </div>
                          ) : null}
                        </div>
                      ))}
                    {diffReport.blocks.filter((b) => !onlyChangedDiff || b.diffKind !== 'equal')
                      .length === 0 ? (
                      <p className="py-6 text-center text-[12.5px] text-fg-subtle">
                        所选两个版本内容完全一致，未发现实质性文本差异（已屏蔽 WPS 格式重存噪声）。
                      </p>
                    ) : null}
                  </div>
                ) : null}
              </div>
            ) : null}
          </div>
        )}
      </div>
    </MicroWindow>
  )
}

// ── 版式渲染（人类视图）─────────────────────────────────────────────────────
// 只吃后端 render 切片：纸面尺寸/边距来自 sectPr，字样来自样式继承链展开后的确定值。

const TWIPS_PER_PX = 20

function px(twipsValue: number | undefined | null): number | undefined {
  if (!twipsValue || twipsValue <= 0) return undefined
  return Math.round(twipsValue / TWIPS_PER_PX)
}

const HIGHLIGHT_COLORS: Record<string, string> = {
  yellow: '#fff59d',
  green: '#b9f6ca',
  cyan: '#b2ebf2',
  magenta: '#f8bbd0',
  blue: '#bbdefb',
  red: '#ffcdd2',
  darkBlue: '#9fa8da',
  darkCyan: '#80cbc4',
  darkGreen: '#a5d6a7',
  darkMagenta: '#ce93d8',
  darkRed: '#ef9a9a',
  darkGray: '#b0bec5',
  lightGray: '#e0e0e0',
  black: '#9e9e9e',
}

function alignOf(fmt: ParaFmt): React.CSSProperties['textAlign'] {
  switch (fmt.align) {
    case 'center':
      return 'center'
    case 'right':
    case 'end':
      return 'right'
    case 'both':
    case 'justify':
    case 'distribute':
      return 'justify'
    default:
      return 'left'
  }
}

function runStyle(fmt: RunFmt): React.CSSProperties {
  const deco = [fmt.underline ? 'underline' : '', fmt.strike ? 'line-through' : '']
    .filter(Boolean)
    .join(' ')
  return {
    fontWeight: fmt.bold ? 600 : undefined,
    fontStyle: fmt.italic ? 'italic' : undefined,
    textDecoration: deco || undefined,
    fontSize: fmt.sizeHalfPt ? `${(fmt.sizeHalfPt / 2) * (4 / 3)}px` : undefined,
    color: /^[0-9a-fA-F]{6}$/.test(fmt.color) ? `#${fmt.color}` : undefined,
    fontFamily: fmt.font || undefined,
    backgroundColor: HIGHLIGHT_COLORS[fmt.highlight] ?? undefined,
    verticalAlign:
      fmt.vertAlign === 'superscript' ? 'super' : fmt.vertAlign === 'subscript' ? 'sub' : undefined,
  }
}

function DocPara({ para }: { para: RPara }) {
  const fmt = para.fmt
  const hasText = para.runs.some((r) => r.text.length > 0)
  const style: React.CSSProperties = {
    marginTop: px(fmt.spaceBeforeTwips),
    marginBottom: px(fmt.spaceAfterTwips),
    marginLeft: px(fmt.indentLeftTwips),
    marginRight: px(fmt.indentRightTwips),
    textIndent: px(fmt.indentFirstLineTwips),
    textAlign: alignOf(fmt),
    lineHeight: fmt.lineTwips ? `${px(fmt.lineTwips)}px` : undefined,
    whiteSpace: 'pre-wrap',
    overflowWrap: 'anywhere',
  }

  if (!hasText) {
    return <p style={style} className="min-h-[1em]" />
  }

  return (
    <p style={style}>
      {para.runs.map((run, idx) => (
        <span key={idx} style={runStyle(run.fmt)}>
          {run.text}
        </span>
      ))}
    </p>
  )
}

function DocTable({ table }: { table: RTable }) {
  const gridSum = table.grid.reduce((a, b) => a + (b > 0 ? b : 0), 0)
  const totalTwips = table.widthTwips > 0 ? table.widthTwips : gridSum

  return (
    <div className="my-2 overflow-x-auto">
      <table
        className="border-collapse text-left"
        style={{ width: px(totalTwips) ?? '100%', tableLayout: gridSum > 0 ? 'fixed' : 'auto' }}
      >
        <tbody>
          {table.rows.map((row, rIdx) => (
            <tr key={rIdx}>
              {row.cells.map((cell, cIdx) => (
                <td
                  key={cIdx}
                  colSpan={cell.gridSpan > 1 ? cell.gridSpan : undefined}
                  className={
                    'border border-neutral-400 px-1.5 py-1 align-top' +
                    (row.header ? ' bg-neutral-100 font-medium' : '')
                  }
                  style={{ width: px(cell.widthTwips) }}
                >
                  {cell.paras.length === 0 ? (
                    <p className="min-h-[1em]" />
                  ) : (
                    cell.paras.map((p, pIdx) => <DocPara key={pIdx} para={p} />)
                  )}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}

type BlockCtx = {
  nodeByAnchor: Map<string, DocNodeRecord>
  refsByAnchor: Map<string, string[]>
  focusedAnchor: string | null
  editingAnchor: string | null
  editDraft: string
  editMode: 'redline' | 'clean'
  onDraftChange: (v: string) => void
  onModeChange: (m: 'redline' | 'clean') => void
  onStartEdit: (node: DocNodeRecord) => void
  onCancelEdit: () => void
  onCommitEdit: (node: DocNodeRecord) => void
  onToggleLock: (node: DocNodeRecord) => void
  registerRef: (anchor: string, el: HTMLDivElement | null) => void
}

function DocBody({
  slice,
  loading,
  missing,
  ctx,
}: {
  slice: RenderSlice | null
  loading: boolean
  missing: boolean
  ctx: BlockCtx
}) {
  if (!slice) {
    return (
      <div className="py-8 text-center">
        <p className="text-[12.5px] text-fg-muted">
          {loading ? '正在加载版式…' : '该版本没有版式数据。'}
        </p>
        {!loading ? (
          <p className="mt-1 text-[11.5px] text-fg-subtle">
            {missing ? '把这份 .docx 重新入库一次即可生成版式。' : ''}
          </p>
        ) : null}
      </div>
    )
  }

  const page: PageSetup = slice.page
  return (
    <div className="py-3">
      <div
        className="mx-auto bg-white text-neutral-900 shadow-[0_2px_12px_rgba(0,0,0,0.18)] ring-1 ring-black/10"
        style={{
          width: px(page.widthTwips),
          minHeight: px(page.heightTwips),
          paddingTop: px(page.marginTopTwips),
          paddingBottom: px(page.marginBottomTwips),
          paddingLeft: px(page.marginLeftTwips),
          paddingRight: px(page.marginRightTwips),
        }}
      >
        {slice.blocks.map((block, idx) => (
          <DocBlock key={`${block.anchor}:${idx}`} block={block} ctx={ctx} />
        ))}
      </div>
      <div className="mt-2 text-center font-mono text-[10.5px] tabular-nums text-fg-subtle">
        {slice.paraCount} 段 · {slice.tableCount} 表 · v{slice.rev}
        {slice.hasMore ? ` · 已载入前 ${slice.blocks.length}/${slice.total} 块` : ''}
      </div>
    </div>
  )
}

function DocBlock({ block, ctx }: { block: RenderBlock; ctx: BlockCtx }) {
  const node = ctx.nodeByAnchor.get(block.anchor) ?? null
  const focused = ctx.focusedAnchor === block.anchor
  const editing = node !== null && ctx.editingAnchor === node.anchorId
  const refs = ctx.refsByAnchor.get(block.anchor) ?? []

  if (block.kind === 'page_break') {
    return (
      <div
        ref={(el) => ctx.registerRef(block.anchor, el)}
        className="my-3 flex items-center gap-2 text-[10px] text-neutral-400"
      >
        <span className="h-px flex-1 bg-neutral-300" />
        分页
        <span className="h-px flex-1 bg-neutral-300" />
      </div>
    )
  }

  return (
    <div
      ref={(el) => ctx.registerRef(block.anchor, el)}
      className={
        'group relative ' + (focused ? 'rounded-[3px] bg-amber-100/60 ring-1 ring-amber-400' : '')
      }
    >
      {node ? (
        <div className="pointer-events-none absolute right-0 top-0 z-10 flex translate-y-[-60%] items-center gap-1 rounded-full border border-neutral-300 bg-white/95 px-1.5 py-0.5 text-[10px] text-neutral-500 opacity-0 shadow-sm transition-opacity group-hover:pointer-events-auto group-hover:opacity-100">
          <span className="font-mono tabular-nums">{node.anchorId}</span>
          {node.mandatoryTags.length > 0 ? (
            <span className="font-medium text-amber-600">{node.mandatoryTags.join('/')}</span>
          ) : null}
          {refs.length > 0 ? <span className="text-brand">引用 {refs.join('、')}</span> : null}
          <button
            type="button"
            onClick={() => ctx.onToggleLock(node)}
            title={node.locked ? '已锁定，点击解锁' : '锁定关键条款防误改'}
            className={
              'inline-flex items-center gap-0.5 ' +
              (node.locked ? 'font-medium text-amber-600' : 'hover:text-neutral-900')
            }
          >
            {node.locked ? <Lock size={10} /> : <Unlock size={10} />}
            {node.locked ? '已锁' : '锁定'}
          </button>
          {block.kind === 'para' && node.xmlLocator.startsWith('p:') ? (
            <button
              type="button"
              onClick={() => ctx.onStartEdit(node)}
              className="inline-flex items-center gap-0.5 hover:text-neutral-900"
            >
              <Pencil size={10} />
              修订
            </button>
          ) : null}
        </div>
      ) : null}

      {block.kind === 'table' && block.table ? (
        <DocTable table={block.table} />
      ) : block.para ? (
        <DocPara para={block.para} />
      ) : null}

      {editing && node ? (
        <div className="my-1.5 space-y-2 rounded-[6px] border border-brand/50 bg-white p-2">
          <textarea
            rows={3}
            value={ctx.editDraft}
            onChange={(e) => ctx.onDraftChange(e.target.value)}
            className="w-full rounded-[6px] border border-neutral-300 px-2 py-1.5 text-[13px] leading-6 text-neutral-900 outline-none focus:border-brand"
          />
          <div className="flex flex-wrap items-center justify-between gap-2 text-[11px]">
            <div className="flex items-center gap-3 text-neutral-500">
              <label className="inline-flex items-center gap-1">
                <input
                  type="radio"
                  name="patchMode"
                  checked={ctx.editMode === 'redline'}
                  onChange={() => ctx.onModeChange('redline')}
                  className="accent-[var(--brand)]"
                />
                修订红线
              </label>
              <label className="inline-flex items-center gap-1">
                <input
                  type="radio"
                  name="patchMode"
                  checked={ctx.editMode === 'clean'}
                  onChange={() => ctx.onModeChange('clean')}
                  className="accent-[var(--brand)]"
                />
                净稿覆盖
              </label>
            </div>
            <div className="flex items-center gap-1.5">
              <button
                type="button"
                onClick={ctx.onCancelEdit}
                className="h-6 rounded-field border border-neutral-300 px-2 text-neutral-500 hover:text-neutral-900"
              >
                取消
              </button>
              <button
                type="button"
                onClick={() => ctx.onCommitEdit(node)}
                className="inline-flex h-6 items-center gap-1 rounded-field bg-brand px-2 font-medium text-brand-fg hover:bg-brand-hover"
              >
                <Check size={11} />
                保存版本
              </button>
            </div>
          </div>
        </div>
      ) : null}
    </div>
  )
}

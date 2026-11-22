export type NodeKind = 'heading' | 'paragraph' | 'list_item' | 'table' | 'table_chunk'
export type ExportMode = 'redline' | 'clean' | 'raw'
export type DiffKind = 'equal' | 'modified' | 'inserted' | 'deleted' | 'moved'
export type SpanOp = 'equal' | 'insert' | 'delete'

export type DocRecord = {
  id: string
  ownerUserId: string
  spaceId: string
  title: string
  filename: string
  currentRev: number
  sourceFormat: string
  createdAtMs: number
  updatedAtMs: number
}

export type DocRevisionMeta = {
  docId: string
  rev: number
  note: string
  authorName: string
  exportMode: string
  nodeCount: number
  tableCount: number
  mandatoryCount: number
  byteSize: number
  createdAtMs: number
}

export type DocNodeRecord = {
  ord: number
  anchorId: string
  parentAnchorId: string | null
  explicitAnchor: string | null
  xmlLocator: string
  nodeKind: NodeKind
  headingPath: string
  textContent: string
  textHash: string
  rowKeys: string[]
  mandatoryTags: string[]
  locked: boolean
  tokenEstimate: number
  isSearchChunk: boolean
}

export type DocRefRecord = {
  sourceAnchor: string
  targetHint: string
}

export type DocDetail = {
  doc: DocRecord
  revisions: DocRevisionMeta[]
  nodes: DocNodeRecord[]
  refs: DocRefRecord[]
}

export type DocSearchHit = {
  hitAnchorId: string
  hitText: string
  headingPath: string
  parentNode: DocNodeRecord | null
  relatedRefs: DocRefRecord[]
}

export type DiffSpan = {
  op: SpanOp
  text: string
}

export type TableRowDiff = {
  rowKey: string
  op: SpanOp
  beforeRow: string | null
  afterRow: string | null
}

export type DiffBlock = {
  anchorId: string
  headingPath: string
  nodeKind: NodeKind
  diffKind: DiffKind
  beforeText: string
  afterText: string
  spans: DiffSpan[]
  addedChars: number
  deletedChars: number
  locked: boolean
  tableRowDiffs: TableRowDiff[]
}

export type DocDiffReport = {
  docId: string
  fromRev: number
  toRev: number
  addedChars: number
  deletedChars: number
  changedBlocks: number
  lockedConflicts: string[]
  blocks: DiffBlock[]
}

// ── 渲染层（人看的版式，来自 /api/docs/{id}/render）─────────────────────────

export type PageSetup = {
  widthTwips: number
  heightTwips: number
  marginTopTwips: number
  marginRightTwips: number
  marginBottomTwips: number
  marginLeftTwips: number
  headerTwips: number
  footerTwips: number
  gutterTwips: number
  landscape: boolean
}

export type RunFmt = {
  bold: boolean
  italic: boolean
  underline: boolean
  strike: boolean
  sizeHalfPt: number
  color: string
  font: string
  vertAlign: string
  highlight: string
}

export type ImageRef = {
  /** docx 包内路径，如 word/media/image1.png */
  path: string
  /** 0 表示按原图尺寸 */
  widthPx: number
  heightPx: number
}

export type RunSpan = {
  text: string
  fmt: RunFmt
  image?: ImageRef
}

export type ParaFmt = {
  styleId: string
  align: string
  indentLeftTwips: number
  indentRightTwips: number
  indentFirstLineTwips: number
  spaceBeforeTwips: number
  spaceAfterTwips: number
  lineTwips: number
  lineRule: string
  numId: string
  numLevel: number
  headingLevel: number
  keepNext: boolean
  pageBreakBefore: boolean
}

export type RPara = {
  ord: number
  fmt: ParaFmt
  runs: RunSpan[]
}

export type RTableCell = {
  gridSpan: number
  vMerge: string
  widthTwips: number
  paras: RPara[]
}

export type RTableRow = {
  heightTwips: number
  header: boolean
  cells: RTableCell[]
}

export type RTable = {
  firstOrd: number
  grid: number[]
  rows: RTableRow[]
  widthTwips: number
  align: string
}

export type RenderBlock = {
  kind: 'para' | 'table' | 'page_break'
  ord: number
  anchor: string
  headingLevel: number
  headingPath: string
  para?: RPara
  table?: RTable
}

export type RenderSlice = {
  docId: string
  rev: number
  page: PageSetup
  blocks: RenderBlock[]
  total: number
  firstOrd: number
  lastOrd: number
  firstSeq: number
  lastSeq: number
  hasMore: boolean
  paraCount: number
  tableCount: number
  pageBreakCount: number
}

export class DocClientError extends Error {
  status: number
  code: string
  needsHumanSaveAsDocx: boolean

  constructor(status: number, code: string, message: string, needsHumanSaveAsDocx = false) {
    super(message)
    this.name = 'DocClientError'
    this.status = status
    this.code = code
    this.needsHumanSaveAsDocx = needsHumanSaveAsDocx
  }
}

const OLE2_MAGIC = [0xd0, 0xcf, 0x11, 0xe0, 0xa1, 0xb1, 0x1a, 0xe1]
const MAX_UPLOAD_BYTES = 50 * 1024 * 1024

export async function inspectDocFileBeforeUpload(file: File): Promise<Uint8Array> {
  if (file.size === 0) {
    throw new DocClientError(400, 'InvalidInput', '所选文件为空，请重新选择 .docx 文档。')
  }
  if (file.size > MAX_UPLOAD_BYTES) {
    throw new DocClientError(400, 'InvalidInput', '文档大小超过 50 MB 上限。')
  }
  const buf = new Uint8Array(await file.arrayBuffer())
  const isOle2 = OLE2_MAGIC.every((b, idx) => buf[idx] === b)
  const lowerName = file.name.toLowerCase()
  if (isOle2 || lowerName.endsWith('.doc') || lowerName.endsWith('.wps')) {
    if (!(buf[0] === 0x50 && buf[1] === 0x4b)) {
      throw new DocClientError(
        400,
        'LegacyDocFormat',
        '检测到二进制 .doc / .wps 老格式。为保证公文红头、跨页表与字体零失真，请先在本机 Word 或 WPS 中「另存为 .docx」并核对版式后再上传。',
        true,
      )
    }
  }
  if (!(buf[0] === 0x50 && buf[1] === 0x4b)) {
    throw new DocClientError(
      400,
      'InvalidInput',
      '文件头不是有效的 OOXML (.docx) 压缩包，请确认文件未损坏。',
    )
  }
  return buf
}

async function tryDocJson<T>(path: string, init?: RequestInit): Promise<T | null> {
  let res: Response
  try {
    res = await fetch(path, {
      ...init,
      credentials: 'include',
    })
  } catch {
    return null
  }

  const ct = res.headers.get('content-type') ?? ''
  if (!ct.includes('application/json')) {
    return null
  }

  const payload = (await res.json()) as T & { code?: string; message?: string }
  if (!res.ok) {
    const msg = payload.message || `文档请求失败 (${res.status})`
    const code = payload.code || 'RequestError'
    throw new DocClientError(
      res.status,
      code,
      msg,
      msg.includes('另存为 .docx') || msg.includes('.doc/.wps'),
    )
  }
  return payload
}

// ── Browser-native OOXML + Diff Fallback (when API_TARGET backend is offline) ──

type PreviewStoredDoc = {
  detail: DocDetail
  revisionsNodes: Record<number, DocNodeRecord[]>
  revisionsRawXml: Record<number, string>
  revisionsZip: Record<number, Uint8Array>
}

const previewDocsBySpace = new Map<string, PreviewStoredDoc[]>()

function textHash(raw: string): string {
  let h1 = 0x811c9dc5
  let h2 = 0x9e3779b9
  for (let i = 0; i < raw.length; i++) {
    const c = raw.charCodeAt(i)
    h1 = Math.imul(h1 ^ c, 0x01000193)
    h2 = Math.imul(h2 ^ (c + i), 0x85ebca6b)
  }
  return `${(h1 >>> 0).toString(16).padStart(8, '0')}${(h2 >>> 0).toString(16).padStart(8, '0')}`
}

function estimateTokens(text: string): number {
  let cjk = 0
  let ascii = 0
  for (const ch of text) {
    if (ch.charCodeAt(0) > 127) cjk++
    else ascii++
  }
  return Math.max(1, cjk + Math.ceil(ascii / 4))
}

function extractMandatoryTags(text: string): string[] {
  const checks: [string, string][] = [
    ['★', '★星标'],
    ['▲', '▲重要'],
    ['※', '※重点'],
    ['废标', '废标红线'],
    ['无效投标', '无效投标'],
    ['实质性', '实质性要求'],
    ['必须满足', '必须满足'],
  ]
  return checks.filter(([needle]) => text.includes(needle)).map(([, label]) => label)
}

function extractCrossRefs(text: string): string[] {
  const out: string[] = []
  const seen = new Set<string>()
  const re = /(表\s*\d+(?:[-.]\d+)?|附件\s*\d+|第\s*\d+(?:\.\d+)*\s*[条章节款])/g
  for (const m of text.matchAll(re)) {
    const val = (m[0] ?? '').trim()
    if (val && !seen.has(val)) {
      seen.add(val)
      out.push(val)
    }
  }
  return out
}

function tokenizeDiff(text: string): string[] {
  const out: string[] = []
  let ascii = ''
  let ws = ''
  for (const ch of text) {
    if (/[a-zA-Z0-9_.-]/.test(ch)) {
      if (ws) {
        out.push(ws)
        ws = ''
      }
      ascii += ch
    } else if (/\s/.test(ch)) {
      if (ascii) {
        out.push(ascii)
        ascii = ''
      }
      ws += ch
    } else {
      if (ascii) {
        out.push(ascii)
        ascii = ''
      }
      if (ws) {
        out.push(ws)
        ws = ''
      }
      out.push(ch)
    }
  }
  if (ascii) out.push(ascii)
  if (ws) out.push(ws)
  return out
}

export function diffTextSpansClient(
  before: string,
  after: string,
): { spans: DiffSpan[]; addedChars: number; deletedChars: number } {
  if (before === after) {
    return {
      spans: before ? [{ op: 'equal', text: before }] : [],
      addedChars: 0,
      deletedChars: 0,
    }
  }
  const a = tokenizeDiff(before)
  const b = tokenizeDiff(after)
  const m = a.length
  const n = b.length
  if (m * n > 120_000) {
    const spans: DiffSpan[] = []
    if (before) spans.push({ op: 'delete', text: before })
    if (after) spans.push({ op: 'insert', text: after })
    return { spans, addedChars: after.length, deletedChars: before.length }
  }

  const dp: number[][] = Array.from({ length: m + 1 }, () => new Array<number>(n + 1).fill(0))
  for (let i = m - 1; i >= 0; i--) {
    const row = dp[i]!
    const nextRow = dp[i + 1]!
    for (let j = n - 1; j >= 0; j--) {
      if (a[i] === b[j]) {
        row[j] = (nextRow[j + 1] ?? 0) + 1
      } else {
        row[j] = Math.max(nextRow[j] ?? 0, row[j + 1] ?? 0)
      }
    }
  }

  const raw: DiffSpan[] = []
  let i = 0
  let j = 0
  while (i < m && j < n) {
    if (a[i] === b[j]) {
      raw.push({ op: 'equal', text: a[i]! })
      i++
      j++
    } else if ((dp[i + 1]?.[j] ?? 0) >= (dp[i]?.[j + 1] ?? 0)) {
      raw.push({ op: 'delete', text: a[i]! })
      i++
    } else {
      raw.push({ op: 'insert', text: b[j]! })
      j++
    }
  }
  while (i < m) raw.push({ op: 'delete', text: a[i++]! })
  while (j < n) raw.push({ op: 'insert', text: b[j++]! })

  const merged: DiffSpan[] = []
  for (const s of raw) {
    if (!s.text) continue
    const last = merged[merged.length - 1]
    if (last && last.op === s.op) {
      last.text += s.text
    } else {
      merged.push({ ...s })
    }
  }

  let addedChars = 0
  let deletedChars = 0
  for (const s of merged) {
    if (s.op === 'insert') addedChars += s.text.length
    if (s.op === 'delete') deletedChars += s.text.length
  }
  return { spans: merged, addedChars, deletedChars }
}

function diceSim(a: string, b: string): number {
  const ca = [...a.replace(/\s+/g, '')]
  const cb = [...b.replace(/\s+/g, '')]
  if (ca.length < 2 || cb.length < 2) return a === b ? 1 : 0
  const map = new Map<string, number>()
  for (let i = 0; i < ca.length - 1; i++) {
    const bg = `${ca[i]}${ca[i + 1]}`
    map.set(bg, (map.get(bg) ?? 0) + 1)
  }
  let hits = 0
  for (let j = 0; j < cb.length - 1; j++) {
    const bg = `${cb[j]}${cb[j + 1]}`
    const cnt = map.get(bg) ?? 0
    if (cnt > 0) {
      map.set(bg, cnt - 1)
      hits++
    }
  }
  return (2 * hits) / (ca.length - 1 + (cb.length - 1))
}

function computeClientDocDiff(
  docId: string,
  fromRev: number,
  toRev: number,
  oldAll: DocNodeRecord[],
  newAll: DocNodeRecord[],
): DocDiffReport {
  const oldNodes = oldAll.filter((n) => !n.isSearchChunk)
  const newNodes = newAll.filter((n) => !n.isSearchChunk)
  const oldMatched = new Array<boolean>(oldNodes.length).fill(false)
  const newMatched = new Array<boolean>(newNodes.length).fill(false)
  const pairByNew = new Map<number, { oldIdx: number; kind: DiffKind }>()

  // Level 1: explicit anchor
  const oldExp = new Map<string, number>()
  oldNodes.forEach((n, idx) => {
    if (n.explicitAnchor && !oldExp.has(n.explicitAnchor)) oldExp.set(n.explicitAnchor, idx)
  })
  newNodes.forEach((n, j) => {
    if (n.explicitAnchor) {
      const i = oldExp.get(n.explicitAnchor)
      if (i !== undefined && !oldMatched[i]) {
        oldMatched[i] = true
        newMatched[j] = true
        pairByNew.set(j, {
          oldIdx: i,
          kind: oldNodes[i]!.textHash === n.textHash ? 'equal' : 'modified',
        })
      }
    }
  })

  // Level 2: exact hash
  newNodes.forEach((n, j) => {
    if (newMatched[j]) return
    const i = oldNodes.findIndex(
      (o, idx) => !oldMatched[idx] && o.textHash === n.textHash && o.headingPath === n.headingPath,
    )
    if (i >= 0) {
      oldMatched[i] = true
      newMatched[j] = true
      pairByNew.set(j, { oldIdx: i, kind: 'equal' })
    }
  })

  // Level 3: fuzzy similarity
  newNodes.forEach((n, j) => {
    if (newMatched[j]) return
    let bestIdx = -1
    let bestScore = 0
    oldNodes.forEach((o, i) => {
      if (oldMatched[i]) return
      const s = diceSim(o.textContent, n.textContent) + (o.headingPath === n.headingPath ? 0.12 : 0)
      if (s >= 0.28 && s > bestScore) {
        bestScore = s
        bestIdx = i
      }
    })
    if (bestIdx >= 0) {
      oldMatched[bestIdx] = true
      newMatched[j] = true
      pairByNew.set(j, { oldIdx: bestIdx, kind: 'modified' })
    }
  })

  const blocks: DiffBlock[] = []
  let addedChars = 0
  let deletedChars = 0
  let changedBlocks = 0
  const lockedConflicts: string[] = []
  const emittedOld = new Set<number>()
  let oldCursor = 0

  newNodes.forEach((n, j) => {
    const p = pairByNew.get(j)
    while (oldCursor < oldNodes.length) {
      if (oldMatched[oldCursor]) {
        oldCursor++
        continue
      }
      if (p && oldCursor < p.oldIdx && !emittedOld.has(oldCursor)) {
        const o = oldNodes[oldCursor]!
        emittedOld.add(oldCursor)
        const delC = o.textContent.length
        deletedChars += delC
        changedBlocks++
        if (o.locked) lockedConflicts.push(o.anchorId)
        blocks.push({
          anchorId: o.anchorId,
          headingPath: o.headingPath,
          nodeKind: o.nodeKind,
          diffKind: 'deleted',
          beforeText: o.textContent,
          afterText: '',
          spans: [{ op: 'delete', text: o.textContent }],
          addedChars: 0,
          deletedChars: delC,
          locked: o.locked,
          tableRowDiffs: [],
        })
        oldCursor++
        continue
      }
      break
    }

    if (p) {
      const o = oldNodes[p.oldIdx]!
      emittedOld.add(p.oldIdx)
      if (p.kind === 'equal') {
        blocks.push({
          anchorId: n.anchorId,
          headingPath: n.headingPath,
          nodeKind: n.nodeKind,
          diffKind: 'equal',
          beforeText: o.textContent,
          afterText: n.textContent,
          spans: [{ op: 'equal', text: n.textContent }],
          addedChars: 0,
          deletedChars: 0,
          locked: o.locked || n.locked,
          tableRowDiffs: [],
        })
      } else {
        const d = diffTextSpansClient(o.textContent, n.textContent)
        addedChars += d.addedChars
        deletedChars += d.deletedChars
        changedBlocks++
        const isLocked = o.locked || n.locked
        if (isLocked && (d.addedChars > 0 || d.deletedChars > 0)) {
          lockedConflicts.push(n.anchorId)
        }
        blocks.push({
          anchorId: n.anchorId,
          headingPath: n.headingPath,
          nodeKind: n.nodeKind,
          diffKind: 'modified',
          beforeText: o.textContent,
          afterText: n.textContent,
          spans: d.spans,
          addedChars: d.addedChars,
          deletedChars: d.deletedChars,
          locked: isLocked,
          tableRowDiffs: [],
        })
      }
    } else {
      const addC = n.textContent.length
      addedChars += addC
      changedBlocks++
      blocks.push({
        anchorId: n.anchorId,
        headingPath: n.headingPath,
        nodeKind: n.nodeKind,
        diffKind: 'inserted',
        beforeText: '',
        afterText: n.textContent,
        spans: [{ op: 'insert', text: n.textContent }],
        addedChars: addC,
        deletedChars: 0,
        locked: n.locked,
        tableRowDiffs: [],
      })
    }
  })

  oldNodes.forEach((o, i) => {
    if (!oldMatched[i] && !emittedOld.has(i)) {
      const delC = o.textContent.length
      deletedChars += delC
      changedBlocks++
      if (o.locked) lockedConflicts.push(o.anchorId)
      blocks.push({
        anchorId: o.anchorId,
        headingPath: o.headingPath,
        nodeKind: o.nodeKind,
        diffKind: 'deleted',
        beforeText: o.textContent,
        afterText: '',
        spans: [{ op: 'delete', text: o.textContent }],
        addedChars: 0,
        deletedChars: delC,
        locked: o.locked,
        tableRowDiffs: [],
      })
    }
  })

  return {
    docId,
    fromRev,
    toRev,
    addedChars,
    deletedChars,
    changedBlocks,
    lockedConflicts,
    blocks,
  }
}

// ── Lightweight Client-side ZIP + OOXML Parser for Standalone Preview ────────

async function inflateRaw(compressed: Uint8Array): Promise<Uint8Array> {
  const ds = new DecompressionStream('deflate-raw')
  const stream = new Blob([compressed as BlobPart]).stream().pipeThrough(ds)
  const buf = await new Response(stream).arrayBuffer()
  return new Uint8Array(buf)
}

async function readDocxEntryText(zipBytes: Uint8Array, targetPath: string): Promise<string | null> {
  const dv = new DataView(zipBytes.buffer, zipBytes.byteOffset, zipBytes.byteLength)
  let eocdOffset = -1
  for (let i = zipBytes.length - 22; i >= Math.max(0, zipBytes.length - 65558); i--) {
    if (dv.getUint32(i, true) === 0x06054b50) {
      eocdOffset = i
      break
    }
  }
  if (eocdOffset < 0) {
    throw new DocClientError(400, 'InvalidInput', '.docx 压缩目录结构损坏')
  }

  const cdOffset = dv.getUint32(eocdOffset + 16, true)
  const totalEntries = dv.getUint16(eocdOffset + 10, true)
  const decoder = new TextDecoder('utf-8')
  let cursor = cdOffset

  for (let idx = 0; idx < totalEntries; idx++) {
    if (cursor + 46 > zipBytes.length || dv.getUint32(cursor, true) !== 0x02014b50) break
    const method = dv.getUint16(cursor + 10, true)
    const compSize = dv.getUint32(cursor + 20, true)
    const nameLen = dv.getUint16(cursor + 28, true)
    const extraLen = dv.getUint16(cursor + 30, true)
    const commentLen = dv.getUint16(cursor + 32, true)
    const localOffset = dv.getUint32(cursor + 42, true)
    const name = decoder.decode(zipBytes.subarray(cursor + 46, cursor + 46 + nameLen))
    cursor += 46 + nameLen + extraLen + commentLen

    if (name === targetPath) {
      const lNameLen = dv.getUint16(localOffset + 26, true)
      const lExtraLen = dv.getUint16(localOffset + 28, true)
      const dataStart = localOffset + 30 + lNameLen + lExtraLen
      const payload = zipBytes.subarray(dataStart, dataStart + compSize)
      const raw = method === 0 ? payload : await inflateRaw(payload)
      return decoder.decode(raw)
    }
  }
  return null
}

function parseDocxXmlBrowser(docXml: string): {
  nodes: DocNodeRecord[]
  refs: DocRefRecord[]
  nodeCount: number
  tableCount: number
  mandatoryCount: number
} {
  const parser = new DOMParser()
  const dom = parser.parseFromString(docXml, 'application/xml')
  const body = dom.getElementsByTagNameNS('*', 'body')[0] ?? dom.documentElement
  const nodes: DocNodeRecord[] = []
  const refs: DocRefRecord[] = []
  const headingStack: { lvl: number; anchorId: string; title: string }[] = []
  let pOrd = 0
  let tblOrd = 0
  let tableCount = 0
  let mandatoryCount = 0

  const extractParaText = (pEl: Element): string => {
    const out: string[] = []
    const walk = (el: Element) => {
      const ln = el.localName
      if (ln === 'Fallback' || ln === 'proofErr' || ln === 'del' || ln === 'delText') return
      if (ln === 't') {
        out.push(el.textContent ?? '')
        return
      }
      if (ln === 'tab') {
        out.push(' ')
        return
      }
      if (ln === 'br' || ln === 'cr') {
        out.push('\n')
        return
      }
      for (const child of Array.from(el.children)) {
        walk(child)
      }
    }
    walk(pEl)
    return out.join('').replace(/[ \t]+/g, ' ').trim()
  }

  const children = Array.from(body.children)
  for (const child of children) {
    if (child.localName === 'p') {
      const curOrd = pOrd++
      const text = extractParaText(child)
      if (!text) continue

      const paraId =
        child.getAttribute('w14:paraId') ||
        child.getAttribute('paraId') ||
        null
      const pStyle = child.getElementsByTagNameNS('*', 'pStyle')[0]?.getAttribute('w:val') ?? ''
      const outlineRaw = child.getElementsByTagNameNS('*', 'outlineLvl')[0]?.getAttribute('w:val')
      const isList = child.getElementsByTagNameNS('*', 'numPr').length > 0

      let headingLvl: number | null = null
      if (outlineRaw !== null && outlineRaw !== undefined) {
        const n = Number(outlineRaw)
        if (!Number.isNaN(n)) headingLvl = Math.min(6, n + 1)
      } else if (/^(Heading|标题)\s*[1-6]/i.test(pStyle)) {
        headingLvl = Number(pStyle.match(/[1-6]/)?.[0] ?? 1)
      } else if (text.length <= 42 && (/^第[一二三四五六七八九十0-9]+[卷章节]/.test(text) || /^\d+(\.\d+){0,3}\s+\S/.test(text))) {
        headingLvl = text.startsWith('第') && !text.includes('节') ? 1 : 2
      }

      if (headingLvl !== null) {
        while (headingStack.length > 0 && headingStack[headingStack.length - 1]!.lvl >= headingLvl) {
          headingStack.pop()
        }
      }

      const parentAnchorId = headingStack[headingStack.length - 1]?.anchorId ?? null
      const headingPath =
        headingLvl !== null
          ? [...headingStack.map((h) => h.title), text].join(' / ')
          : headingStack.length > 0
            ? headingStack.map((h) => h.title).join(' / ')
            : '文档正文'

      const anchorId = paraId ? `a_${paraId}` : `p_${curOrd}`
      const mandatoryTags = extractMandatoryTags(text)
      if (mandatoryTags.length > 0) mandatoryCount++
      for (const t of extractCrossRefs(text)) {
        refs.push({ sourceAnchor: anchorId, targetHint: t })
      }

      nodes.push({
        ord: nodes.length,
        anchorId,
        parentAnchorId,
        explicitAnchor: paraId,
        xmlLocator: `p:${curOrd}`,
        nodeKind: headingLvl !== null ? 'heading' : isList ? 'list_item' : 'paragraph',
        headingPath,
        textContent: text,
        textHash: textHash(text),
        rowKeys: [],
        mandatoryTags,
        locked: false,
        tokenEstimate: estimateTokens(text),
        isSearchChunk: false,
      })

      if (headingLvl !== null) {
        headingStack.push({ lvl: headingLvl, anchorId, title: text })
      }
    } else if (child.localName === 'tbl') {
      const trList = Array.from(child.children).filter((c) => c.localName === 'tr')
      const rows: string[][] = []
      let firstP: number | null = null
      for (const tr of trList) {
        const rowCells: string[] = []
        const tcList = Array.from(tr.children).filter((c) => c.localName === 'tc')
        for (const tc of tcList) {
          const cellTexts: string[] = []
          for (const p of Array.from(tc.getElementsByTagNameNS('*', 'p'))) {
            if (firstP === null) firstP = pOrd
            pOrd++
            const pt = extractParaText(p)
            if (pt) cellTexts.push(pt)
          }
          rowCells.push(cellTexts.join(' '))
        }
        if (rowCells.some((c) => c.trim())) rows.push(rowCells)
      }
      if (rows.length > 0) {
        tableCount++
        const curTbl = tblOrd++
        const lines = rows.map((r) => r.join(' | '))
        const rowKeys = rows.map((r) =>
          r
            .map((c) => c.trim())
            .filter(Boolean)
            .slice(0, 2)
            .join('|'),
        )
        const fullText = lines.join('\n')
        const anchorId = `tbl_${curTbl}`
        const parentAnchorId = headingStack[headingStack.length - 1]?.anchorId ?? null
        const headingPath =
          headingStack.length > 0 ? headingStack.map((h) => h.title).join(' / ') : '文档正文'
        const mandatoryTags = extractMandatoryTags(fullText)
        if (mandatoryTags.length > 0) mandatoryCount++
        nodes.push({
          ord: nodes.length,
          anchorId,
          parentAnchorId,
          explicitAnchor: anchorId,
          xmlLocator: firstP !== null ? `p:${firstP}` : `tbl:${curTbl}`,
          nodeKind: 'table',
          headingPath,
          textContent: fullText,
          textHash: textHash(fullText),
          rowKeys,
          mandatoryTags,
          locked: false,
          tokenEstimate: estimateTokens(fullText),
          isSearchChunk: false,
        })
      }
    }
  }

  return {
    nodes,
    refs,
    nodeCount: nodes.filter((n) => !n.isSearchChunk).length,
    tableCount,
    mandatoryCount,
  }
}

function crc32(bytes: Uint8Array): number {
  let c = 0xffffffff
  for (let i = 0; i < bytes.length; i++) {
    c ^= bytes[i]!
    for (let k = 0; k < 8; k++) {
      c = (c >>> 1) ^ (c & 1 ? 0xedb88320 : 0)
    }
  }
  return (c ^ 0xffffffff) >>> 0
}

function buildMinimalZip(entries: { name: string; data: Uint8Array }[]): Uint8Array {
  const enc = new TextEncoder()
  const localParts: Uint8Array[] = []
  const cdParts: Uint8Array[] = []
  let offset = 0

  for (const e of entries) {
    const nameBytes = enc.encode(e.name)
    const crc = crc32(e.data)
    const lh = new Uint8Array(30 + nameBytes.length)
    const lv = new DataView(lh.buffer)
    lv.setUint32(0, 0x04034b50, true)
    lv.setUint16(4, 20, true)
    lv.setUint16(8, 0, true) // Stored
    lv.setUint32(14, crc, true)
    lv.setUint32(18, e.data.length, true)
    lv.setUint32(22, e.data.length, true)
    lv.setUint16(26, nameBytes.length, true)
    lh.set(nameBytes, 30)

    localParts.push(lh, e.data)

    const ch = new Uint8Array(46 + nameBytes.length)
    const cv = new DataView(ch.buffer)
    cv.setUint32(0, 0x02014b50, true)
    cv.setUint16(4, 20, true)
    cv.setUint16(6, 20, true)
    cv.setUint16(10, 0, true)
    cv.setUint32(16, crc, true)
    cv.setUint32(20, e.data.length, true)
    cv.setUint32(24, e.data.length, true)
    cv.setUint16(28, nameBytes.length, true)
    cv.setUint32(42, offset, true)
    ch.set(nameBytes, 46)
    cdParts.push(ch)

    offset += lh.length + e.data.length
  }

  const cdSize = cdParts.reduce((s, p) => s + p.length, 0)
  const eocd = new Uint8Array(22)
  const ev = new DataView(eocd.buffer)
  ev.setUint32(0, 0x06054b50, true)
  ev.setUint16(8, entries.length, true)
  ev.setUint16(10, entries.length, true)
  ev.setUint32(12, cdSize, true)
  ev.setUint32(16, offset, true)

  const totalLen = offset + cdSize + 22
  const out = new Uint8Array(totalLen)
  let pos = 0
  for (const p of localParts) {
    out.set(p, pos)
    pos += p.length
  }
  for (const p of cdParts) {
    out.set(p, pos)
    pos += p.length
  }
  out.set(eocd, pos)
  return out
}

function escapeXml(raw: string): string {
  return raw
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
}

function buildPreviewDocxBytes(
  nodes: DocNodeRecord[],
  prevNodes: DocNodeRecord[] | null,
  mode: ExportMode,
): Uint8Array {
  const enc = new TextEncoder()
  const prevMap = new Map((prevNodes ?? []).map((n) => [n.anchorId, n]))
  const nowIso = new Date().toISOString().slice(0, 19) + 'Z'
  let revId = 1

  const bodyXml = nodes
    .filter((n) => !n.isSearchChunk)
    .map((n) => {
      if (n.nodeKind === 'table') {
        const rows = n.textContent
          .split('\n')
          .map((line) => {
            const cells = line
              .split('|')
              .map(
                (c) =>
                  `<w:tc><w:p><w:r><w:t xml:space="preserve">${escapeXml(c.trim())}</w:t></w:r></w:p></w:tc>`,
              )
              .join('')
            return `<w:tr>${cells}</w:tr>`
          })
          .join('')
        return `<w:tbl>${rows}</w:tbl>`
      }

      const prev = prevMap.get(n.anchorId)
      if (mode === 'redline' && prev && prev.textContent !== n.textContent) {
        const { spans } = diffTextSpansClient(prev.textContent, n.textContent)
        const runs = spans
          .map((s) => {
            if (s.op === 'equal') {
              return `<w:r><w:t xml:space="preserve">${escapeXml(s.text)}</w:t></w:r>`
            }
            if (s.op === 'delete') {
              return `<w:del w:id="${revId++}" w:author="工作台审校" w:date="${nowIso}"><w:r><w:delText xml:space="preserve">${escapeXml(s.text)}</w:delText></w:r></w:del>`
            }
            return `<w:ins w:id="${revId++}" w:author="工作台审校" w:date="${nowIso}"><w:r><w:t xml:space="preserve">${escapeXml(s.text)}</w:t></w:r></w:ins>`
          })
          .join('')
        return `<w:p>${runs}</w:p>`
      }

      const pPr = n.nodeKind === 'heading' ? '<w:pPr><w:pStyle w:val="Heading1"/></w:pPr>' : ''
      return `<w:p>${pPr}<w:r><w:t xml:space="preserve">${escapeXml(n.textContent)}</w:t></w:r></w:p>`
    })
    .join('')

  const documentXml = `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>${bodyXml}</w:body>
</w:document>`

  const contentTypes = `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>`

  const rels = `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>`

  return buildMinimalZip([
    { name: '[Content_Types].xml', data: enc.encode(contentTypes) },
    { name: '_rels/.rels', data: enc.encode(rels) },
    { name: 'word/document.xml', data: enc.encode(documentXml) },
  ])
}

function ensurePreviewSeed(spaceId: string): PreviewStoredDoc[] {
  const existing = previewDocsBySpace.get(spaceId)
  if (existing) return existing

  const now = Date.now()
  const docId = `doc_seed_${spaceId}`
  const makeNode = (
    ord: number,
    anchorId: string,
    parentAnchorId: string | null,
    nodeKind: NodeKind,
    headingPath: string,
    text: string,
    locked = false,
  ): DocNodeRecord => ({
    ord,
    anchorId,
    parentAnchorId,
    explicitAnchor: anchorId,
    xmlLocator: `p:${ord}`,
    nodeKind,
    headingPath,
    textContent: text,
    textHash: textHash(text),
    rowKeys: nodeKind === 'table' ? text.split('\n').map((r) => r.split('|').slice(0, 2).map((x) => x.trim()).join('|')) : [],
    mandatoryTags: extractMandatoryTags(text),
    locked,
    tokenEstimate: estimateTokens(text),
    isSearchChunk: false,
  })

  const v1Nodes: DocNodeRecord[] = [
    makeNode(0, 'a_3_0', null, 'heading', '第三章 运维服务方案', '第三章 运维服务方案'),
    makeNode(
      1,
      'a_3_1',
      'a_3_0',
      'paragraph',
      '第三章 运维服务方案',
      '3.1 我方为本项目配置 7×24 小时值守团队，分三班轮换，每班不少于 4 人。值班表见表 2-3。',
    ),
    makeNode(
      2,
      'a_3_2',
      'a_3_0',
      'paragraph',
      '第三章 运维服务方案',
      '★ 3.2 一般故障 30 分钟内响应、2 小时内恢复；重大故障 15 分钟内响应，2 小时内给出处置方案（废标红线条款，不得偏离）。',
      true,
    ),
    makeNode(
      3,
      'a_3_3',
      'a_3_0',
      'paragraph',
      '第三章 运维服务方案',
      '3.3 运维响应时间为 4 个工作小时，所有故障处理过程在运维平台留痕，月度汇总提交甲方。',
    ),
    makeNode(
      4,
      'a_3_4',
      'a_3_0',
      'paragraph',
      '第三章 运维服务方案',
      '3.4 如连续两个季度无重大故障，可下调值守班次为 5×8 小时。',
    ),
    makeNode(
      5,
      'tbl_0',
      'a_3_0',
      'table',
      '第三章 运维服务方案',
      '序号 | 关键服务项 | 响应要求\n1 | 核心交换机备件 | 1:1 备件池储备\n2 | ▲ 驻场值守人数 | 每班不少于 4 人',
    ),
  ]

  const v2Nodes: DocNodeRecord[] = v1Nodes.map((n) => ({ ...n }))
  v2Nodes[3] = {
    ...v2Nodes[3]!,
    textContent: '3.3 运维响应时间为 4 小时，所有故障处理过程在运维平台留痕，月度汇总提交甲方。',
    textHash: textHash('3.3 运维响应时间为 4 小时，所有故障处理过程在运维平台留痕，月度汇总提交甲方。'),
  }
  v2Nodes[4] = {
    ...v2Nodes[4]!,
    textContent: '3.4 如连续两个季度无重大故障，可下调值守班次为 5×8 小时，需甲方书面确认并约定恢复原班次条件。',
    textHash: textHash('3.4 如连续两个季度无重大故障，可下调值守班次为 5×8 小时，需甲方书面确认并约定恢复原班次条件。'),
  }

  const refs: DocRefRecord[] = [{ sourceAnchor: 'a_3_1', targetHint: '表 2-3' }]
  const title = spaceId === 'me' ? '周报_第47周' : spaceId === 'delivery' ? '乙项目_运维交接方案' : '技术标_甲方案_v3'
  const filename = `${title}.docx`

  const doc: DocRecord = {
    id: docId,
    ownerUserId: 'preview',
    spaceId,
    title,
    filename,
    currentRev: 2,
    sourceFormat: 'docx',
    createdAtMs: now - 3_600_000,
    updatedAtMs: now,
  }

  const revisions: DocRevisionMeta[] = [
    {
      docId,
      rev: 2,
      note: '修订 2 处 SLA 响应与班次前置条件',
      authorName: '陈默',
      exportMode: 'redline',
      nodeCount: v2Nodes.length,
      tableCount: 1,
      mandatoryCount: 2,
      byteSize: 48_240,
      createdAtMs: now - 120_000,
    },
    {
      docId,
      rev: 1,
      note: 'Word/WPS 另存 .docx 初稿入库',
      authorName: '李渝',
      exportMode: 'raw',
      nodeCount: v1Nodes.length,
      tableCount: 1,
      mandatoryCount: 2,
      byteSize: 47_810,
      createdAtMs: now - 3_600_000,
    },
  ]

  const seeded: PreviewStoredDoc = {
    detail: { doc, revisions, nodes: v2Nodes, refs },
    revisionsNodes: { 1: v1Nodes, 2: v2Nodes },
    revisionsRawXml: {},
    revisionsZip: {
      1: buildPreviewDocxBytes(v1Nodes, null, 'raw'),
      2: buildPreviewDocxBytes(v2Nodes, v1Nodes, 'redline'),
    },
  }
  const list = [seeded]
  previewDocsBySpace.set(spaceId, list)
  return list
}

function findPreviewDoc(docId: string): PreviewStoredDoc | undefined {
  for (const list of previewDocsBySpace.values()) {
    const found = list.find((d) => d.detail.doc.id === docId)
    if (found) return found
  }
  return undefined
}

// ── Public API ──────────────────────────────────────────────────────────────

export async function listDocs(spaceId: string): Promise<DocRecord[]> {
  const remote = await tryDocJson<{ docs: DocRecord[] }>(
    `/api/docs/?spaceId=${encodeURIComponent(spaceId)}`,
    { method: 'GET' },
  )
  if (remote) return remote.docs
  return ensurePreviewSeed(spaceId).map((d) => d.detail.doc)
}

export async function getDocDetail(docId: string, rev?: number): Promise<DocDetail> {
  const qs = rev !== undefined ? `?rev=${rev}` : ''
  const remote = await tryDocJson<DocDetail>(`/api/docs/${encodeURIComponent(docId)}${qs}`, {
    method: 'GET',
  })
  if (remote) return remote

  const found = findPreviewDoc(docId)
  if (!found) {
    throw new DocClientError(404, 'NotFound', '文档不存在')
  }
  const targetRev = rev ?? found.detail.doc.currentRev
  return {
    ...found.detail,
    nodes: found.revisionsNodes[targetRev] ?? found.detail.nodes,
  }
}

export async function uploadDocFile(
  file: File,
  spaceId: string,
  note = '初稿 .docx 入库',
): Promise<DocDetail> {
  const bytes = await inspectDocFileBeforeUpload(file)

  const form = new FormData()
  form.append('file', file)
  form.append('spaceId', spaceId)
  form.append('note', note)

  const remote = await tryDocJson<DocDetail>('/api/docs/upload', {
    method: 'POST',
    body: form,
  })
  if (remote) return remote

  // Browser-native preview fallback: parse genuine .docx XML in browser
  const docXml = await readDocxEntryText(bytes, 'word/document.xml')
  if (!docXml) {
    throw new DocClientError(400, 'InvalidInput', '压缩包内未找到 word/document.xml')
  }
  const parsed = parseDocxXmlBrowser(docXml)
  const now = Date.now()
  const id = `doc_${now.toString(36)}`
  const title = file.name.replace(/\.docx$/i, '')

  const doc: DocRecord = {
    id,
    ownerUserId: 'preview',
    spaceId,
    title,
    filename: file.name,
    currentRev: 1,
    sourceFormat: 'docx',
    createdAtMs: now,
    updatedAtMs: now,
  }
  const revMeta: DocRevisionMeta = {
    docId: id,
    rev: 1,
    note,
    authorName: '当前成员',
    exportMode: 'raw',
    nodeCount: parsed.nodeCount,
    tableCount: parsed.tableCount,
    mandatoryCount: parsed.mandatoryCount,
    byteSize: bytes.length,
    createdAtMs: now,
  }
  const stored: PreviewStoredDoc = {
    detail: { doc, revisions: [revMeta], nodes: parsed.nodes, refs: parsed.refs },
    revisionsNodes: { 1: parsed.nodes },
    revisionsRawXml: { 1: docXml },
    revisionsZip: { 1: bytes },
  }
  const list = ensurePreviewSeed(spaceId)
  list.unshift(stored)
  return stored.detail
}

export async function uploadDocRevisionFile(
  docId: string,
  file: File,
  note = '回传修订版 .docx',
): Promise<{ detail: DocDetail; diff: DocDiffReport }> {
  const bytes = await inspectDocFileBeforeUpload(file)

  const form = new FormData()
  form.append('file', file)
  form.append('note', note)

  const remote = await tryDocJson<{ detail: DocDetail; diff: DocDiffReport }>(
    `/api/docs/${encodeURIComponent(docId)}/upload-revision`,
    { method: 'POST', body: form },
  )
  if (remote) return remote

  const found = findPreviewDoc(docId)
  if (!found) throw new DocClientError(404, 'NotFound', '文档不存在')

  const docXml = await readDocxEntryText(bytes, 'word/document.xml')
  if (!docXml) {
    throw new DocClientError(400, 'InvalidInput', '压缩包内未找到 word/document.xml')
  }
  const parsed = parseDocxXmlBrowser(docXml)
  const prevRev = found.detail.doc.currentRev
  const prevNodes = found.revisionsNodes[prevRev] ?? found.detail.nodes
  const lockedSet = new Set(prevNodes.filter((n) => n.locked).map((n) => n.anchorId))
  const nextNodes = parsed.nodes.map((n) => ({
    ...n,
    locked: lockedSet.has(n.anchorId) || (n.explicitAnchor ? lockedSet.has(n.explicitAnchor) : false),
  }))

  const nextRev = prevRev + 1
  const now = Date.now()
  found.detail.doc.currentRev = nextRev
  found.detail.doc.updatedAtMs = now
  found.detail.nodes = nextNodes
  found.detail.refs = parsed.refs
  found.detail.revisions.unshift({
    docId,
    rev: nextRev,
    note,
    authorName: 'WPS 回传',
    exportMode: 'raw',
    nodeCount: parsed.nodeCount,
    tableCount: parsed.tableCount,
    mandatoryCount: parsed.mandatoryCount,
    byteSize: bytes.length,
    createdAtMs: now,
  })
  found.revisionsNodes[nextRev] = nextNodes
  found.revisionsRawXml[nextRev] = docXml
  found.revisionsZip[nextRev] = bytes

  const diff = computeClientDocDiff(docId, prevRev, nextRev, prevNodes, nextNodes)
  return { detail: { ...found.detail }, diff }
}

export async function searchDoc(docId: string, query: string): Promise<DocSearchHit[]> {
  const q = query.trim()
  if (!q) return []
  const remote = await tryDocJson<{ hits: DocSearchHit[] }>(
    `/api/docs/${encodeURIComponent(docId)}/search?q=${encodeURIComponent(q)}`,
    { method: 'GET' },
  )
  if (remote) return remote.hits

  const found = findPreviewDoc(docId)
  if (!found) return []
  const lower = q.toLowerCase()
  const byAnchor = new Map(found.detail.nodes.map((n) => [n.anchorId, n]))
  const hits: DocSearchHit[] = []
  for (const n of found.detail.nodes) {
    if (
      n.textContent.toLowerCase().includes(lower) ||
      n.headingPath.toLowerCase().includes(lower)
    ) {
      const parent = n.parentAnchorId ? byAnchor.get(n.parentAnchorId) ?? n : n
      hits.push({
        hitAnchorId: n.anchorId,
        hitText: n.textContent,
        headingPath: n.headingPath,
        parentNode: parent,
        relatedRefs: found.detail.refs.filter((r) => r.sourceAnchor === n.anchorId),
      })
    }
  }
  return hits
}

export async function toggleNodeLock(
  docId: string,
  anchorId: string,
  locked: boolean,
): Promise<DocNodeRecord> {
  const remote = await tryDocJson<{ node: DocNodeRecord }>(
    `/api/docs/${encodeURIComponent(docId)}/nodes/${encodeURIComponent(anchorId)}/lock`,
    {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ locked }),
    },
  )
  if (remote) return remote.node

  const found = findPreviewDoc(docId)
  if (!found) throw new DocClientError(404, 'NotFound', '文档不存在')
  const curNodes = found.revisionsNodes[found.detail.doc.currentRev] ?? found.detail.nodes
  const target = curNodes.find((n) => n.anchorId === anchorId)
  if (!target) throw new DocClientError(404, 'NotFound', '条款节点不存在')
  target.locked = locked
  found.detail.nodes = [...curNodes]
  return { ...target }
}

/**
 * 取版式渲染切片。这是人看到的唯一视图，节点流不参与展示。
 * 服务端没存渲染数据的版本（v5 之前入库）返回 null，由 UI 明确提示。
 */
export async function fetchDocRender(
  docId: string,
  opts: { rev?: number; from?: number; to?: number } = {},
): Promise<RenderSlice | null> {
  const qs = new URLSearchParams()
  if (opts.rev !== undefined) qs.set('rev', String(opts.rev))
  if (opts.from !== undefined) qs.set('from', String(opts.from))
  if (opts.to !== undefined) qs.set('to', String(opts.to))
  const q = qs.toString()
  return tryDocJson<RenderSlice>(
    `/api/docs/${encodeURIComponent(docId)}/render${q ? `?${q}` : ''}`,
    { method: 'GET' },
  )
}

/**
 * 正文图片直出地址：图片从该版本 docx 归档里按需取，前端只当普通 <img> 用。
 */
export function docMediaUrl(docId: string, path: string, rev?: number): string {
  const qs = new URLSearchParams({ path })
  if (rev !== undefined) qs.set('rev', String(rev))
  return `/api/docs/${encodeURIComponent(docId)}/media?${qs.toString()}`
}

export async function fetchDocDiff(
  docId: string,
  fromRev: number,
  toRev: number,
): Promise<DocDiffReport> {
  const remote = await tryDocJson<DocDiffReport>(
    `/api/docs/${encodeURIComponent(docId)}/diff?from=${fromRev}&to=${toRev}`,
    { method: 'GET' },
  )
  if (remote) return remote

  const found = findPreviewDoc(docId)
  if (!found) throw new DocClientError(404, 'NotFound', '文档不存在')
  const oldNodes = found.revisionsNodes[fromRev] ?? []
  const newNodes = found.revisionsNodes[toRev] ?? []
  return computeClientDocDiff(docId, fromRev, toRev, oldNodes, newNodes)
}

export async function patchDocEdits(
  docId: string,
  edits: { anchorId: string; newText: string }[],
  mode: 'redline' | 'clean',
  note?: string,
  force = false,
): Promise<{ detail: DocDetail; diff: DocDiffReport | null }> {
  const remote = await tryDocJson<{ detail: DocDetail; diff: DocDiffReport | null }>(
    `/api/docs/${encodeURIComponent(docId)}/patch`,
    {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ edits, mode, note, force }),
    },
  )
  if (remote) return remote

  const found = findPreviewDoc(docId)
  if (!found) throw new DocClientError(404, 'NotFound', '文档不存在')
  const curRev = found.detail.doc.currentRev
  const curNodes = found.revisionsNodes[curRev] ?? found.detail.nodes

  const lockedHits: string[] = []
  const editMap = new Map(edits.map((e) => [e.anchorId, e.newText]))
  for (const e of edits) {
    const node = curNodes.find((n) => n.anchorId === e.anchorId)
    if (!node) throw new DocClientError(404, 'NotFound', `未找到条款 ${e.anchorId}`)
    if (node.locked && !force) lockedHits.push(node.anchorId)
  }
  if (lockedHits.length > 0) {
    throw new DocClientError(
      409,
      'Conflict',
      `以下条款已锁定，拒绝直接覆盖: ${lockedHits.join(', ')}`,
    )
  }

  const nextNodes: DocNodeRecord[] = curNodes.map((n) => {
    const newText = editMap.get(n.anchorId)
    if (newText === undefined) return { ...n }
    return {
      ...n,
      textContent: newText,
      textHash: textHash(newText),
      mandatoryTags: extractMandatoryTags(newText),
      tokenEstimate: estimateTokens(newText),
    }
  })

  const nextRev = curRev + 1
  const now = Date.now()
  const blob = buildPreviewDocxBytes(nextNodes, curNodes, mode)
  found.detail.doc.currentRev = nextRev
  found.detail.doc.updatedAtMs = now
  found.detail.nodes = nextNodes
  found.detail.revisions.unshift({
    docId,
    rev: nextRev,
    note: note || `修订 ${edits.length} 处（${mode === 'redline' ? '修订红线' : '净稿'}）`,
    authorName: '当前成员',
    exportMode: mode,
    nodeCount: nextNodes.length,
    tableCount: found.detail.revisions[0]?.tableCount ?? 0,
    mandatoryCount: nextNodes.filter((n) => n.mandatoryTags.length > 0).length,
    byteSize: blob.length,
    createdAtMs: now,
  })
  found.revisionsNodes[nextRev] = nextNodes
  found.revisionsZip[nextRev] = blob

  const diff = computeClientDocDiff(docId, curRev, nextRev, curNodes, nextNodes)
  return { detail: { ...found.detail }, diff }
}

export async function downloadDocRevision(
  docId: string,
  rev: number,
  mode: ExportMode,
  fallbackFilename: string,
): Promise<void> {
  const url = `/api/docs/${encodeURIComponent(docId)}/revisions/${rev}/download?mode=${mode}`
  let blob: Blob | null = null
  let downloadName = `${fallbackFilename.replace(/\.docx$/i, '')}_v${rev}_${
    mode === 'redline' ? '修订红线版' : mode === 'clean' ? '净稿版' : '原包'
  }.docx`

  try {
    const res = await fetch(url, { method: 'GET', credentials: 'include' })
    const ct = res.headers.get('content-type') ?? ''
    if (res.ok && !ct.includes('text/html') && !ct.includes('application/json')) {
      blob = await res.blob()
    } else if (!res.ok && ct.includes('application/json')) {
      const err = (await res.json()) as { code?: string; message?: string }
      throw new DocClientError(res.status, err.code ?? 'DownloadError', err.message ?? '导出失败')
    }
  } catch (err) {
    if (err instanceof DocClientError) throw err
  }

  if (!blob) {
    const found = findPreviewDoc(docId)
    if (!found) throw new DocClientError(404, 'NotFound', '文档不存在')
    const nodes = found.revisionsNodes[rev] ?? found.detail.nodes
    const prevNodes = rev > 1 ? found.revisionsNodes[rev - 1] ?? null : null
    const bytes =
      mode === 'raw' && found.revisionsZip[rev]
        ? found.revisionsZip[rev]!
        : buildPreviewDocxBytes(nodes, prevNodes, mode)
    blob = new Blob([bytes as BlobPart], {
      type: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
    })
  }

  const objUrl = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = objUrl
  a.download = downloadName
  document.body.appendChild(a)
  a.click()
  a.remove()
  window.setTimeout(() => URL.revokeObjectURL(objUrl), 1000)
}

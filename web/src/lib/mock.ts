/* 纯静态假数据：只为了把界面撑起来。
 * 接真数据时本文件整体删除，界面组件不改结构。 */

export type Member = {
  name: string
  role: string
}

export type WorkItem = {
  id: string
  title: string
  kind: '标书' | 'Word' | '表格' | '提醒'
  at: string
  active?: boolean
  members: string[]
}

export type WorkGroup = {
  label: string
  items: WorkItem[]
}

/** 空间是企业里的第一级实体：个人空间只是"成员数为 1 的空间"。
 *  界面按 kind 分叉：个人不显示成员、在场、共享。 */
export type Space = {
  id: string
  name: string
  kind: 'personal' | 'team'
  sub: string
}

export const spaces: Space[] = [
  { id: 'me', name: '我的', kind: 'personal', sub: '只有你' },
  { id: 'presales', name: '售前组', kind: 'team', sub: '6 人 · 3 人在场' },
  { id: 'delivery', name: '交付组', kind: 'team', sub: '11 人 · 1 人在场' },
]

/** 左边的分组：办公里第一层是"手上的活"，不是聊天记录 */
export const workspaces: WorkGroup[] = [
  {
    label: '今天',
    items: [
      { id: 'w1', title: '甲方案 · 技术标第三章', kind: '标书', at: '刚刚', active: true, members: ['李渝', '陈默', '王岚'] },
      { id: 'w2', title: '供应商比价表 11 月', kind: '表格', at: '1 小时前', members: ['陈默', '赵隅'] },
      { id: 'w3', title: '周会纪要 · 第 47 周', kind: 'Word', at: '3 小时前', members: ['王岚'] },
    ],
  },
  {
    label: '近七天',
    items: [
      { id: 'w4', title: '投标资格预审清单', kind: '提醒', at: '昨天', members: ['李渝'] },
      { id: 'w5', title: '交付排期（运维）', kind: '表格', at: '周二', members: ['赵隅', '陈默'] },
    ],
  },
]

/** 个人空间：没有成员，条目也没有头像 */
export const personalGroups: WorkGroup[] = [
  {
    label: '今天',
    items: [
      { id: 'p1', title: '周报草稿', kind: 'Word', at: '刚刚', active: true, members: [] },
      { id: 'p2', title: '个人待办 · 12 月', kind: '提醒', at: '2 小时前', members: [] },
    ],
  },
  {
    label: '近七天',
    items: [{ id: 'p3', title: '述职材料大纲', kind: 'Word', at: '周一', members: [] }],
  },
]

export const deliveryGroups: WorkGroup[] = [
  {
    label: '今天',
    items: [
      { id: 'd1', title: '乙项目 · 交付排期', kind: '表格', at: '30 分钟前', active: true, members: ['赵隅', '陈默', '王岚'] },
      { id: 'd2', title: '运维交接清单', kind: '表格', at: '2 小时前', members: ['赵隅'] },
    ],
  },
  {
    label: '近七天',
    items: [{ id: 'd3', title: '乙项目验收报告', kind: 'Word', at: '周四', members: ['王岚', '陈默'] }],
  },
]

export const spaceGroups: Record<string, WorkGroup[]> = {
  me: personalGroups,
  presales: workspaces,
  delivery: deliveryGroups,
}

export function spaceOf(id: string): Space {
  return spaces.find((s) => s.id === id) ?? spaces[0]!
}


/** 当前这份活的参与者，头像是首字 */
export const members: Member[] = [
  { name: '李渝', role: '售前' },
  { name: '陈默', role: '研发' },
  { name: '王岚', role: '运营' },
]

export type Thread = {
  title: string
  kind: '标书' | 'Word' | '表格'
  doc: string
  deadline: string
}

export const thread: Thread = {
  title: '甲方案 · 技术标第三章',
  kind: '标书',
  doc: '技术标_甲方案_v3.docx',
  deadline: '12 月 3 日 17:00 截止',
}

/** 每个空间当前打开的那份活，跟左栏第一条对应 */
export const threads: Record<string, Thread> = {
  me: { title: '周报草稿', kind: 'Word', doc: '周报_第 47 周.docx', deadline: '今天 18:00 提交' },
  presales: thread,
  delivery: { title: '乙项目 · 交付排期', kind: '表格', doc: '乙项目_交付排期_v2.xlsx', deadline: '12 月 5 日 上线' },
}

export function threadOf(id: string) {
  return threads[id] ?? thread
}

export type MsgRow = {
  id: string
  /** you = 自己；mate = 同事；assistant = 阁吏 */
  who: 'you' | 'mate' | 'assistant'
  name: string
  at: string
  text: string
  tool?: { name: string; args: string; state?: 'run' | 'done' }
  diff?: { file: string; add: number; del: number }
}

export const messages: MsgRow[] = [
  {
    id: 'm1',
    who: 'mate',
    name: '李渝',
    at: '09:41',
    text: '运维响应这节甲方模板限 800 字，现在 1,240 字。先砍哪段？',
  },
  {
    id: 'm2',
    who: 'assistant',
    name: '阁吏',
    at: '09:41',
    text: '三段里只有「值班表」是重复信息——第 2 页和第 6 页各写了一遍。把它并成一张表、删掉重复的那段，字数落到 760，其余不动。',
    tool: { name: 'read_doc', args: '技术标_甲方案_v3.docx · 2–6 页', state: 'done' },
  },
  {
    id: 'm3',
    who: 'you',
    name: '陈默',
    at: '09:43',
    text: '合并可以，SLA 那两条别动，那是评分点。',
  },
  {
    id: 'm4',
    who: 'assistant',
    name: '阁吏',
    at: '09:44',
    text: '没动。合并值班表后 762 字，SLA 两条原样保留，锚点也没变，评标索引还能对上。',
    diff: { file: '技术标_甲方案_v3.docx', add: 18, del: 496 },
  },
  {
    id: 'm5',
    who: 'mate',
    name: '王岚',
    at: '09:46',
    text: '改完发我一份，我顺手把封面日期改成 12 月 3 日。',
  },
]

export type CtxLayer = {
  key: string
  label: string
  used: number
  cap: number
}

export const ctxLayers: CtxLayer[] = [
  { key: 'system', label: '系统提示', used: 620, cap: 9000 },
  { key: 'prefix', label: '前缀（稳定区）', used: 5100, cap: 9000 },
  { key: 'tail', label: '尾部增量', used: 1820, cap: 9000 },
]

export const ctxSummary = {
  budget: 9000,
  used: 7540,
  hitRate: 0.78,
  cachedTokens: 39800,
  promptTokens: 51000,
}

export type AdminUser = {
  id: string
  name: string
  grp: string
  state: '启用' | '停用'
  quota: number
  used: number
  createdAt: string
}

export const adminUsers: AdminUser[] = [
  { id: 'u_01H8QK', name: '陈默', grp: '研发', state: '启用', quota: 500, used: 312, createdAt: '2026-09-02' },
  { id: 'u_01H8QN', name: '李渝', grp: '产品', state: '启用', quota: 200, used: 41, createdAt: '2026-09-11' },
  { id: 'u_01H8QT', name: '王岚', grp: '运营', state: '停用', quota: 100, used: 100, createdAt: '2026-10-04' },
  { id: 'u_01H8R2', name: '赵隅', grp: '研发', state: '启用', quota: 500, used: 87, createdAt: '2026-10-22' },
]

/* ── 下面三段是"AI 改文档"的三种呈现方式，用来挑样式，接真数据时整体替换 ── */

export const docChange = {
  file: '技术标_甲方案_v3.docx',
  kind: 'docx' as const,
  add: 18,
  del: 496,
  summary: '合并第 2、6 页重复的值班表',
  folds: [
    { removed: '第 6 页整段「值班表」（与第 2 页重复）', added: '并入表 2-3，第 6 页改为引用「见表 2-3」' },
    { removed: '两处不一致的响应时限表述（4 小时 / 4 个工作小时）', added: '统一为「4 小时内响应」，与 SLA 条款一致' },
  ],
  pairs: [
    {
      before: '运维响应时间为 4 个工作小时，值班表见第 2 页附表与第 6 页附表。',
      after: '运维响应时间为 4 小时，值班表见表 2-3。',
    },
    {
      before: '重大故障处理情况按季度汇总，另附第 6 页值班安排。',
      after: '重大故障处理情况按季度汇总。',
    },
  ],
  sheet: {
    name: '表 2-3 值班表 · 字数统计',
    cells: [
      { cell: 'C12', before: '1,240 字', after: '762 字' },
      { cell: 'C13', before: '4 个工作小时', after: '4 小时' },
      { cell: 'D7', before: '见第 6 页附表', after: '见表 2-3' },
    ],
  },
}

/* ── 文件夹树：跟"最近"是同一批活的两种视图，不是两套数据 ── */

export type FolderNode = {
  id: string
  name: string
  items?: WorkItem[]
  children?: FolderNode[]
}

export const folderTrees: Record<string, FolderNode[]> = {
  me: [
    {
      id: 'f-m1',
      name: '草稿',
      items: [
        { id: 'p1', title: '周报草稿', kind: 'Word', at: '刚刚', active: true, members: [] },
        { id: 'p3', title: '述职材料大纲', kind: 'Word', at: '周一', members: [] },
      ],
    },
    { id: 'f-m2', name: '待办', items: [{ id: 'p2', title: '个人待办 · 12 月', kind: '提醒', at: '2 小时前', members: [] }] },
  ],
  presales: [
    {
      id: 'f-p1',
      name: '投标',
      items: [
        { id: 'w1', title: '甲方案 · 技术标第三章', kind: '标书', at: '刚刚', active: true, members: ['李渝', '陈默', '王岚'] },
        { id: 'w4', title: '投标资格预审清单', kind: '提醒', at: '昨天', members: ['李渝'] },
      ],
      children: [
        { id: 'f-p1a', name: '甲方案', items: [{ id: 'w6', title: '技术标_甲方案_v2.docx', kind: 'Word', at: '周一', members: ['李渝'] }] },
        { id: 'f-p1b', name: '乙方案', items: [] },
      ],
    },
    { id: 'f-p2', name: '供应商', items: [{ id: 'w2', title: '供应商比价表 11 月', kind: '表格', at: '1 小时前', members: ['陈默', '赵隅'] }] },
    { id: 'f-p3', name: '会议纪要', items: [{ id: 'w3', title: '周会纪要 · 第 47 周', kind: 'Word', at: '3 小时前', members: ['王岚'] }] },
  ],
  delivery: [
    {
      id: 'f-d1',
      name: '乙项目',
      items: [
        { id: 'd1', title: '乙项目 · 交付排期', kind: '表格', at: '30 分钟前', active: true, members: ['赵隅', '陈默', '王岚'] },
        { id: 'd3', title: '乙项目验收报告', kind: 'Word', at: '周四', members: ['王岚', '陈默'] },
      ],
    },
    { id: 'f-d2', name: '运维交接', items: [{ id: 'd2', title: '运维交接清单', kind: '表格', at: '2 小时前', members: ['赵隅'] }] },
  ],
}

/* ── 空间设置：成员 / 配额 / 审计 ── */

export type QuotaMember = { name: string; role: string; used: number; cap: number }

export type SpaceQuota = {
  month: string
  tokens: number
  cap: number
  hitRate: number
  members: QuotaMember[]
  audit: { at: string; who: string; what: string }[]
}

export const quotas: Record<string, SpaceQuota> = {
  presales: {
    month: '11 月',
    tokens: 6_840_000,
    cap: 12_000_000,
    hitRate: 0.78,
    members: [
      { name: '李渝', role: '管理员', used: 2_100_000, cap: 4_000_000 },
      { name: '陈默', role: '成员', used: 3_120_000, cap: 4_000_000 },
      { name: '王岚', role: '成员', used: 1_620_000, cap: 4_000_000 },
    ],
    audit: [
      { at: '09:44', who: '阁吏', what: '改 技术标_甲方案_v3.docx · 合并值班表 −496 字' },
      { at: '09:43', who: '陈默', what: '发消息 · 3 人已读' },
      { at: '09:12', who: '李渝', what: '把 甲方案 移入 投标/甲方案' },
    ],
  },
  delivery: {
    month: '11 月',
    tokens: 2_100_000,
    cap: 8_000_000,
    hitRate: 0.72,
    members: [
      { name: '赵隅', role: '管理员', used: 900_000, cap: 3_000_000 },
      { name: '陈默', role: '成员', used: 700_000, cap: 3_000_000 },
      { name: '王岚', role: '成员', used: 500_000, cap: 3_000_000 },
    ],
    audit: [
      { at: '10:02', who: '赵隅', what: '新建 乙项目验收报告' },
      { at: '09:30', who: '阁吏', what: '导出 交付排期.xlsx' },
    ],
  },
  me: {
    month: '11 月',
    tokens: 0,
    cap: 0,
    hitRate: 0,
    members: [{ name: '陈默', role: '仅本人', used: 0, cap: 0 }],
    audit: [],
  },
}


/** 覆盖卡片里的文档正文：state 决定锁/改/正在改的样式 */
export const docParagraphs = [
  {
    id: '3.1',
    state: 'plain' as const,
    text: '我方为本项目配置 7×24 小时值守团队，分三班轮换，每班不少于 4 人。值班表见表 2-3。',
  },
  {
    id: '3.2',
    state: 'locked' as const,
    text: '一般故障 30 分钟内响应、2 小时内恢复；重大故障 15 分钟内响应，2 小时内给出处置方案。',
  },
  {
    id: '3.3',
    state: 'changed' as const,
    text: '所有故障处理过程在运维平台留痕，形成工单，月度汇总后提交甲方审阅。',
  },
  {
    id: '3.4',
    state: 'editing' as const,
    text: '如连续两个季度无重大故障，可下调值守班次为 5×8 小时，并在合同中约定恢复原班次的条件。',
  },
  {
    id: '3.5',
    state: 'plain' as const,
    text: '备件储备：核心交换机、服务器电源模块按 1:1 备件池储备，存放于甲方机房备件柜。',
  },
] as const

/** 根目录列表：与"程序·模板"分开，这里是一个空间的家底 */
export const rootFiles = [
  { id: 'r1', name: '技术标_甲方案_v3.docx', kind: 'doc' as const, size: '1.2 MB', at: '今天 09:41', who: '陈默、王岚' },
  { id: 'r2', name: '交付排期.xlsx', kind: 'sheet' as const, size: '88 KB', at: '昨天 17:20', who: '王岚' },
  { id: 'r3', name: '投标文件', kind: 'folder' as const, size: '—', at: '昨天 11:02', who: '3 人' },
  { id: 'r4', name: '周报_第 47 周.docx', kind: 'doc' as const, size: '24 KB', at: '周一 10:15', who: '陈默' },
  { id: 'r5', name: '脚本/导出目录.py', kind: 'code' as const, size: '3 KB', at: '周一 09:02', who: '你' },
] as const

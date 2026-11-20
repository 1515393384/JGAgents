/** 档位唯一文案源：注册页选择、定价弹窗、登录页价格条三处共读，避免说不一致 */
export type PlanId = 'free' | 'vip' | 'svip' | 'agent'

export type PlanOption = { id: PlanId; title: string; tag: string; points: string[] }

/** 价格唯一定价源 */
export const planPrice: Record<PlanId, { price: string; unit: string; head: string }> = {
  free: { price: '¥0', unit: '/ 永久', head: '2 席以内小团队' },
  vip: { price: '¥49', unit: '/ 席 / 月', head: '全量模型，按席起步' },
  svip: { price: '¥99', unit: '/ 席 / 月', head: 'KEY 只在你手上' },
  agent: { price: '¥199', unit: '/ 席 / 月起', head: '阁吏直接改 Word、Excel、标书' },
}

/** 定价对照表：true = 含，false = 不含，字符串 = 写具体口径。
 *  按能力分组、行名用名词短语（大厂定价表的通行写法），不堆功能清单 */
export type PlanCell = string | boolean
export type PlanRow = { label: string; cells: Record<PlanId, PlanCell> }
export type PlanGroup = { group: string; rows: PlanRow[] }

export const planGroups: PlanGroup[] = [
  {
    group: '办公能力',
    rows: [
      {
        label: '文档改写（Word / Excel / 标书）',
        cells: {
          free: '只读预览',
          vip: '单段改写',
          svip: '单段改写',
          agent: '整篇 · 批量 · 逐段接受',
        },
      },
      {
        label: '定时提醒 / 模板库',
        cells: { free: false, vip: true, svip: true, agent: true },
      },
      {
        label: '通知渠道',
        cells: {
          free: '站内（系统通知）',
          vip: '站内 · 邮件',
          svip: '站内 · 邮件',
          agent: '站内 · 邮件 · 短信（按条计费）',
        },
      },
    ],
  },
  {
    group: '模型与上下文',
    rows: [
      {
        label: '架构优化',
        cells: {
          free: '共享通道',
          vip: '独享通道',
          svip: '独享通道 · 可直连',
          agent: '独享通道 · 办公引擎并行',
        },
      },
      {
        label: '多端续看 / 断线续传',
        cells: { free: true, vip: true, svip: true, agent: true },
      },
      {
        label: '可用模型',
        cells: {
          free: '基础模型集',
          vip: '架阁全量',
          svip: '架阁全量',
          agent: '架阁全量',
        },
      },
      {
        label: '自备模型 API',
        cells: { free: true, vip: true, svip: true, agent: true },
      },
    ],
  },
  {
    group: '空间与容量',
    rows: [
      {
        label: '空间数',
        cells: { free: '1', vip: '3', svip: '10', agent: '不限' },
      },
      {
        label: '席位数',
        cells: { free: '2', vip: '10', svip: '25', agent: '不限' },
      },
      {
        label: '存储',
        cells: { free: '2 GB', vip: '50 GB', svip: '200 GB', agent: '1 TB' },
      },
      {
        label: '网盘挂载',
        cells: { free: false, vip: '1 个', svip: '3 个', agent: '不限' },
      },
    ],
  },
  {
    group: '安全与计费',
    rows: [
      {
        label: '密钥归属',
        cells: {
          free: '我方托管（免输 KEY）',
          vip: '你自己保管',
          svip: '你自己保管',
          agent: '你自己保管',
        },
      },
      {
        label: '加密方式',
        cells: {
          free: '政策约束',
          vip: '密码学保证',
          svip: '密码学保证',
          agent: '密码学保证',
        },
      },
    ],
  },
]

/** 全部档位共同承诺：不是卖点，单独一条横条写，不占对照表的行 */
export const planIncluded: string[] = [
  '不读取内容、不用于模型训练，日志不记录正文',
  '付费档密钥由用户保管，服务端不存储、无法解密',
  '审计仅记录访问元数据：操作人、时间、文件标识',
  '相同开头仅计费一次；加解密不计费；多端续看与断点续传',
  '模型用量按采购成本价直出，0 加价，账单按 token 明细可核',
  '不承诺固定折扣：上游调价同步调整；大额走预付额度包，一次谈定',
]

export const routeOptions: PlanOption[] = [
  {
    id: 'free',
    title: '免费版',
    tag: '免费',
    points: ['1 个空间、2 个席位，架阁基础模型集。', '上下文复用照给：相同开头只计一次费。'],
  },
  {
    id: 'vip',
    title: '服务端处理',
    tag: 'VIP',
    points: [
      '架阁全量模型，不限空间与席位。',
      '我们只做编排、存储与计量，不解密内容。',
      '文档可单段改写；整篇、批量改写选 Office Agent。',
    ],
  },
  {
    id: 'svip',
    title: '云空间加密版',
    tag: 'SVIP',
    points: [
      '随机生成 KEY，务必记住并保存好，我方不存。',
      '架阁全量模型之外，可接自备模型 API。',
      '上传前在你本机加密，我们只存密文；KEY 丢了，这批数据谁也解不开。',
    ],
  },
  {
    id: 'agent',
    title: '办公自动化',
    tag: 'Office Agent',
    points: [
      '阁吏直接进工作台改 Word、Excel、标书，改动标出位置，可以逐段接受。',
      '定时提醒、模板库、协作编辑（按席位）。',
      '含 VIP 全部能力。',
    ],
  },
]

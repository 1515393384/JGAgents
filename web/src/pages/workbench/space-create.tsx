import { useState } from 'react'
import { KeyRound, ShieldCheck, TriangleAlert, Users } from 'lucide-react'
import { MicroWindow } from '../../components/micro-window'
import { routeOptions } from '../../lib/route-copy'
import type { PlanId } from '../../lib/route-copy'

/**
 * 新建空间：默认开加密，并且把代价写在明面上——
 * 加密的是"密钥不出本机"，所以这个空间不能再做服务端前缀复用。
 */
export function SpaceCreate({ open, onClose }: { open: boolean; onClose: () => void }) {
  const [name, setName] = useState('')
  const [team, setTeam] = useState(true)
  const [encrypted, setEncrypted] = useState(true)
  // 加密之后必然要回答：阁吏 到底怎么读到内容（只有服务端处理 / 云空间加密两种）
  const [route, setRoute] = useState<PlanId>('vip')
  const readRoutes = routeOptions.filter((r) => r.id === 'vip' || r.id === 'svip')

  return (
    <MicroWindow
      open={open}
      onClose={onClose}
      icon={<Users size={13} />}
      title="新建空间"
      subtitle="加密默认开启"
      footer={
        <div className="flex h-12 items-center gap-2 border-t border-surface-border px-3">
          <span className="text-[10.5px] text-fg-subtle">
            KEY 随机生成、只显示一次，我们不存；丢了这批数据就解不开了。
          </span>
          <button
            type="button"
            onClick={onClose}
            className="ml-auto h-7 rounded-field px-2.5 text-[11.5px] text-fg-muted hover:bg-bg-subtle hover:text-fg"
          >
            取消
          </button>
          <button
            type="button"
            disabled={!name.trim()}
            className="h-7 rounded-field bg-brand px-3 text-[11.5px] font-medium text-brand-fg transition-opacity hover:bg-brand-hover disabled:opacity-35"
          >
            创建
          </button>
        </div>
      }
    >
      <div className="min-h-0 flex-1 overflow-y-auto px-4 py-4">
        <label className="block text-[11px] text-fg-subtle" htmlFor="space-name">
          空间名称
        </label>
        <input
          id="space-name"
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="例：甲方投标组"
          className="mt-1.5 h-9 w-full rounded-field border border-surface-border bg-bg px-2.5 text-[13px] text-fg outline-none placeholder:text-fg-subtle focus:border-brand/60"
        />

        <div className="mt-3 text-[11px] text-fg-subtle">类型</div>
        <div className="mt-1.5 flex gap-2">
          {[
            { id: 'team', label: '团队空间', on: true },
            { id: 'me', label: '仅自己', on: false },
          ].map((t) => (
            <button
              key={t.id}
              type="button"
              onClick={() => setTeam(t.on)}
              className={
                'flex-1 rounded-field border px-2.5 py-2 text-left text-[12px] transition-colors ' +
                (team === t.on
                  ? 'border-brand/50 bg-brand-soft text-fg'
                  : 'border-surface-border text-fg-muted hover:bg-bg-subtle')
              }
            >
              {t.label}
            </button>
          ))}
        </div>

        {/* 加密：默认打开，且写清"谁都不能恢复" */}
        <div
          className={
            'mt-4 rounded-field border px-3 py-2.5 transition-colors ' +
            (encrypted ? 'border-brand/45 bg-brand-soft/50' : 'border-surface-border bg-bg')
          }
        >
          <div className="flex items-start gap-2">
            <ShieldCheck size={14} className={encrypted ? 'mt-0.5 text-brand' : 'mt-0.5 text-fg-subtle'} />
            <div className="min-w-0">
              <div className="text-[12.5px] font-medium">端到端加密</div>
              <p className="mt-1 text-[11px] leading-5 text-fg-muted">
                创建时随机生成 KEY，只给你看一次，我们不存。内容和文件在离开本机前就已成密文。
              </p>
              <p className="mt-1 flex items-start gap-1 text-[11px] leading-5 text-warn">
                <TriangleAlert size={11} className="mt-0.5 shrink-0" />
                KEY 丢了 = 这批数据谁也解不开，包括我们。连接、计费出问题能帮着查，内容打不开。
              </p>
              <p className="mt-1 text-[10.5px] leading-5 text-fg-subtle">
                老实说：网页版的页面代码由我们分发，理论上前端可被替换，所以加密在浏览器里是承诺而非数学证明；
                要做成可验证的，请用桌面客户端（签名 + 可复现构建）。
              </p>
            </div>
            <button
              type="button"
              onClick={() => setEncrypted((v) => !v)}
              aria-label={encrypted ? '关闭加密' : '开启加密'}
              aria-pressed={encrypted}
              className={
                'ml-auto mt-0.5 h-5 w-9 shrink-0 rounded-pill p-0.5 transition-colors ' +
                (encrypted ? 'bg-brand' : 'bg-bg-accent')
              }
            >
              <span
                className={
                  'block h-4 w-4 rounded-pill bg-white shadow-sm transition-transform ' +
                  (encrypted ? 'translate-x-4' : 'translate-x-0')
                }
              />
            </button>
          </div>
        </div>

        {/* 把代价也说清楚，不藏 */}
        {encrypted ? (
          <p className="mt-2.5 rounded-field bg-bg-subtle/70 px-3 py-2 text-[11px] leading-5 text-fg-subtle">
            加密不吃亏：上下文照样存、开头照样只算一次，只是存的是密文、解开在你机器上。
            要我们替你读得多一点，就关掉加密。
          </p>
        ) : (
          <p className="mt-2.5 rounded-field bg-bg-subtle/70 px-3 py-2 text-[11px] leading-5 text-fg-subtle">
            不加密时可以服务端复用前缀，成本最低；内容和文件由我们静态加密保管，
            管理员技术上可解，任何解密都会留审计记录。
          </p>
        )}

        {/* 加密空间必须回答的问题：模型在云端，明文迟早要出现在某个进程里 */}
        {encrypted ? (
          <div className="mt-4">
            <div className="text-[11px] text-fg-subtle">阁吏 怎么读到内容</div>
            <p className="mt-0.5 text-[10.5px] leading-5 text-fg-subtle">
              这个问题注册时问过一次，这里只是跟着走；想改在这儿也能改。
            </p>
            <div className="mt-1.5 space-y-1.5">
              {readRoutes.map((r) => (
                <button
                  key={r.id}
                  type="button"
                  onClick={() => setRoute(r.id)}
                  className={
                    'flex w-full items-start gap-2 rounded-field border px-2.5 py-2 text-left transition-colors ' +
                    (route === r.id
                      ? 'border-brand/50 bg-brand-soft'
                      : 'border-surface-border hover:bg-bg-subtle')
                  }
                >
                  <span
                    className={
                      'mt-0.5 h-3 w-3 shrink-0 rounded-pill border ' +
                      (route === r.id ? 'border-brand bg-brand' : 'border-fg-subtle/60')
                    }
                  />
                  <span className="min-w-0">
                    <span className="flex items-center gap-1.5 text-[12px] text-fg">
                      {r.title}
                      <span className="rounded-pill bg-bg-subtle px-1.5 text-[10px] text-fg-subtle">
                        {r.tag}
                      </span>
                    </span>
                    <span className="mt-1 block space-y-0.5 text-[10.5px] leading-[1.6] text-fg-muted">
                      {r.points.map((line, i) => (
                        <span key={i} className="flex gap-1">
                          <span className="shrink-0 text-fg-subtle">{i + 1}.</span>
                          <span>{line}</span>
                        </span>
                      ))}
                    </span>
                  </span>
                </button>
              ))}
            </div>
          </div>
        ) : null}

        <div className="mt-4 flex items-start gap-1.5 text-[10.5px] leading-5 text-fg-subtle">
          <KeyRound size={11} className="mt-0.5 shrink-0" />
          密码本身只存 Argon2id 哈希（每用户随机 salt + 服务端 pepper），我们自己也无法反推；
          登录态是一枚随机令牌，库里只留它的哈希。密码最少 12 位，常见弱口令直接拦下。
        </div>
      </div>
    </MicroWindow>
  )
}

import { Button, Stat } from '../../components/ui'
import { adminUsers } from '../../lib/mock'

const tabs = ['用户', '连接', '计量', '存档', '健康']

export function AdminPage() {
  return (
    <div className="h-full overflow-y-auto">
      <div className="mx-auto max-w-[1400px] px-4 py-5 md:px-6 md:py-6">
        <div className="flex flex-wrap items-center gap-2">
          <h1 className="text-lg font-medium tracking-tight">中控台</h1>
          <div className="ml-auto">
            <Button variant="outline">新建用户</Button>
          </div>
        </div>

        {/* 窄屏横向滚动，不换行不挤压 */}
        <div className="-mx-4 mt-4 overflow-x-auto px-4 md:mx-0 md:px-0">
          <div className="flex items-center gap-1 border-b border-border">
            {tabs.map((t, i) => (
              <span
                key={t}
                className={
                  i === 0
                    ? '-mb-px shrink-0 border-b-2 border-brand px-3 py-2 text-sm text-fg'
                    : '-mb-px shrink-0 border-b-2 border-transparent px-3 py-2 text-sm text-fg-muted'
                }
              >
                {t}
              </span>
            ))}
          </div>
        </div>

        <div className="mt-5 grid grid-cols-2 gap-2.5 md:gap-3 lg:grid-cols-4">
          <Stat label="用户" value="12" hint="启用 10 · 停用 2" />
          <Stat label="活跃连接" value="3" hint="上限 5k" />
          <Stat label="今日请求" value="1,284" hint="失败 6" />
          <Stat label="前缀命中率" value="78%" hint="近 24 小时" />
        </div>

        <div className="mt-5 overflow-x-auto rounded-card border border-border md:mt-6">
          <table className="w-full min-w-[520px] text-left text-sm">
            <thead className="bg-bg-subtle text-xs text-fg-muted">
              <tr>
                <th className="px-3 py-2 font-normal md:px-4">ID</th>
                <th className="px-3 py-2 font-normal md:px-4">名称</th>
                <th className="px-3 py-2 font-normal md:px-4">分组</th>
                <th className="px-3 py-2 font-normal md:px-4">状态</th>
                <th className="px-3 py-2 font-normal md:px-4">配额</th>
                <th className="hidden px-4 py-2 font-normal md:table-cell">创建</th>
              </tr>
            </thead>
            <tbody>
              {adminUsers.map((u) => (
                <tr key={u.id} className="border-t border-border">
                  <td className="px-3 py-2 font-mono text-xs text-fg-subtle md:px-4">{u.id}</td>
                  <td className="whitespace-nowrap px-3 py-2 md:px-4">{u.name}</td>
                  <td className="px-3 py-2 text-fg-muted md:px-4">{u.grp}</td>
                  <td
                    className={
                      u.state === '启用'
                        ? 'whitespace-nowrap px-3 py-2 text-ok md:px-4'
                        : 'whitespace-nowrap px-3 py-2 text-fg-subtle md:px-4'
                    }
                  >
                    {u.state}
                  </td>
                  <td className="whitespace-nowrap px-3 py-2 font-mono text-xs text-fg-muted md:px-4">
                    {u.used} / {u.quota}
                  </td>
                  <td className="hidden whitespace-nowrap px-4 py-2 text-xs text-fg-subtle md:table-cell">
                    {u.createdAt}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  )
}

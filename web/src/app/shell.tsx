import { Outlet } from '@tanstack/react-router'

/** 只有底色和文字色，不套顶栏。
 * 企业工作台是"整屏应用"：导航在左栏里，顶栏会把正文压短一截、还会跟会话头打架。 */
export function AppShell() {
  return (
    <div className="h-full bg-bg text-fg">
      <Outlet />
    </div>
  )
}

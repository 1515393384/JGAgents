import { Outlet, createRootRoute, createRoute, createRouter } from '@tanstack/react-router'
import { AppShell } from './shell'
import { WorkbenchPage } from '../pages/workbench'
import { AdminPage } from '../pages/admin'
import { LoginPage } from '../pages/login'

const rootRoute = createRootRoute({ component: Outlet })

/** 带底色的布局；登录页不在它下面。 */
const appLayout = createRoute({
  getParentRoute: () => rootRoute,
  id: 'app',
  component: AppShell,
})

/** 空间是地址的一部分：/s/presales 能直接发给同事。
 *  / 落地到个人空间，和 /s/me 是同一屏。 */
const rootSpaceRoute = createRoute({
  getParentRoute: () => appLayout,
  path: '/',
  component: WorkbenchPage,
})

const spaceRoute = createRoute({
  getParentRoute: () => appLayout,
  path: '/s/$spaceId',
  component: WorkbenchPage,
})

const adminRoute = createRoute({
  getParentRoute: () => appLayout,
  path: '/admin',
  component: AdminPage,
})

const loginRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/login',
  component: LoginPage,
})

const routeTree = rootRoute.addChildren([
  appLayout.addChildren([rootSpaceRoute, spaceRoute, adminRoute]),
  loginRoute,
])

export const router = createRouter({ routeTree })

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router
  }
}

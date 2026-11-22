export type AuthUser = {
  id: string
  name: string
  group: string
  role: 'admin' | 'member'
  state: 'active' | 'disabled'
  dailyRequests: number
  monthlyTokens: number
  usedRequests: number
  usedTokens: number
  createdAtMs: number
  lastActiveMs: number
}

export class AuthClientError extends Error {
  status: number
  constructor(status: number, message: string) {
    super(message)
    this.name = 'AuthClientError'
    this.status = status
  }
}

const PREVIEW_USER_KEY = 'jiagou.preview.user.v1'
const PREVIEW_ACCOUNTS_KEY = 'jiagou.preview.accounts.v1'

type PreviewProfile = AuthUser

function readPreviewUser(): AuthUser | null {
  try {
    const raw = sessionStorage.getItem(PREVIEW_USER_KEY) || localStorage.getItem(PREVIEW_USER_KEY)
    if (!raw) return null
    const parsed = JSON.parse(raw) as unknown
    if (!parsed || typeof parsed !== 'object' || typeof (parsed as AuthUser).name !== 'string') {
      return null
    }
    return parsed as AuthUser
  } catch {
    return null
  }
}

function writePreviewUser(user: AuthUser | null, remember: boolean): void {
  try {
    sessionStorage.removeItem(PREVIEW_USER_KEY)
    localStorage.removeItem(PREVIEW_USER_KEY)
    if (!user) return
    const serialized = JSON.stringify(user)
    if (remember) {
      localStorage.setItem(PREVIEW_USER_KEY, serialized)
    } else {
      sessionStorage.setItem(PREVIEW_USER_KEY, serialized)
    }
  } catch {
  }
}

function readPreviewAccounts(): PreviewProfile[] {
  try {
    const raw = localStorage.getItem(PREVIEW_ACCOUNTS_KEY)
    if (!raw) return []
    const parsed = JSON.parse(raw) as unknown
    return Array.isArray(parsed) ? (parsed as PreviewProfile[]) : []
  } catch {
    return []
  }
}

function writePreviewAccounts(accounts: PreviewProfile[]): void {
  try {
    localStorage.setItem(PREVIEW_ACCOUNTS_KEY, JSON.stringify(accounts))
  } catch {
  }
}

function makePreviewUser(name: string): AuthUser {
  const now = Date.now()
  return {
    id: `u_preview_${now.toString(36)}`,
    name,
    group: '个人',
    role: 'member',
    state: 'active',
    dailyRequests: 500,
    monthlyTokens: 2_000_000,
    usedRequests: 0,
    usedTokens: 0,
    createdAtMs: now,
    lastActiveMs: now,
  }
}

async function tryAuthApi<T>(path: string, init?: RequestInit): Promise<T | null> {
  let res: Response
  try {
    res = await fetch(path, {
      ...init,
      credentials: 'include',
      headers: {
        'Content-Type': 'application/json',
        ...(init?.headers as Record<string, string> | undefined),
      },
    })
  } catch {
    return null
  }

  const contentType = res.headers.get('content-type') ?? ''
  if (!contentType.includes('application/json')) {
    return null
  }

  const payload = (await res.json()) as T & { message?: string }
  if (!res.ok) {
    const message = payload.message || `认证请求失败 (${res.status})`
    throw new AuthClientError(res.status, message)
  }

  return payload
}

export async function fetchCurrentUser(): Promise<AuthUser | null> {
  try {
    const remote = await tryAuthApi<{ user: AuthUser }>('/api/auth/me', {
      method: 'GET',
    })
    if (remote?.user) {
      writePreviewUser(null, false)
      return remote.user
    }
    return readPreviewUser()
  } catch (err) {
    if (err instanceof AuthClientError && (err.status === 401 || err.status === 403)) {
      writePreviewUser(null, false)
      return null
    }
    throw err
  }
}

export async function loginWithPassword(
  username: string,
  password: string,
  remember: boolean,
): Promise<AuthUser> {
  const name = username.trim()
  if (!name) throw new AuthClientError(400, '请输入用户名')
  if (!password) throw new AuthClientError(400, '请输入密码')

  const remote = await tryAuthApi<{ user: AuthUser }>('/api/auth/login', {
    method: 'POST',
    body: JSON.stringify({ username: name, password, remember }),
  })
  if (remote?.user) {
    writePreviewUser(null, false)
    return remote.user
  }

  const accounts = readPreviewAccounts()
  const existing = accounts.find((a) => a.name === name)
  const user: AuthUser = existing
    ? { ...existing, lastActiveMs: Date.now() }
    : makePreviewUser(name)
  writePreviewUser(user, remember)
  return user
}

export async function registerWithPassword(
  username: string,
  password: string,
  remember = true,
): Promise<AuthUser> {
  const name = username.trim()
  if (!name) throw new AuthClientError(400, '请输入用户名')
  if (password.length < 12) throw new AuthClientError(400, '密码至少需要 12 位')

  const remote = await tryAuthApi<{ user: AuthUser }>('/api/auth/register', {
    method: 'POST',
    body: JSON.stringify({ username: name, password, remember }),
  })
  if (remote?.user) {
    writePreviewUser(null, false)
    return remote.user
  }

  const accounts = readPreviewAccounts()
  if (accounts.some((a) => a.name === name)) {
    throw new AuthClientError(409, '该用户名已被注册')
  }
  const user = makePreviewUser(name)
  writePreviewAccounts([user, ...accounts])
  writePreviewUser(user, remember)
  return user
}

export async function logoutSession(): Promise<void> {
  writePreviewUser(null, false)
  try {
    await fetch('/api/auth/logout', {
      method: 'POST',
      credentials: 'include',
    })
  } catch {
  }
}

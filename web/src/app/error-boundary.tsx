import { Component, type ErrorInfo, type ReactNode } from 'react'

type Props = { children: ReactNode }
type State = { error: Error | null }

/**
 * 兜底：任何渲染异常都显示成一屏可读的错误，而不是白屏。
 * 白屏是最糟的失败方式——它让"代码错了"和"网断了"看起来一模一样。
 */
export class ErrorBoundary extends Component<Props, State> {
  state: State = { error: null }

  static getDerivedStateFromError(error: Error): State {
    return { error }
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error('[render]', error, info.componentStack)
  }

  render() {
    const { error } = this.state
    if (!error) return this.props.children

    return (
      <div className="flex h-full items-center justify-center bg-bg p-6 text-fg">
        <div className="w-full max-w-lg rounded-card border border-border bg-bg-elevated p-6">
          <div className="text-sm font-medium">界面出错了</div>
          <p className="mt-1 text-xs text-fg-muted">
            这不是网络问题，是渲染异常。刷新通常能恢复；如果反复出现，把下面这段发给我们。
          </p>
          <pre className="mt-4 max-h-56 overflow-auto rounded-field border border-border bg-bg p-3 font-mono text-[11px] leading-relaxed text-danger">
            {error.message}
            {error.stack ? `\n\n${error.stack}` : ''}
          </pre>
          <button
            type="button"
            onClick={() => window.location.reload()}
            className="mt-4 inline-flex h-9 items-center rounded-field bg-brand px-3 text-sm text-brand-fg"
          >
            重新加载
          </button>
        </div>
      </div>
    )
  }
}

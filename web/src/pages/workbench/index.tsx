import { useState } from 'react'
import { useParams } from '@tanstack/react-router'
import { Composer } from './composer'
import { ConversationList } from './conversation-list'
import { Dock } from './dock'
import { DocCard } from './doc-card'
import { MessageStream } from './message-stream'
import { SpaceCreate } from './space-create'
import { SpaceRoot } from './space-root'
import { TemplateWindow } from './template-window'
import { spaceOf } from '../../lib/mock'

/** 同一时刻只开一个微应用窗口，壳体统一，内容各归各的 */
type Win =
  | { kind: 'doc' }
  | { kind: 'root' }
  | { kind: 'space' }
  | { kind: 'tpl'; label: string }
  | null

export function WorkbenchPage() {
  const [navOpen, setNavOpen] = useState(false)
  const [win, setWin] = useState<Win>({ kind: 'doc' })
  // / 落在个人空间；/s/:spaceId 才是显式空间
  const params = useParams({ strict: false }) as { spaceId?: string }
  const space = spaceOf(params.spaceId ?? 'me')
  const close = () => setWin(null)

  return (
    <div className="flex h-full min-h-0 bg-bg">
      <ConversationList space={space} open={navOpen} onClose={() => setNavOpen(false)} />
      <section className="flex min-w-0 flex-1 flex-col">
        <MessageStream space={space} onOpenNav={() => setNavOpen(true)} />
        <Composer />
      </section>

      <Dock
        onOpenDoc={() => setWin({ kind: 'doc' })}
        onOpenRoot={() => setWin({ kind: 'root' })}
        onOpenSpace={() => setWin({ kind: 'space' })}
        onOpenTemplate={(label) => setWin({ kind: 'tpl', label })}
      />

      <DocCard open={win?.kind === 'doc'} onClose={close} />
      <SpaceRoot open={win?.kind === 'root'} onClose={close} />
      <SpaceCreate open={win?.kind === 'space'} onClose={close} />
      <TemplateWindow
        open={win?.kind === 'tpl'}
        label={win?.kind === 'tpl' ? win.label : ''}
        onClose={close}
      />
    </div>
  )
}

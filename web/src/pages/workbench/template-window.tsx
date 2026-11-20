import { FilePlus2 } from 'lucide-react'
import { MicroWindow } from '../../components/micro-window'

/** 从 dock 上的模板点开：复用同一套微应用窗口，内容先是空白 */
export function TemplateWindow({
  open,
  label,
  onClose,
}: {
  open: boolean
  label: string
  onClose: () => void
}) {
  return (
    <MicroWindow
      open={open}
      onClose={onClose}
      icon={<FilePlus2 size={13} />}
      title={`${label} · 未命名`}
      subtitle="新建"
      status={
        <span className="hidden shrink-0 rounded-pill bg-bg-subtle px-1.5 py-0.5 text-[10.5px] text-fg-subtle sm:inline">
          尚未保存
        </span>
      }
    >
      <div className="min-h-0 flex-1 overflow-y-auto bg-surface-sunken px-3 py-4 md:px-6">
        <div className="mx-auto flex max-w-[640px] flex-col items-center justify-center rounded-field bg-surface py-20 text-center shadow-card">
          <p className="text-[13.5px] text-fg-muted">一张空白{label}，落在这个空间里</p>
          <p className="mt-2 max-w-[22rem] text-[11.5px] leading-5 text-fg-subtle">
            直接开始写，或者回左边跟阁吏说一句要什么——
            它会先把骨架搭出来，改动照样记版本，前缀也只生成一次。
          </p>
        </div>
      </div>
    </MicroWindow>
  )
}

import { useEffect, useState } from 'react'

export type ThemeMode = 'light' | 'dark'

export type Preset = {
  id: string
  label: string
  mode: ThemeMode
  /** 只用来画小色块：[页面, 卡片, 强调] */
  swatch: [string, string, string]
}

/** 主题只写 id，颜色全在 tokens.css 里，一处维护。
 * 换主题 = html[class=dark] + html[data-theme=id]，组件里没有任何颜色判断。 */
export const presets: Preset[] = [
  { id: 'paper', label: '纸白', mode: 'light', swatch: ['#efeeec', '#fbfaf8', '#3ea4ec'] },
  { id: 'mist', label: '雾灰', mode: 'light', swatch: ['#e9eaec', '#fafbfc', '#2f80ed'] },
  { id: 'night', label: '暗夜', mode: 'dark', swatch: ['#1f1f1f', '#262626', '#4daafc'] },
  { id: 'warm', label: '炭灰', mode: 'dark', swatch: ['#242322', '#302e2c', '#6cb6ff'] },
  { id: 'onedark', label: '深蓝夜', mode: 'dark', swatch: ['#282c34', '#2f343d', '#61afef'] },
]

const K_PRESET = 'jiagou.preset'

function applyPreset(id: string) {
  const el = document.documentElement
  const p = presets.find((x) => x.id === id) ?? presets[0]!
  el.classList.toggle('dark', p.mode === 'dark')
  el.dataset.theme = p.id
  // 手机浏览器地址栏跟着走
  const bg = getComputedStyle(el).getPropertyValue('--bg').trim()
  document
    .querySelector('meta[name="theme-color"]')
    ?.setAttribute('content', bg || (p.mode === 'dark' ? '#1f1f1f' : '#efeeec'))
}

function initial(): string {
  const saved = localStorage.getItem(K_PRESET)
  if (saved && presets.some((p) => p.id === saved)) return saved
  return document.documentElement.classList.contains('dark') ? 'night' : 'paper'
}

export function useTheme() {
  const [preset, setPreset] = useState(initial)

  useEffect(() => {
    applyPreset(preset)
    localStorage.setItem(K_PRESET, preset)
  }, [preset])

  return {
    preset,
    /** 当前是亮还是暗（给 canvas 之类的画笔用） */
    theme: (document.documentElement.classList.contains('dark') ? 'dark' : 'light') as ThemeMode,
    setPreset,
    /** 亮暗一键反转：跳到另一档的代表主题 */
    toggle: () => setPreset((cur) => {
      const m = presets.find((p) => p.id === cur)?.mode ?? 'light'
      return m === 'dark' ? 'paper' : 'night'
    }),
  }
}

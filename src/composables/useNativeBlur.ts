import { invoke } from '@tauri-apps/api/core'
import { useMutationObserver, useResizeObserver, useEventListener } from '@vueuse/core'
import { onMounted, onScopeDispose, type Ref } from 'vue'

interface BlurRegion {
  x: number
  y: number
  width: number
  height: number
  radius: number
}

const SURFACES = "[class*='bg-rune-surface'], [class*='bg-rune-raised']"

function regionsIn(root: HTMLElement): BlurRegion[] {
  const found = [...root.querySelectorAll<HTMLElement>(SURFACES)].filter(
    (element) => !element.parentElement?.closest(SURFACES),
  )
  return found.flatMap((element) => {
    const box = element.getBoundingClientRect()
    if (box.width < 1 || box.height < 1) return []
    const style = getComputedStyle(element)
    if (style.visibility === 'hidden' || style.display === 'none') return []
    const radius = Math.min(
      parseFloat(style.borderTopLeftRadius) || 0,
      box.width / 2,
      box.height / 2,
    )
    return [{ x: box.left, y: box.top, width: box.width, height: box.height, radius }]
  })
}

function windowRegion(root: HTMLElement): BlurRegion[] {
  const box = root.getBoundingClientRect()
  const radius = parseFloat(getComputedStyle(root).borderTopLeftRadius) || 0
  return [{ x: box.left, y: box.top, width: box.width, height: box.height, radius }]
}

export function useNativeBlur(root: Ref<HTMLElement | null | undefined>, whole = false) {
  let frame = 0
  let last = ''
  const html = document.documentElement

  async function apply() {
    frame = 0
    if (!root.value) return
    const regions = whole ? windowRegion(root.value) : regionsIn(root.value)
    const key = JSON.stringify(regions)
    if (key === last) return
    last = key
    try {
      const active = await invoke<boolean>('set_blur_regions', { regions, clip: !whole })
      html.classList.toggle('native-blur', active)
    } catch {
      html.classList.remove('native-blur')
    }
  }
  const schedule = () => {
    if (!frame) frame = requestAnimationFrame(() => void apply())
  }

  onMounted(schedule)
  useMutationObserver(root, schedule, { subtree: true, childList: true, attributes: true })
  useResizeObserver(root, schedule)
  useEventListener(window, 'resize', schedule)
  onScopeDispose(() => {
    if (frame) cancelAnimationFrame(frame)
    html.classList.remove('native-blur')
  })
}
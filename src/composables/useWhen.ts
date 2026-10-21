import { useDateFormat, useTimeAgo } from '@vueuse/core'
import { computed, toValue, type MaybeRefOrGetter } from 'vue'

const DAY_MS = 86_400_000

export function useWhen(iso: MaybeRefOrGetter<string | null | undefined>) {
  const date = computed(() => {
    const raw = toValue(iso)
    const at = raw ? new Date(raw) : null
    return at && !Number.isNaN(at.getTime()) ? at : null
  })
  const ago = useTimeAgo(() => date.value ?? 0)
  const clock = useDateFormat(() => date.value ?? 0, 'h:mm A')
  const day = useDateFormat(() => date.value ?? 0, 'ddd D MMM')
  const full = useDateFormat(() => date.value ?? 0, 'dddd D MMMM YYYY, h:mm:ss A')

  const label = computed(() => {
    const at = date.value
    if (!at) return ''
    const startOfToday = new Date().setHours(0, 0, 0, 0)
    if (at.getTime() >= startOfToday) return `Today, ${clock.value}`
    if (at.getTime() >= startOfToday - DAY_MS) return `Yesterday, ${clock.value}`
    return `${day.value}, ${clock.value}`
  })

  return {
    label,
    ago,
    full: computed(() => (date.value ? full.value : '')),
    valid: computed(() => date.value !== null),
  }
}
import { useDateFormat, useTimeAgo, useNow, useIntervalFn } from '@vueuse/core'
import { computed, toValue, type MaybeRefOrGetter } from 'vue'

export function useWhen(iso: MaybeRefOrGetter<string | null | undefined>) {
  const now = useNow({ scheduler: (update) => useIntervalFn(update, 60_000) })
  const date = computed(() => {
    const raw = toValue(iso)
    const at = raw ? new Date(raw) : null
    return at && !Number.isNaN(at.getTime()) ? at : null
  })
  const ago = useTimeAgo(() => date.value ?? 0)
  const clock = useDateFormat(() => date.value ?? 0, 'h:mm A')
  const day = useDateFormat(() => date.value ?? 0, 'ddd D MMM')
  const dated = useDateFormat(() => date.value ?? 0, 'D MMM YYYY')
  const full = useDateFormat(() => date.value ?? 0, 'dddd D MMMM YYYY, h:mm:ss A')

  const label = computed(() => {
    const at = date.value
    if (!at) return ''
    const today = new Date(now.value)
    today.setHours(0, 0, 0, 0)
    const tomorrow = new Date(today)
    tomorrow.setDate(tomorrow.getDate() + 1)
    const afterTomorrow = new Date(tomorrow)
    afterTomorrow.setDate(afterTomorrow.getDate() + 1)
    const yesterday = new Date(today)
    yesterday.setDate(yesterday.getDate() - 1)
    if (at >= today && at < tomorrow) return `Today, ${clock.value}`
    if (at >= tomorrow && at < afterTomorrow) return `Tomorrow, ${clock.value}`
    if (at >= yesterday && at < today) return `Yesterday, ${clock.value}`
    return `${at.getFullYear() === today.getFullYear() ? day.value : dated.value}, ${clock.value}`
  })

  return {
    label,
    ago,
    full: computed(() => (date.value ? full.value : '')),
    valid: computed(() => date.value !== null),
  }
}
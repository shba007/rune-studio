import { useLocalStorage } from '@vueuse/core'

export const SNOOZE_CHOICES = [
  { id: '10', label: '10 min', minutes: 10 },
  { id: '60', label: '1 hour', minutes: 60 },
  { id: 'tomorrow', label: 'Tomorrow', minutes: 0 },
] as const

export function tomorrowMorningMinutes(now = new Date()) {
  const at = new Date(now)
  at.setDate(at.getDate() + 1)
  at.setHours(9, 0, 0, 0)
  return Math.max(1, Math.ceil((at.getTime() - now.getTime()) / 60000))
}

export const snoozeMinutes = (id: string, now = new Date()) =>
  id === 'tomorrow'
    ? tomorrowMorningMinutes(now)
    : (SNOOZE_CHOICES.find((choice) => choice.id === id)?.minutes ?? 10)

export function useReminderSound() {
  return useLocalStorage('rune.reminderSound', true)
}

export function playReminderSound() {
  try {
    const Context =
      window.AudioContext ??
      (window as unknown as { webkitAudioContext?: typeof AudioContext }).webkitAudioContext
    if (!Context) return
    const context = new Context()
    const start = context.currentTime
    for (const [index, frequency] of [880, 1175].entries()) {
      const oscillator = context.createOscillator()
      const gain = context.createGain()
      oscillator.frequency.value = frequency
      gain.gain.setValueAtTime(0.0001, start + index * 0.22)
      gain.gain.exponentialRampToValueAtTime(0.18, start + index * 0.22 + 0.03)
      gain.gain.exponentialRampToValueAtTime(0.0001, start + index * 0.22 + 0.2)
      oscillator.connect(gain).connect(context.destination)
      oscillator.start(start + index * 0.22)
      oscillator.stop(start + index * 0.22 + 0.22)
    }
    window.setTimeout(() => void context.close(), 700)
  } catch {
    return
  }
}
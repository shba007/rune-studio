import { useLocalStorage } from '@vueuse/core'

export function useDomainPrefs() {
  const showSanctuary = useLocalStorage('rune.showSanctuary', false)
  const overrides = useLocalStorage<Record<string, string>>('rune.goalDomains', {})
  return { showSanctuary, overrides }
}
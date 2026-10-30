import { useLocalStorage } from '@vueuse/core'

export function useDomainPrefs() {
  const showSanctuary = useLocalStorage('rune.showSanctuary', false)
  const overrides = useLocalStorage<Record<string, string>>('rune.goalDomains', {})
  const subDomains = useLocalStorage<Record<string, string[]>>('rune.goalSubDomains', {})
  const goalMindsets = useLocalStorage<Record<string, string>>('rune.goalMindsets', {})
  return { showSanctuary, overrides, subDomains, goalMindsets }
}
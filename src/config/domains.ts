import registry from '../../src-tauri/src/goal_domains.json'
import type { IntakeEntry } from '@/types/decisions'

export type DomainZone = 'engine' | 'sanctuary'

export interface SubDomain {
  id: string
  number: string
  name: string
}

export interface Domain {
  id: string
  number: number
  name: string
  zone: DomainZone
  subDomains: SubDomain[]
  keywords: string[]
}

export const DOMAINS: Domain[] = registry as Domain[]

export const UNASSIGNED: Domain = {
  id: 'unassigned',
  number: 0,
  name: 'Unassigned',
  zone: 'engine',
  subDomains: [],
  keywords: [],
}

const CLASS_DOMAIN: Record<IntakeEntry['class'], string | null> = {
  health: 'physical',
  mind: 'emotional',
  finance: 'financial',
  general: null,
}

export const domainById = (id: string): Domain =>
  DOMAINS.find((domain) => domain.id === id) ?? UNASSIGNED

export function suggestDomain(text: string): Domain | null {
  const haystack = text.toLowerCase()
  return DOMAINS.find((domain) => domain.keywords.some((word) => haystack.includes(word))) ?? null
}

export function domainOf(entry: IntakeEntry, overrides: Record<string, string> = {}): Domain {
  if (entry.domain != null) return domainById(entry.domain)
  const chosen = overrides[entry.id]
  if (chosen) return domainById(chosen)
  const goal = entry.approvedGoal ?? entry.goal
  const hit = suggestDomain(`${entry.category ?? ''} ${goal?.outcome ?? ''} ${entry.text}`)
  const byClass = CLASS_DOMAIN[entry.class]
  if (byClass && (!hit || hit.id === 'career')) return domainById(byClass)
  return hit ?? UNASSIGNED
}
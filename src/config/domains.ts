import type { IntakeEntry } from '@/types/decisions'

export type DomainZone = 'engine' | 'sanctuary'

export interface Domain {
  id: string
  number: number
  name: string
  zone: DomainZone
  keywords: string[]
}

export const DOMAINS: Domain[] = [
  {
    id: 'physical',
    number: 1,
    name: 'Physical Vitality',
    zone: 'engine',
    keywords: [
      'health',
      'fit',
      'sleep',
      'exercise',
      'run',
      'workout',
      'nutrition',
      'diet',
      'weight',
      'strength',
      'cardio',
      'medical',
      'body',
    ],
  },
  {
    id: 'emotional',
    number: 2,
    name: 'Emotional Regulation',
    zone: 'engine',
    keywords: [
      'mind',
      'stress',
      'anxiety',
      'emotion',
      'mood',
      'mental',
      'calm',
      'therapy',
      'meditat',
      'resilien',
    ],
  },
  {
    id: 'cognitive',
    number: 3,
    name: 'Cognitive Capital',
    zone: 'engine',
    keywords: [
      'learn',
      'study',
      'read',
      'focus',
      'memory',
      'knowledge',
      'skill',
      'language',
      'course',
      'think',
    ],
  },
  {
    id: 'career',
    number: 4,
    name: 'Career & Impact',
    zone: 'engine',
    keywords: [
      'career',
      'job',
      'work',
      'business',
      'promotion',
      'startup',
      'client',
      'professional',
      'project',
    ],
  },
  {
    id: 'financial',
    number: 5,
    name: 'Financial Architecture',
    zone: 'engine',
    keywords: [
      'finance',
      'money',
      'save',
      'saving',
      'budget',
      'invest',
      'debt',
      'income',
      'wealth',
      'retire',
    ],
  },
  {
    id: 'environment',
    number: 9,
    name: 'Environment & Friction',
    zone: 'engine',
    keywords: [
      'home',
      'declutter',
      'organi',
      'workspace',
      'environment',
      'tidy',
      'clean',
      'automation',
      'routine',
    ],
  },
  {
    id: 'romantic',
    number: 6,
    name: 'Romantic Partnership',
    zone: 'sanctuary',
    keywords: ['partner', 'romantic', 'relationship', 'spouse', 'dating', 'marriage', 'intimacy'],
  },
  {
    id: 'social',
    number: 7,
    name: 'Social & Tribe',
    zone: 'sanctuary',
    keywords: ['friend', 'social', 'community', 'network', 'tribe'],
  },
  {
    id: 'family',
    number: 8,
    name: 'Family & Kinship',
    zone: 'sanctuary',
    keywords: ['family', 'parent', 'child', 'kid', 'sibling', 'kinship'],
  },
  {
    id: 'play',
    number: 10,
    name: 'Pure Play & Recovery',
    zone: 'sanctuary',
    keywords: ['play', 'hobby', 'travel', 'art', 'music', 'game', 'fun', 'vacation', 'leisure'],
  },
  {
    id: 'purpose',
    number: 11,
    name: 'Character & Purpose',
    zone: 'sanctuary',
    keywords: ['purpose', 'meaning', 'value', 'character', 'spiritual', 'faith', 'mission'],
  },
  {
    id: 'legacy',
    number: 12,
    name: 'Contribution & Legacy',
    zone: 'sanctuary',
    keywords: ['legacy', 'contribut', 'volunteer', 'charity', 'give back', 'mentor'],
  },
]

export const UNASSIGNED: Domain = {
  id: 'unassigned',
  number: 0,
  name: 'Unassigned',
  zone: 'engine',
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

export function domainOf(entry: IntakeEntry, overrides: Record<string, string> = {}): Domain {
  const chosen = overrides[entry.id]
  if (chosen) return domainById(chosen)
  const goal = entry.approvedGoal ?? entry.goal
  const haystack = `${entry.category ?? ''} ${goal?.outcome ?? ''} ${entry.text}`.toLowerCase()
  const byClass = CLASS_DOMAIN[entry.class]
  const hit = DOMAINS.find((domain) => domain.keywords.some((word) => haystack.includes(word)))
  if (byClass && (!hit || hit.id === 'career')) return domainById(byClass)
  return hit ?? UNASSIGNED
}
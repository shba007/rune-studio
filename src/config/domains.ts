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

export const DOMAINS: Domain[] = [
  {
    id: 'physical',
    number: 1,
    name: 'Physical Vitality',
    zone: 'engine',
    subDomains: [
      {
        id: 'sleep-and-circadian-architecture',
        number: '1.1',
        name: 'Sleep & Circadian Architecture',
      },
      {
        id: 'metabolic-and-nutritional-biochemistry',
        number: '1.2',
        name: 'Metabolic & Nutritional Biochemistry',
      },
      { id: 'cardiorespiratory-capacity', number: '1.3', name: 'Cardiorespiratory Capacity' },
      {
        id: 'musculoskeletal-strength-and-hypertrophy',
        number: '1.4',
        name: 'Musculoskeletal Strength & Hypertrophy',
      },
      {
        id: 'mobility-stability-and-biomechanics',
        number: '1.5',
        name: 'Mobility, Stability & Biomechanics',
      },
      {
        id: 'preventive-medicine-and-toxin-avoidance',
        number: '1.6',
        name: 'Preventive Medicine & Toxin Avoidance',
      },
    ],
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
    name: 'Mental & Emotional Regulation',
    zone: 'engine',
    subDomains: [
      {
        id: 'autonomic-nervous-system-regulation',
        number: '2.1',
        name: 'Autonomic Nervous System (ANS) Regulation',
      },
      {
        id: 'cognitive-appraisal-and-reframing',
        number: '2.2',
        name: 'Cognitive Appraisal & Reframing',
      },
      {
        id: 'emotional-granularity-and-agility',
        number: '2.3',
        name: 'Emotional Granularity & Agility',
      },
      {
        id: 'dopaminergic-baseline-and-impulse-control',
        number: '2.4',
        name: 'Dopaminergic Baseline & Impulse Control',
      },
      {
        id: 'trauma-integration-and-shadow-resolution',
        number: '2.5',
        name: 'Trauma Integration & Shadow Resolution',
      },
      {
        id: 'psychological-boundaries-and-energy-protection',
        number: '2.6',
        name: 'Psychological Boundaries & Energy Protection',
      },
    ],
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
    name: 'Cognitive & Intellectual Capital',
    zone: 'engine',
    subDomains: [
      {
        id: 'attentional-control-and-deep-work',
        number: '3.1',
        name: 'Attentional Control & Deep Work',
      },
      {
        id: 'knowledge-acquisition-and-synthesis',
        number: '3.2',
        name: 'Knowledge Acquisition & Synthesis',
      },
      {
        id: 'mental-models-and-decision-architecture',
        number: '3.3',
        name: 'Mental Models & Decision Architecture',
      },
      {
        id: 'creative-problem-solving-and-divergent-thinking',
        number: '3.4',
        name: 'Creative Problem-Solving & Divergent Thinking',
      },
      {
        id: 'epistemic-hygiene-and-information-diet',
        number: '3.5',
        name: 'Epistemic Hygiene & Information Diet',
      },
      {
        id: 'memory-and-expressive-articulation',
        number: '3.6',
        name: 'Memory & Expressive Articulation',
      },
    ],
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
    name: 'Career & Professional Impact',
    zone: 'engine',
    subDomains: [
      { id: 'skill-rarity', number: '4.1', name: 'Skill Rarity (Career Capital)' },
      { id: 'market-leverage', number: '4.2', name: 'Market Leverage' },
      { id: 'autonomy', number: '4.3', name: 'Autonomy' },
      { id: 'execution-velocity', number: '4.4', name: 'Execution Velocity' },
      { id: 'professional-network', number: '4.5', name: 'Professional Network' },
      { id: 'strategic-positioning', number: '4.6', name: 'Strategic Positioning' },
    ],
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
    subDomains: [
      { id: 'cash-flow-management', number: '5.1', name: 'Cash-Flow Management' },
      { id: 'liquid-runway', number: '5.2', name: 'Liquid Runway' },
      { id: 'automated-investing', number: '5.3', name: 'Automated Investing' },
      { id: 'debt-optimization', number: '5.4', name: 'Debt Optimization' },
      { id: 'income-diversification', number: '5.5', name: 'Income Diversification' },
      { id: 'risk-and-insurance', number: '5.6', name: 'Risk & Insurance' },
    ],
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
    name: 'Environment & Logistics',
    zone: 'engine',
    subDomains: [
      { id: 'workspace-ergonomics', number: '9.1', name: 'Workspace Ergonomics' },
      { id: 'digital-hygiene', number: '9.2', name: 'Digital Hygiene' },
      { id: 'commute-friction', number: '9.3', name: 'Commute Friction' },
      { id: 'air-and-light-quality', number: '9.4', name: 'Air & Light Quality' },
      { id: 'life-admin-automation', number: '9.5', name: 'Life Admin Automation' },
      { id: 'geographic-fit', number: '9.6', name: 'Geographic Fit' },
    ],
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
    subDomains: [
      { id: 'psychological-safety', number: '6.1', name: 'Psychological Safety' },
      { id: 'conflict-repair-velocity', number: '6.2', name: 'Conflict Repair Velocity' },
      { id: 'shared-vision', number: '6.3', name: 'Shared Vision' },
      {
        id: 'physical-and-emotional-intimacy',
        number: '6.4',
        name: 'Physical & Emotional Intimacy',
      },
      { id: 'boundaries', number: '6.5', name: 'Boundaries' },
      { id: 'shared-logistics', number: '6.6', name: 'Shared Logistics' },
    ],
    keywords: ['partner', 'romantic', 'relationship', 'spouse', 'dating', 'marriage', 'intimacy'],
  },
  {
    id: 'social',
    number: 7,
    name: 'Social & Community Network',
    zone: 'sanctuary',
    subDomains: [
      { id: 'inner-circle', number: '7.1', name: 'Inner Circle (3-5 Anchors)' },
      { id: 'mentorship-and-peers', number: '7.2', name: 'Mentorship & Peers' },
      { id: 'third-place-community', number: '7.3', name: 'Third-Place Community' },
      { id: 'social-vitality', number: '7.4', name: 'Social Vitality' },
      { id: 'boundary-enforcement', number: '7.5', name: 'Boundary Enforcement' },
      { id: 'hospitality', number: '7.6', name: 'Hospitality' },
    ],
    keywords: ['friend', 'social', 'community', 'network', 'tribe'],
  },
  {
    id: 'family',
    number: 8,
    name: 'Family & Kinship',
    zone: 'sanctuary',
    subDomains: [
      { id: 'intergenerational-boundaries', number: '8.1', name: 'Intergenerational Boundaries' },
      { id: 'caregiving-logistics', number: '8.2', name: 'Caregiving Logistics' },
      { id: 'intentional-connection', number: '8.3', name: 'Intentional Connection' },
      { id: 'conflict-resolution', number: '8.4', name: 'Conflict Resolution' },
      {
        id: 'financial-and-emotional-independence',
        number: '8.5',
        name: 'Financial & Emotional Independence',
      },
    ],
    keywords: ['family', 'parent', 'child', 'kid', 'sibling', 'kinship'],
  },
  {
    id: 'play',
    number: 10,
    name: 'Recreation, Play & Recovery',
    zone: 'sanctuary',
    subDomains: [
      { id: 'active-flow-hobbies', number: '10.1', name: 'Active Flow Hobbies' },
      {
        id: 'true-psychological-detachment',
        number: '10.2',
        name: 'True Psychological Detachment',
      },
      { id: 'travel-and-novelty', number: '10.3', name: 'Travel & Novelty' },
      { id: 'artistic-expression', number: '10.4', name: 'Artistic Expression' },
      { id: 'nature-immersion', number: '10.5', name: 'Nature Immersion' },
      { id: 'social-play', number: '10.6', name: 'Social Play' },
    ],
    keywords: ['play', 'hobby', 'travel', 'art', 'music', 'game', 'fun', 'vacation', 'leisure'],
  },
  {
    id: 'purpose',
    number: 11,
    name: 'Character, Identity & Purpose',
    zone: 'sanctuary',
    subDomains: [
      { id: 'core-values-alignment', number: '11.1', name: 'Core Values Alignment' },
      { id: 'self-trust', number: '11.2', name: 'Self-Trust (Integrity)' },
      { id: 'existential-meaning', number: '11.3', name: 'Existential Meaning' },
      { id: 'epistemic-humility', number: '11.4', name: 'Epistemic Humility' },
      { id: 'courage-and-agency', number: '11.5', name: 'Courage & Agency' },
      { id: 'anti-fragile-identity', number: '11.6', name: 'Anti-Fragile Identity' },
    ],
    keywords: ['purpose', 'meaning', 'value', 'character', 'spiritual', 'faith', 'mission'],
  },
  {
    id: 'legacy',
    number: 12,
    name: 'Contribution & Legacy',
    zone: 'sanctuary',
    subDomains: [
      { id: 'mentorship-and-teaching', number: '12.1', name: 'Mentorship & Teaching' },
      { id: 'civic-and-community-impact', number: '12.2', name: 'Civic & Community Impact' },
      { id: 'philanthropic-allocation', number: '12.3', name: 'Philanthropic Allocation' },
      { id: 'ecological-footprint', number: '12.4', name: 'Ecological Footprint' },
      { id: 'knowledge-sharing', number: '12.5', name: 'Knowledge Sharing' },
    ],
    keywords: ['legacy', 'contribut', 'volunteer', 'charity', 'give back', 'mentor'],
  },
]

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
  const chosen = overrides[entry.id]
  if (chosen) return domainById(chosen)
  const goal = entry.approvedGoal ?? entry.goal
  const hit = suggestDomain(`${entry.category ?? ''} ${goal?.outcome ?? ''} ${entry.text}`)
  const byClass = CLASS_DOMAIN[entry.class]
  if (byClass && (!hit || hit.id === 'career')) return domainById(byClass)
  return hit ?? UNASSIGNED
}
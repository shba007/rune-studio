import type { IconName } from '@/icons'

export const FLOW_VIEWS = [
  { value: 'app' as const, label: 'App' },
  { value: 'graph' as const, label: 'Graph' },
]

export interface FlowDefinition {
  id: string
  name: string
  description: string
  icon: IconName
  command: `#${string}`
}

export const FLOWS: FlowDefinition[] = [
  {
    id: 'image-voice',
    name: 'Image to Voice',
    description: 'Describe an image and turn the description into speech.',
    icon: 'speaker',
    command: '#image to voice',
  },
  {
    id: 'visual-analysis',
    name: 'Visual Analysis',
    description: 'Analyze a selected screen, window or image.',
    icon: 'scan',
    command: '#visual analysis',
  },
  {
    id: 'portfolio-studio',
    name: 'Portfolio Studio',
    description: 'Review RAW photographs and select portfolio views.',
    icon: 'image',
    command: '#portfolio studio',
  },
]
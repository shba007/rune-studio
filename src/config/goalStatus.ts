import type { IntakeEntry } from '@/types/decisions'
import { domainById } from './domains'

export type GoalTone = 'idle' | 'ok' | 'warn' | 'down' | 'busy'

export function goalStatus(entry: IntakeEntry): { label: string; tone: GoalTone } {
  const reviewing = !!entry.question && !!entry.approvedGoal
  if (entry.status === 'discovery_planning') return { label: 'Discovering a goal', tone: 'busy' }
  if (entry.status === 'discovery_review') return { label: 'Review proposed goal', tone: 'warn' }
  if (entry.status === 'domain_planning') return { label: 'Suggesting a domain', tone: 'busy' }
  if (entry.status === 'domain_selection') return { label: 'Confirm domain', tone: 'warn' }
  if (entry.status === 'subdomain_planning')
    return { label: 'Preparing focused questions', tone: 'busy' }
  if (entry.status === 'subdomain_selection') return { label: 'Choose goal context', tone: 'warn' }
  if (entry.status === 'pathway_planning') return { label: 'Comparing routes', tone: 'busy' }
  if (entry.status === 'task_planning') return { label: 'Drafting first-week tasks', tone: 'busy' }
  if (reviewing) return { label: 'Review changes', tone: 'warn' }
  if (entry.question) return { label: 'Needs your answer', tone: 'warn' }
  if (entry.approvedGoal) return { label: 'Approved goal', tone: 'ok' }
  return { label: 'Defining your goal', tone: 'idle' }
}

export function goalTitle(entry: IntakeEntry): string {
  return (
    (entry.goal ?? entry.approvedGoal)?.outcome ||
    entry.text ||
    (entry.domainMode === 'auto' ? `${domainById(entry.domain ?? '').name} · discovery` : '')
  )
}
import type { IntakeEntry } from '@/types/decisions'

export type GoalTone = 'idle' | 'ok' | 'warn' | 'down' | 'busy'

export function goalStatus(entry: IntakeEntry): { label: string; tone: GoalTone } {
  const reviewing = !!entry.question && !!entry.approvedGoal
  if (entry.status === 'pathway_planning') return { label: 'Comparing routes', tone: 'busy' }
  if (entry.status === 'task_planning') return { label: 'Drafting first-week tasks', tone: 'busy' }
  if (reviewing) return { label: 'Review changes', tone: 'warn' }
  if (entry.question) return { label: 'Needs your answer', tone: 'warn' }
  if (entry.approvedGoal) return { label: 'Approved goal', tone: 'ok' }
  return { label: 'Defining your goal', tone: 'idle' }
}

export function goalTitle(entry: IntakeEntry): string {
  return (entry.goal ?? entry.approvedGoal)?.outcome || entry.text
}
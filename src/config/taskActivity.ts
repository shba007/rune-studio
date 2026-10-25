import type { AgentState } from '@/types/protocol'
import type { PlannedRequest } from '@/composables/useTaskBoard'
export function taskOrbState(
  state: AgentState,
  requests: Pick<PlannedRequest, 'phase'>[],
): AgentState {
  if (
    [
      'planning',
      'loading_model',
      'acting',
      'observing',
      'awaiting_confirm',
      'listening',
      'reflecting',
    ].includes(state)
  )
    return state
  if (requests.some((r) => ['queued', 'planning', 'running'].includes(r.phase))) return 'planning'
  const last = requests.at(-1)?.phase
  if (last === 'done') return 'done'
  if (last === 'failed' || last === 'blocked') return 'failed'
  if (last === 'cancelled') return 'stopped'
  return state
}
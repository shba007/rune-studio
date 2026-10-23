import type { ScreenRequestSnapshot } from '@/types/protocol'

export function screenRequestText(snapshot: ScreenRequestSnapshot): string {
  switch (snapshot.status) {
    case 'queued':
      return snapshot.waiting
        ? 'Screen analysis requested. Waiting for the agent.'
        : 'Request queued. Ask your host agent to call rune_wait_for_request.'
    case 'delivered':
      return 'Request delivered. Waiting for the agent to capture the screen.'
    case 'captured':
      return 'Screen sent to your agent. Read its analysis in your host.'
    case 'cancelled':
      return 'Screen request cancelled.'
    case 'expired':
      return 'Screen request expired. Connect your agent and try again.'
    default:
      return snapshot.waiting
        ? 'External agent is waiting. Click Screen Analysis to request a capture.'
        : 'To use Screen Analysis, ask your host agent to call rune_wait_for_request.'
  }
}
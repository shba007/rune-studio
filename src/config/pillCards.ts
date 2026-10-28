export type PillCard =
  | 'confirm'
  | 'analysis'
  | 'decision'
  | 'decisionLink'
  | 'taskPanel'
  | 'board'
  | 'server'
  | 'reflection'
  | 'review'
  | 'feedback'
  | 'update'
  | 'reminder'

export interface PillCardState {
  question: boolean
  analysis: boolean
  decision: boolean
  decisionInKnowledge: boolean
  taskPanel: boolean
  board: boolean
  working: boolean
  server: boolean
  reflection: boolean
  review: boolean
  feedback: boolean
  update: boolean
}

export interface PillCards {
  cards: PillCard[]
  more: number
}

export function pillCards(state: PillCardState): PillCards {
  if (state.question) return { cards: ['confirm'], more: 0 }

  const cards: PillCard[] = []
  if (state.analysis) cards.push('analysis')
  else if (state.decisionInKnowledge) cards.push('decisionLink')
  else if (state.decision) cards.push('decision')
  else if (state.taskPanel) cards.push('taskPanel')
  else if (state.board) cards.push('board')

  const blocked = state.analysis || state.decision || state.decisionInKnowledge || state.working
  if (blocked) return { cards, more: 0 }

  const quiet: PillCard[] = []
  if (state.server) quiet.push('server')
  if (state.reflection) quiet.push('reflection')
  if (state.review) quiet.push('review')
  else if (state.feedback) quiet.push('feedback')
  if (state.update) quiet.push('update')

  if (quiet.length) {
    cards.push(quiet[0]!)
    return { cards, more: quiet.length - 1 }
  }
  cards.push('reminder')
  return { cards, more: 0 }
}
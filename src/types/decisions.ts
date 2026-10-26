export interface DecisionOption {
  id: string
  label: string
  explanation: string
}

export interface DecisionQuestion {
  id: string
  revision: number
  kind: string
  title: string
  explanation: string
  options: DecisionOption[]
  draft?: { optionId: string | null; text: string }
}

export interface DecisionAnswer {
  id: string
  revision: number
  optionId: string | null
  text: string
}

export interface IntakeEntry {
  id: string
  revision: number
  text: string
  class: 'general' | 'health' | 'mind' | 'finance'
  kind: string
  status: string
  question: DecisionQuestion | null
  goal: Record<string, string> | null
  approvedGoal?: Record<string, string> | null
  error: string | null
  taskId: string | null
}
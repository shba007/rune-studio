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
  domain?: string | null
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
  category?: string
  domain?: string | null
  domainSource?: 'user' | 'auto' | null
  domainMode?: 'manual' | 'auto' | null
  subDomains?: { id: string; answers: unknown[]; skipped: boolean }[]
  subdomainProgress?: {
    selectionDone: boolean
    current: string | null
    completed: string[]
    finished: boolean
  } | null
  question: DecisionQuestion | null
  goal: Record<string, string> | null
  approvedGoal?: Record<string, string> | null
  outcomeMetric?: import('./workspace').OutcomeMetric | null
  taskProposals?: GoalTaskDraft | null
  pathways?: {
    assessment: string
    similarOutcome: string
    options: GoalPathway[]
    selected: string | null
    approved: GoalPathway | null
  } | null
  error: string | null
  taskId: string | null
}
export interface GoalPathway {
  id: string
  title: string
  summary: string
  rationale: string
  pros: string[]
  cons: string[]
  duration: string
  resources: string[]
  feasibility: 'feasible' | 'conditional' | 'infeasible' | 'unknown'
  missingInformation: string[]
}
export interface ProposalSchedule {
  local: string
  zone: string
  repeat: 'once' | 'daily' | 'weekly' | 'monthly'
  anchorDay: number | null
}
export interface TaskApproval {
  title: string
  instructions: string
  executionMode: 'manual' | 'agent' | 'analysis'
  schedule: ProposalSchedule | null
}
export interface ProposedGoalTask extends TaskApproval {
  id: string
  rationale: string
  createdTaskId: string | null
  dismissed: boolean
}
export interface GoalTaskDraft {
  id: string
  goal: Record<string, string>
  pathway: GoalPathway
  startLocal: string
  endLocal: string
  zone: string
  assessment: string
  tasks: ProposedGoalTask[]
}
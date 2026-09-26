export type MediaType = 'image' | 'video'

export interface MediaItem {
  id: string
  name: string
  path: string
  size: number
  mimeType: string
  mediaType: MediaType
  lastModified: number
  thumbnailUrl?: string
  thumbnailPath?: string
}

export type StagedActionType = 'MOVE_SAME_DIR' | 'QUARANTINE_DUPLICATE' | 'MOVE_TO_TRASH'

export interface StagedAction {
  id: string
  type: StagedActionType
  sourceItem: MediaItem
  matchedWith?: MediaItem
  targetDirectory?: string
  targetPath?: string
  confidence: number
  reason: string
  enabled: boolean
}

export interface IdentityCluster {
  id: string
  suggestedName: string
  customName: string
  primaryItem: MediaItem
  exemplars: MediaItem[]
  items: MediaItem[]
  confidenceAverage: number
}

export interface PipelineProgress {
  current: number
  total: number
  currentItem: MediaItem | null
  currentStage: string
}

export type PipelineState = 'idle' | 'profiling' | 'clustering' | 'review' | 'completed'
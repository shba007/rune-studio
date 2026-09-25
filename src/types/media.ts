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
}

export interface ComparisonVerdict {
  isMatch: boolean
  confidence: number
  similarityType?: string
  explanation: string
  thought?: string
  visualSummaryA?: string
  visualSummaryB?: string
  recommendedPrimary?: 'item_a' | 'item_b'
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
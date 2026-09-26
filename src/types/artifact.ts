export type ResolutionClass = 'high' | 'medium' | 'low'
export type IlluminationClass = 'uniform' | 'side_shadow' | 'backlit' | 'dim' | 'harsh'
export type OcclusionLevel = 'none' | 'partial_under_25' | 'heavy_under_60' | 'severe_over_60'
export type MotionBlurClass = 'none' | 'slight' | 'severe'

export interface ObservationQuality {
  resolutionClass: ResolutionClass
  illumination: IlluminationClass
  occlusionLevel: OcclusionLevel
  motionBlur: MotionBlurClass
}

export type ViewAngle =
  | 'frontal'
  | 'three_quarter'
  | 'profile'
  | 'rear_or_back'
  | 'close_detail'
  | 'unobservable'

export interface AssetAssessment {
  subjectDetected: boolean
  targetCategory: 'person' | 'product' | 'scene' | 'document' | 'other'
  viewAngle: ViewAngle
  observationQuality: ObservationQuality
  salientVisualSummary: string
  distinctiveFeatures: string[]
}

export interface ArbitrationVerdict {
  isMatch: boolean
  confidence: number
  verdictReason: string
  alignedKeyFeatures: string[]
  contradictoryFeatures: string[]
}

export interface MediaArtifact {
  id: string
  fileHash: string
  sourcePath: string
  mediaType: 'image' | 'video'
  createdAt: number
  fidelityScore: number
  assessment: AssetAssessment
  isUsable: boolean
}
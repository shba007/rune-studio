import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type {
  MediaItem,
  StagedAction,
  StagedActionType,
  IdentityCluster,
  PipelineProgress,
  PipelineState,
} from '@/types/media'
import type { MediaArtifact, ArbitrationVerdict } from '@/types/artifact'

export interface IntentPreset {
  id: string
  label: string
  shortLabel: string
  badge: string
  description: string
}

export const COMPARISON_INTENTS: Record<string, IntentPreset> = {
  same_person: {
    id: 'same_person',
    label: 'Identity / Person',
    shortLabel: 'Person',
    badge: '👤 Face & Identity',
    description: 'Invariant facial biometrics and craniofacial arbitration via side-by-side VLM.',
  },
  same_product: {
    id: 'same_product',
    label: 'Product / Physical Item',
    shortLabel: 'Product',
    badge: '📦 Product Model',
    description: 'Hardware silhouette, seam pattern, and branding alignment ignoring model/wearer.',
  },
  duplicates: {
    id: 'duplicates',
    label: 'Near / Exact Duplicate',
    shortLabel: 'Duplicate',
    badge: '📑 Deduplication',
    description: 'Photographic framing, camera angle, and composition deduplication.',
  },
  same_category: {
    id: 'same_category',
    label: 'Theme / Scene Category',
    shortLabel: 'Theme',
    badge: '🎨 Scene Theme',
    description: 'Environmental ambiance, setting type, and thematic grouping.',
  },
}

const SAVED_MODEL_KEY = 'rune_selected_model'
const SAVED_RECENTS_KEY = 'rune_recent_scanned_dirs'

function getStoredRecentDirectories(): string[] {
  try {
    const stored = localStorage.getItem(SAVED_RECENTS_KEY)
    return stored ? JSON.parse(stored) : []
  } catch {
    return []
  }
}

export interface ServerHealthInfo {
  status: string
  version: string
  loadedModels: string[]
}

export const useMediaStore = defineStore('media', () => {
  const directoryPath = ref('')
  const isRecursive = ref(true)
  const mediaItems = ref<MediaItem[]>([])
  const recentDirectories = ref<string[]>(getStoredRecentDirectories())

  const filterType = ref<'all' | 'image' | 'video'>('all')
  const searchQuery = ref('')

  const selectedMedia = ref<MediaItem | null>(null)

  // Caches
  const artifactsCache = ref<Record<string, MediaArtifact>>({})
  const arbitrationCache = ref<Record<string, ArbitrationVerdict>>({})

  const selectedIntentKey = ref<string>('same_person')
  const confidenceThreshold = ref<number>(0.8)
  const isThinkingEnabled = ref<boolean>(false)

  const detectedModelName = ref<string>(localStorage.getItem(SAVED_MODEL_KEY) || '')
  const availableModels = ref<string[]>([])
  const isFetchingModels = ref<boolean>(false)
  const isVisionCapable = ref<boolean | null>(null)
  const isCheckingModel = ref<boolean>(false)
  const modelCheckMessage = ref<string>('')
  const serverHealth = ref<ServerHealthInfo | null>(null)

  const stagedActions = ref<StagedAction[]>([])
  const isReviewModalOpen = ref(false)

  const pipelineState = ref<PipelineState>('idle')
  const pipelineProgress = ref<PipelineProgress>({
    current: 0,
    total: 0,
    currentItem: null,
    currentStage: '',
  })
  const identityClusters = ref<IdentityCluster[]>([])
  const isIdentityReviewOpen = ref(false)

  const currentIntent = computed<IntentPreset>(() => {
    return COMPARISON_INTENTS[selectedIntentKey.value] ?? COMPARISON_INTENTS['same_person']!
  })

  const filteredMedia = computed(() => {
    return mediaItems.value.filter((item) => {
      const matchesType = filterType.value === 'all' || item.mediaType === filterType.value
      const matchesSearch =
        !searchQuery.value.trim() ||
        item.name.toLowerCase().includes(searchQuery.value.toLowerCase())
      return matchesType && matchesSearch
    })
  })

  const activeStagedCount = computed(() => {
    return stagedActions.value.filter((a) => a.enabled).length
  })

  const selectedMediaArtifact = computed<MediaArtifact | null>(() => {
    if (!selectedMedia.value) return null
    return artifactsCache.value[selectedMedia.value.path] || null
  })

  const cachedArtifactsCount = computed(() => {
    return Object.keys(artifactsCache.value).length
  })

  function setDirectory(path: string) {
    if (!path) return
    directoryPath.value = path
    const existing = recentDirectories.value.filter((p) => p !== path)
    recentDirectories.value = [path, ...existing].slice(0, 8)
    try {
      localStorage.setItem(SAVED_RECENTS_KEY, JSON.stringify(recentDirectories.value))
    } catch {}
  }

  function setMediaItems(items: MediaItem[]) {
    mediaItems.value = items
    const ids = new Set(items.map((i) => i.id))
    if (selectedMedia.value && !ids.has(selectedMedia.value.id)) {
      selectedMedia.value = null
    }
  }

  function updateThumbnails(
    updates: { id: string; thumbnailUrl: string; thumbnailPath?: string }[],
  ) {
    if (updates.length === 0) return
    const map = new Map(updates.map((u) => [u.id, u]))
    for (const item of mediaItems.value) {
      const u = map.get(item.id)
      if (u) {
        item.thumbnailUrl = u.thumbnailUrl
        if (u.thumbnailPath) {
          item.thumbnailPath = u.thumbnailPath
        }
      }
    }
  }

  function selectMedia(item: MediaItem) {
    if (selectedMedia.value?.id === item.id) {
      selectedMedia.value = null
    } else {
      selectedMedia.value = item
    }
  }

  function clearSelection() {
    selectedMedia.value = null
  }

  function setModel(modelName: string) {
    detectedModelName.value = modelName
    localStorage.setItem(SAVED_MODEL_KEY, modelName)
    isVisionCapable.value = null
  }

  function stageAction(params: {
    type: StagedActionType
    sourceItem: MediaItem
    matchedWith?: MediaItem
    targetDirectory?: string
    targetPath?: string
    confidence: number
    reason: string
  }) {
    const action: StagedAction = {
      id: `${Date.now()}-${Math.random().toString(36).substring(2, 7)}`,
      enabled: true,
      ...params,
    }
    stagedActions.value.push(action)
  }

  function removeStagedAction(id: string) {
    stagedActions.value = stagedActions.value.filter((a) => a.id !== id)
  }

  function toggleStagedAction(id: string) {
    const action = stagedActions.value.find((a) => a.id === id)
    if (action) {
      action.enabled = !action.enabled
    }
  }

  function clearStagedActions() {
    stagedActions.value = []
  }

  function removeItemFromCluster(clusterId: string, itemId: string) {
    const cluster = identityClusters.value.find((c) => c.id === clusterId)
    if (!cluster) return
    cluster.items = cluster.items.filter((item) => item.id !== itemId)
    cluster.exemplars = cluster.exemplars.filter((item) => item.id !== itemId)
    if (cluster.items.length === 0) {
      identityClusters.value = identityClusters.value.filter((c) => c.id !== clusterId)
    }
  }

  function removeCluster(clusterId: string) {
    identityClusters.value = identityClusters.value.filter((c) => c.id !== clusterId)
  }

  return {
    directoryPath,
    isRecursive,
    mediaItems,
    recentDirectories,
    filterType,
    searchQuery,
    selectedMedia,
    selectedMediaArtifact,
    cachedArtifactsCount,

    artifactsCache,
    arbitrationCache,
    selectedIntentKey,
    confidenceThreshold,
    isThinkingEnabled,
    detectedModelName,
    availableModels,
    isFetchingModels,
    isVisionCapable,
    isCheckingModel,
    modelCheckMessage,
    serverHealth,
    stagedActions,
    isReviewModalOpen,

    pipelineState,
    pipelineProgress,
    identityClusters,
    isIdentityReviewOpen,

    currentIntent,
    filteredMedia,
    activeStagedCount,

    setDirectory,
    setMediaItems,
    updateThumbnails,
    selectMedia,
    clearSelection,
    setModel,
    stageAction,
    removeStagedAction,
    toggleStagedAction,
    clearStagedActions,
    removeItemFromCluster,
    removeCluster,
  }
})
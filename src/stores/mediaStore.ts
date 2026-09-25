import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { MediaItem, ComparisonVerdict, StagedAction, StagedActionType } from '@/types/media'

export interface IntentPreset {
  id: string
  label: string
  shortLabel: string
  badge: string
  description: string
  prompt: string
}

export const COMPARISON_INTENTS: Record<string, IntentPreset> = {
  same_person: {
    id: 'same_person',
    label: 'Identity / Person',
    shortLabel: 'Person',
    badge: '👤 Face & Identity',
    description:
      'Matches the same individual across video frames, photos, angles, and lighting while ignoring clothes/hair.',
    prompt:
      'You are performing BIOLOGICAL & FACIAL IDENTITY VERIFICATION. Determine whether the person in Item A and Item B is the EXACT SAME INDIVIDUAL or DIFFERENT PEOPLE.\n' +
      'RULES FOR HIGH ACCURACY:\n' +
      '1. Be skeptical and rigorous: Two subjects sharing similar age, gender, ethnicity, skin tone, or hair color are NOT a match unless their distinctive facial bone structure, eye shape, nose bridge/tip, lip contour, and facial proportions match.\n' +
      '2. If the facial geometry, jawline, nose structure, or eye spacing belong to different individuals, you MUST output isMatch: false with high confidence.\n' +
      '3. Disregard superficial similarities such as makeup, clothes, lighting, pose, or background.\n' +
      '4. If clear facial features are absent, cropped out, or too blurred in either item to prove identity, output isMatch: false.\n' +
      '5. Only output isMatch: true if you are certain they are the exact same person.',
  },
  same_product: {
    id: 'same_product',
    label: 'Product / Physical Item',
    shortLabel: 'Product',
    badge: '📦 Product Model',
    description:
      'Matches the exact product structure or model, ignoring surrounding staging props, countertops, or room fixtures.',
    prompt:
      'You are performing COMMERCIAL PRODUCT IDENTIFICATION. Determine whether Image A and Image B feature the EXACT SAME PRODUCT MODEL, APPAREL, OR HARDWARE ITEM.\n' +
      'RULES FOR HIGH ACCURACY:\n' +
      '1. Compare primary item structure: geometry, wire patterns, seams, materials, branding/logos, dimensions, and functional components.\n' +
      '2. STRICTLY IGNORE BACKGROUND STAGING: Surrounding kitchen or studio props, countertops, faucets, toasters, fruit bowls, background walls, or lighting variations MUST BE COMPLETELY DISREGARDED.\n' +
      '3. If the primary product or item being displayed is the same model/design, output isMatch: true.',
  },
  duplicates: {
    id: 'duplicates',
    label: 'Near / Exact Duplicate',
    shortLabel: 'Duplicate',
    badge: '📑 File Deduplication',
    description:
      'Detects burst shots, identical captures, resized copies, or converted formats to clean storage.',
    prompt:
      'You are performing DEDUPLICATION DETECTION. Evaluate whether Image A and Image B represent the SAME PHOTOGRAPHIC CAPTURE, BURST SHOT, RESIZED VERSION, OR FILE DUPLICATE.\n' +
      'RULES FOR HIGH ACCURACY:\n' +
      '1. Compare the full frame: camera angle, exact subject posture, overall background arrangement, and composition.\n' +
      '2. If background elements, props, or camera perspective significantly differ, it is NOT an exact duplicate (isMatch: false).\n' +
      '3. Minor compression artifacts, resolutions, or color grading differences should still be classified as duplicates (isMatch: true).',
  },
  same_category: {
    id: 'same_category',
    label: 'Theme / Scene Category',
    shortLabel: 'Theme',
    badge: '🎨 Scene Theme',
    description: 'Groups media sharing the same scene context, activity, or visual category.',
    prompt:
      'You are performing THEMATIC AND SCENE CLUSTERING. Determine whether Image A and Image B belong to the SAME EVENT, SCENE THEME, OR CATEGORY (e.g. glamour photoshoot, kitchen interior, document, vacation).\n' +
      'RULES:\n' +
      '1. Evaluate environmental aesthetics, scene context, and overall activity.\n' +
      '2. Specific person identity or exact product model differences do not prevent a category match.',
  },
  custom: {
    id: 'custom',
    label: 'Custom Instruction',
    shortLabel: 'Custom',
    badge: '⚙️ Custom Prompt',
    description: 'Provide custom comparison instructions in the prompt bar.',
    prompt: '',
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

export const useMediaStore = defineStore('media', () => {
  const directoryPath = ref('')
  const isRecursive = ref(true)
  const mediaItems = ref<MediaItem[]>([])
  const recentDirectories = ref<string[]>(getStoredRecentDirectories())

  const filterType = ref<'all' | 'image' | 'video'>('all')
  const searchQuery = ref('')

  const selectedItemA = ref<MediaItem | null>(null)
  const selectedItemB = ref<MediaItem | null>(null)

  const selectedIntentKey = ref<string>('same_person')
  const customPromptText = ref('')
  const confidenceThreshold = ref<number>(0.7)
  const isThinkingEnabled = ref<boolean>(true)

  const detectedModelName = ref<string>(localStorage.getItem(SAVED_MODEL_KEY) || '')
  const availableModels = ref<string[]>([])
  const isFetchingModels = ref<boolean>(false)
  const modelsFetchError = ref<string | null>(null)
  const isVisionCapable = ref<boolean | null>(null)
  const isCheckingModel = ref<boolean>(false)
  const modelCheckMessage = ref<string>('')

  const isComparing = ref(false)
  const comparisonError = ref<string | null>(null)
  const lastVerdict = ref<ComparisonVerdict | null>(null)

  const stagedActions = ref<StagedAction[]>([])
  const isReviewModalOpen = ref(false)

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

  const effectivePrompt = computed(() => {
    if (selectedIntentKey.value === 'custom') {
      return customPromptText.value.trim() || 'Visually compare Item A and Item B for similarity.'
    }
    return currentIntent.value.prompt
  })

  const canCompare = computed(() => {
    return (
      Boolean(detectedModelName.value) &&
      isVisionCapable.value !== false &&
      selectedItemA.value !== null &&
      selectedItemB.value !== null &&
      selectedItemA.value.id !== selectedItemB.value.id &&
      !isComparing.value
    )
  })

  const isThresholdMet = computed(() => {
    if (!lastVerdict.value) return false
    return lastVerdict.value.isMatch && lastVerdict.value.confidence >= confidenceThreshold.value
  })

  const activeStagedCount = computed(() => {
    return stagedActions.value.filter((a) => a.enabled).length
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
    if (selectedItemA.value && !ids.has(selectedItemA.value.id)) selectedItemA.value = null
    if (selectedItemB.value && !ids.has(selectedItemB.value.id)) selectedItemB.value = null
  }

  function selectMedia(item: MediaItem) {
    if (!selectedItemA.value) {
      selectedItemA.value = item
    } else if (selectedItemA.value.id === item.id) {
      selectedItemA.value = null
    } else if (!selectedItemB.value) {
      selectedItemB.value = item
    } else if (selectedItemB.value.id === item.id) {
      selectedItemB.value = null
    } else {
      selectedItemB.value = item
    }
    lastVerdict.value = null
    comparisonError.value = null
  }

  function clearSelection() {
    selectedItemA.value = null
    selectedItemB.value = null
    lastVerdict.value = null
    comparisonError.value = null
  }

  function setComparisonVerdict(verdict: ComparisonVerdict | null) {
    lastVerdict.value = verdict
  }

  function setModel(modelName: string) {
    detectedModelName.value = modelName
    localStorage.setItem(SAVED_MODEL_KEY, modelName)
    isVisionCapable.value = null
    lastVerdict.value = null
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
    const action = stagedActions.value.find((a) => a.id !== id)
    if (action) {
      action.enabled = !action.enabled
    }
  }

  function clearStagedActions() {
    stagedActions.value = []
  }

  return {
    directoryPath,
    isRecursive,
    mediaItems,
    recentDirectories,
    filterType,
    searchQuery,
    selectedItemA,
    selectedItemB,
    selectedIntentKey,
    customPromptText,
    confidenceThreshold,
    isThinkingEnabled,
    detectedModelName,
    availableModels,
    isFetchingModels,
    modelsFetchError,
    isVisionCapable,
    isCheckingModel,
    modelCheckMessage,
    isComparing,
    comparisonError,
    lastVerdict,
    stagedActions,
    isReviewModalOpen,

    currentIntent,
    filteredMedia,
    effectivePrompt,
    canCompare,
    isThresholdMet,
    activeStagedCount,

    setDirectory,
    setMediaItems,
    selectMedia,
    clearSelection,
    setComparisonVerdict,
    setModel,
    stageAction,
    removeStagedAction,
    toggleStagedAction,
    clearStagedActions,
  }
})
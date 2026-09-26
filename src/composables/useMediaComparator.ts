import { ref } from 'vue'
import { ofetch } from 'ofetch'
import { useMediaStore } from '@/stores/mediaStore'

const API_ENDPOINT = import.meta.env.VITE_AI_API_URL
const HEALTH_ENDPOINT = `${API_ENDPOINT}/health`
const MODELS_ENDPOINT = `${API_ENDPOINT}/v1/models`

interface HealthResponse {
  status: string
  version: string
  loaded_models?: string[]
}

interface ModelsResponse {
  data?: Array<{
    id?: string
    name?: string
    capabilities?: {
      vision?: boolean
      image?: boolean
      video?: boolean
      [key: string]: unknown
    }
    modalities?: string[]
  }>
}

export function useMediaComparator() {
  const isComparing = ref(false)
  const error = ref<string | null>(null)
  const mediaStore = useMediaStore()

  async function checkServerHealth(): Promise<boolean> {
    try {
      const healthData = await ofetch<HealthResponse>(HEALTH_ENDPOINT, {
        method: 'GET',
      })

      if (healthData && healthData.status === 'ok') {
        mediaStore.serverHealth = {
          status: healthData.status,
          version: healthData.version,
          loadedModels: healthData.loaded_models || [],
        }

        if (healthData.loaded_models && healthData.loaded_models.length > 0) {
          mediaStore.availableModels = healthData.loaded_models
          if (
            !mediaStore.detectedModelName ||
            !healthData.loaded_models.includes(mediaStore.detectedModelName)
          ) {
            mediaStore.detectedModelName = healthData.loaded_models[0]!
          }
        }
        return true
      }
    } catch (err) {
      console.warn('[useMediaComparator] /health ping failed:', err)
    }
    return false
  }

  async function checkVisionCapability(
    overrideModel?: string,
  ): Promise<{ isVision: boolean; modelName: string }> {
    mediaStore.isCheckingModel = true
    mediaStore.modelCheckMessage = 'Probing server status...'

    if (overrideModel) {
      mediaStore.detectedModelName = overrideModel
    }

    await checkServerHealth()

    try {
      const modelsData = await ofetch<ModelsResponse>(MODELS_ENDPOINT, {
        method: 'GET',
      })

      if (modelsData?.data && modelsData.data.length > 0) {
        const validModels = modelsData.data.filter((m) => Boolean(m.id || m.name))
        const modelIds: string[] = validModels
          .map((m) => m.id || m.name || '')
          .filter((name): name is string => Boolean(name))

        if (modelIds.length > 0) {
          mediaStore.availableModels = modelIds
        }

        if (!mediaStore.detectedModelName || !modelIds.includes(mediaStore.detectedModelName)) {
          mediaStore.detectedModelName = modelIds[0]!
        }

        const activeObj = validModels.find((m) => (m.id || m.name) === mediaStore.detectedModelName)

        if (activeObj?.capabilities?.vision !== undefined) {
          const capable = Boolean(activeObj.capabilities.vision)
          mediaStore.isVisionCapable = capable
          mediaStore.modelCheckMessage = capable
            ? `Vision verified via model capabilities`
            : `Model indicates no vision capability`
          mediaStore.isCheckingModel = false
          return { isVision: capable, modelName: mediaStore.detectedModelName }
        }

        if (activeObj?.modalities && Array.isArray(activeObj.modalities)) {
          const hasImage = activeObj.modalities.some((mod) =>
            ['image', 'vision', 'video'].includes(String(mod).toLowerCase()),
          )
          mediaStore.isVisionCapable = hasImage
          mediaStore.modelCheckMessage = hasImage
            ? `Vision & video verified via endpoint modalities`
            : `Text-only model modalities detected`
          mediaStore.isCheckingModel = false
          return { isVision: hasImage, modelName: mediaStore.detectedModelName }
        }
      }
    } catch (err) {
      console.warn('[useMediaComparator] /v1/models query failed:', err)
    }

    const activeModel = mediaStore.detectedModelName
    const modelLower = activeModel.toLowerCase()
    const isKnownNonVision = /needle|cactus|coder|deepseek-r1-distill|text-embedding/i.test(
      modelLower,
    )
    const isKnownVision = /qwen|vl|vision|omni|llava|pixtral|minicpm|paligemma|glm-4v/i.test(
      modelLower,
    )

    const isVision = isKnownVision && !isKnownNonVision
    mediaStore.isVisionCapable = isVision
    mediaStore.modelCheckMessage = isVision
      ? `Multimodal capability confirmed for ${activeModel}`
      : `Text-only model identified: ${activeModel}`
    mediaStore.isCheckingModel = false

    return { isVision, modelName: activeModel }
  }

  return {
    isComparing,
    error,
    checkServerHealth,
    checkVisionCapability,
  }
}
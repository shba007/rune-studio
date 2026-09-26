import { ref } from 'vue'
import { ofetch } from 'ofetch'
import type { MediaItem } from '@/types/media'
import type { MediaArtifact, AssetAssessment, ArbitrationVerdict } from '@/types/artifact'
import { useMediaScanner } from '@/composables/useMediaScanner'
import { useMediaStore } from '@/stores/mediaStore'
import { calculateFidelityScore, buildArbitrationPrompt } from '@/utils/artifactEvaluator'

const API_ENDPOINT = `${import.meta.env.VITE_AI_API_URL}/v1/chat/completions`

const S3_BUCKET_URL: string = import.meta.env.VITE_S3_BUCKET_URL
if (!S3_BUCKET_URL) {
  throw new Error('FATAL: VITE_S3_BUCKET_URL is missing in environment.')
}

interface VlmCompletionResponse {
  choices?: Array<{
    message?: {
      content?: string
    }
  }>
}

const ASSESSMENT_JSON_SCHEMA = {
  type: 'object',
  properties: {
    subjectDetected: { type: 'boolean' },
    targetCategory: {
      type: 'string',
      enum: ['person', 'product', 'scene', 'document', 'other'],
    },
    viewAngle: {
      type: 'string',
      enum: ['frontal', 'three_quarter', 'profile', 'rear_or_back', 'close_detail', 'unobservable'],
    },
    observationQuality: {
      type: 'object',
      properties: {
        resolutionClass: { type: 'string', enum: ['high', 'medium', 'low'] },
        illumination: {
          type: 'string',
          enum: ['uniform', 'side_shadow', 'backlit', 'dim', 'harsh'],
        },
        occlusionLevel: {
          type: 'string',
          enum: ['none', 'partial_under_25', 'heavy_under_60', 'severe_over_60'],
        },
        motionBlur: { type: 'string', enum: ['none', 'slight', 'severe'] },
      },
      required: ['resolutionClass', 'illumination', 'occlusionLevel', 'motionBlur'],
    },
    salientVisualSummary: { type: 'string' },
    distinctiveFeatures: {
      type: 'array',
      items: { type: 'string' },
    },
  },
  required: [
    'subjectDetected',
    'targetCategory',
    'viewAngle',
    'observationQuality',
    'salientVisualSummary',
    'distinctiveFeatures',
  ],
}

const ARBITRATION_JSON_SCHEMA = {
  type: 'object',
  properties: {
    isMatch: { type: 'boolean' },
    confidence: { type: 'number' },
    verdictReason: { type: 'string' },
    alignedKeyFeatures: { type: 'array', items: { type: 'string' } },
    contradictoryFeatures: { type: 'array', items: { type: 'string' } },
  },
  required: [
    'isMatch',
    'confidence',
    'verdictReason',
    'alignedKeyFeatures',
    'contradictoryFeatures',
  ],
}

function parseJsonSafe<T>(raw: string): T {
  let cleaned = raw.trim()
  if (cleaned.includes('<think>')) {
    cleaned = cleaned.replace(/<think>[\s\S]*?<\/think>/gi, '').trim()
  }
  const codeBlockMatch = cleaned.match(/```(?:json)?\s*([\s\S]*?)\s*```/i)
  if (codeBlockMatch && codeBlockMatch[1]) {
    cleaned = codeBlockMatch[1].trim()
  }
  const firstBrace = cleaned.indexOf('{')
  const lastBrace = cleaned.lastIndexOf('}')
  if (firstBrace !== -1 && lastBrace !== -1 && lastBrace > firstBrace) {
    cleaned = cleaned.substring(firstBrace, lastBrace + 1)
  }
  return JSON.parse(cleaned) as T
}

export function useMediaProfiler() {
  const isProfiling = ref(false)
  const profilerError = ref<string | null>(null)
  const mediaStore = useMediaStore()
  const {
    uploadMediaToS3,
    saveArtifactToDisk,
    loadArtifactFromDisk,
    clearDirectoryCache,
    deleteArtifactFromDisk,
  } = useMediaScanner()

  // In-memory cache for uploaded S3 URLs to avoid duplicate uploads
  const uploadedUrls = new Map<string, string>()

  async function ensureUploadedUrl(item: MediaItem): Promise<string> {
    if (uploadedUrls.has(item.path)) {
      return uploadedUrls.get(item.path)!
    }
    const url = await uploadMediaToS3(item.path, S3_BUCKET_URL)
    uploadedUrls.set(item.path, url)
    return url
  }

  async function getAssessment(item: MediaItem): Promise<MediaArtifact | null> {
    const memCacheKey = item.path
    if (mediaStore.artifactsCache[memCacheKey]) {
      return mediaStore.artifactsCache[memCacheKey]
    }

    try {
      const diskData = await loadArtifactFromDisk(
        mediaStore.directoryPath,
        `${item.size}_${item.lastModified}`,
        'assessment',
      )
      if (diskData) {
        const parsed = JSON.parse(diskData) as MediaArtifact
        mediaStore.artifactsCache[memCacheKey] = parsed
        return parsed
      }
    } catch {}

    return null
  }

  async function profileAsset(item: MediaItem): Promise<MediaArtifact> {
    const existing = await getAssessment(item)
    if (existing) return existing

    isProfiling.value = true
    profilerError.value = null

    const targetModel = mediaStore.detectedModelName || 'qwen3.8-27b'

    try {
      const mediaUrl = await ensureUploadedUrl(item)
      const mediaPayload =
        item.mediaType === 'video'
          ? { type: 'video_url', video_url: { url: mediaUrl } }
          : { type: 'image_url', image_url: { url: mediaUrl } }

      const response = await ofetch<VlmCompletionResponse>(API_ENDPOINT, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', Accept: 'application/json' },
        body: {
          model: targetModel,
          temperature: 0.1,
          max_tokens: 1024,
          response_format: {
            type: 'json_schema',
            json_schema: {
              name: 'asset_assessment',
              strict: true,
              schema: ASSESSMENT_JSON_SCHEMA,
            },
          },
          messages: [
            {
              role: 'system',
              content: `You are an image quality and visual saliency profiler. Analyze the provided media asset and output valid JSON conforming strictly to the requested schema.`,
            },
            {
              role: 'user',
              content: [
                {
                  type: 'text',
                  text: 'Extract observation quality, subject orientation, and a concise visual summary.',
                },
                mediaPayload,
              ],
            },
          ],
        },
      })

      const raw = response.choices?.[0]?.message?.content || '{}'
      const assessment = parseJsonSafe<AssetAssessment>(raw)
      const fidelity = calculateFidelityScore(assessment)

      const artifact: MediaArtifact = {
        id: `${Date.now()}-${Math.random().toString(36).substring(2, 6)}`,
        fileHash: `${item.size}_${item.lastModified}`,
        sourcePath: item.path,
        mediaType: item.mediaType,
        createdAt: Date.now(),
        fidelityScore: fidelity,
        assessment,
        isUsable: assessment.subjectDetected,
      }

      mediaStore.artifactsCache[item.path] = artifact
      await saveArtifactToDisk(
        mediaStore.directoryPath,
        `${item.size}_${item.lastModified}`,
        'assessment',
        JSON.stringify(artifact),
      )

      return artifact
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err)
      console.error('[useMediaProfiler] Profiling failed:', msg)
      profilerError.value = msg
      throw err
    } finally {
      isProfiling.value = false
    }
  }

  async function arbitratePair(
    itemA: MediaItem,
    itemB: MediaItem,
    intentKey: string,
  ): Promise<ArbitrationVerdict> {
    const cacheKey = [itemA.path, itemB.path].sort().join(':::') + `:::${intentKey}`
    if (mediaStore.arbitrationCache[cacheKey]) {
      return mediaStore.arbitrationCache[cacheKey]
    }

    const targetModel = mediaStore.detectedModelName || 'qwen3.8-27b'
    const { systemPrompt, userInstruction } = buildArbitrationPrompt(intentKey)

    const urlA = await ensureUploadedUrl(itemA)
    const urlB = await ensureUploadedUrl(itemB)

    const payloadA =
      itemA.mediaType === 'video'
        ? { type: 'video_url', video_url: { url: urlA } }
        : { type: 'image_url', image_url: { url: urlA } }

    const payloadB =
      itemB.mediaType === 'video'
        ? { type: 'video_url', video_url: { url: urlB } }
        : { type: 'image_url', image_url: { url: urlB } }

    try {
      const response = await ofetch<VlmCompletionResponse>(API_ENDPOINT, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', Accept: 'application/json' },
        body: {
          model: targetModel,
          temperature: mediaStore.isThinkingEnabled ? 0.4 : 0.1,
          max_tokens: 1536,
          response_format: {
            type: 'json_schema',
            json_schema: {
              name: 'pairwise_arbitration',
              strict: true,
              schema: ARBITRATION_JSON_SCHEMA,
            },
          },
          messages: [
            { role: 'system', content: systemPrompt },
            {
              role: 'user',
              content: [
                {
                  type: 'text',
                  text: `${userInstruction}\n\nImage A is the first attached media. Image B is the second attached media.`,
                },
                payloadA,
                payloadB,
              ],
            },
          ],
        },
      })

      const raw = response.choices?.[0]?.message?.content || '{}'
      const verdict = parseJsonSafe<ArbitrationVerdict>(raw)

      // Ensure normalized confidence 0.0 to 1.0
      if (verdict.confidence > 1.0 && verdict.confidence <= 100.0) {
        verdict.confidence /= 100.0
      }
      verdict.confidence = Math.min(1.0, Math.max(0.0, verdict.confidence))

      mediaStore.arbitrationCache[cacheKey] = verdict
      return verdict
    } catch (err: unknown) {
      console.error('[useMediaProfiler] Dual-image arbitration failed:', err)
      return {
        isMatch: false,
        confidence: 0.0,
        verdictReason: 'Arbitration failed due to network or endpoint error.',
        alignedKeyFeatures: [],
        contradictoryFeatures: ['Arbitration request failed.'],
      }
    }
  }

  async function purgeArtifact(item: MediaItem): Promise<void> {
    delete mediaStore.artifactsCache[item.path]
    try {
      await deleteArtifactFromDisk(
        mediaStore.directoryPath,
        `${item.size}_${item.lastModified}`,
        'assessment',
      )
    } catch {}
  }

  async function purgeDirectoryCache(): Promise<void> {
    mediaStore.artifactsCache = {}
    mediaStore.arbitrationCache = {}
    uploadedUrls.clear()
    try {
      await clearDirectoryCache(mediaStore.directoryPath)
    } catch {}
  }

  return {
    isProfiling,
    profilerError,
    getAssessment,
    profileAsset,
    arbitratePair,
    purgeArtifact,
    purgeDirectoryCache,
  }
}
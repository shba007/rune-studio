import { ref } from 'vue'
import { ofetch } from 'ofetch'
import type { MediaItem, ComparisonVerdict } from '@/types/media'
import { useMediaScanner } from '@/composables/useMediaScanner'
import { useMediaStore } from '@/stores/mediaStore'

const API_ENDPOINT = import.meta.env.VITE_AI_API_URL || 'http://localhost:3423/v1/chat/completions'
const MODELS_ENDPOINT = API_ENDPOINT.replace(/\/chat\/completions\/?$/, '/models')
const CONFIGURED_MODEL = import.meta.env.VITE_AI_MODEL || 'ornith-1.5-9b'

const RED_PIXEL_PNG =
  'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg=='

interface ModelsResponse {
  data?: Array<{ id?: string; name?: string }>
}

interface ProbeResponse {
  choices?: Array<{
    message?: {
      content?: string
    }
  }>
}

interface ComparisonResponse {
  choices?: Array<{
    message?: {
      content?: string
      reasoning_content?: string
    }
    text?: string
  }>
}

function extractJsonAndThought(
  rawContent: string,
  reasoningContent?: string,
): { parsed: Record<string, unknown>; thought: string } {
  let thought = reasoningContent?.trim() || ''
  let cleaned = rawContent.trim()

  const thinkMatch = cleaned.match(/<think>([\s\S]*?)<\/think>/i)
  if (thinkMatch && thinkMatch[1]) {
    thought = thinkMatch[1].trim()
    cleaned = cleaned.replace(/<think>[\s\S]*?<\/think>/i, '').trim()
  } else if (cleaned.includes('</think>')) {
    const parts = cleaned.split('</think>')
    if (parts[0]) {
      thought = parts[0].replace(/<think>/i, '').trim()
      cleaned = parts.slice(1).join('</think>').trim()
    }
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

  try {
    const parsed = JSON.parse(cleaned) as Record<string, unknown>
    return { parsed, thought }
  } catch (err: unknown) {
    const msg = err instanceof Error ? err.message : String(err)
    console.error('[useMediaComparator] Failed to parse cleaned JSON:', cleaned)
    throw new Error(`Model output could not be parsed as JSON: ${msg}`)
  }
}

export function useMediaComparator() {
  const isComparing = ref(false)
  const error = ref<string | null>(null)
  const { getMediaBase64 } = useMediaScanner()
  const mediaStore = useMediaStore()

  async function checkVisionCapability(
    overrideModel?: string,
  ): Promise<{ isVision: boolean; modelName: string }> {
    mediaStore.isCheckingModel = true
    mediaStore.modelCheckMessage = 'Checking model capabilities...'

    if (overrideModel) {
      mediaStore.detectedModelName = overrideModel
    }

    try {
      const modelsData = await ofetch<ModelsResponse>(MODELS_ENDPOINT, {
        method: 'GET',
        timeout: 5000,
        retry: 0,
      })

      if (modelsData?.data && modelsData.data.length > 0) {
        const modelIds: string[] = modelsData.data
          .map((m) => m.id || m.name || '')
          .filter((name): name is string => Boolean(name))
        mediaStore.availableModels = modelIds

        if (mediaStore.detectedModelName && modelIds.includes(mediaStore.detectedModelName)) {
        } else if (CONFIGURED_MODEL && modelIds.includes(CONFIGURED_MODEL)) {
          mediaStore.detectedModelName = CONFIGURED_MODEL
        } else if (modelIds.length > 0 && modelIds[0]) {
          mediaStore.detectedModelName = modelIds[0]
        }
      } else if (!mediaStore.detectedModelName) {
        mediaStore.detectedModelName = CONFIGURED_MODEL
      }
    } catch {
      if (!mediaStore.detectedModelName) {
        mediaStore.detectedModelName = CONFIGURED_MODEL
      }
    }

    const activeModel = mediaStore.detectedModelName || CONFIGURED_MODEL

    const modelLower = activeModel.toLowerCase()
    const isKnownVision = /ornith|vl|vision|llava|pixtral|minicpm|paligemma|glm-4v/i.test(
      modelLower,
    )
    const isKnownNonVision = /needle|cactus|coder|deepseek-r1-distill|text-embedding/i.test(
      modelLower,
    )

    if (isKnownNonVision && !isKnownVision) {
      mediaStore.isVisionCapable = false
      mediaStore.modelCheckMessage = `The model "${activeModel}" is a text/tool-only model without vision support.`
      mediaStore.isCheckingModel = false
      return { isVision: false, modelName: activeModel }
    }

    if (isKnownVision) {
      mediaStore.isVisionCapable = true
      mediaStore.modelCheckMessage = `Vision verified for ${activeModel}`
      mediaStore.isCheckingModel = false
      return { isVision: true, modelName: activeModel }
    }

    try {
      const probeRes = await ofetch<ProbeResponse>(API_ENDPOINT, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: {
          model: activeModel,
          max_tokens: 15,
          messages: [
            {
              role: 'user',
              content: [
                {
                  type: 'text',
                  text: 'What is the color of this 1-pixel image? Answer in one word.',
                },
                { type: 'image_url', image_url: { url: RED_PIXEL_PNG } },
              ],
            },
          ],
        },
        timeout: 12000,
        retry: 0,
      })

      const reply = String(probeRes?.choices?.[0]?.message?.content || '').toLowerCase()
      const seesImage = reply.includes('red')

      mediaStore.isVisionCapable = seesImage
      mediaStore.modelCheckMessage = seesImage
        ? `Vision verified for ${activeModel}`
        : `Model responded, but failed visual verification (answered: "${reply.trim()}")`
      mediaStore.isCheckingModel = false
      return { isVision: seesImage, modelName: activeModel }
    } catch (probeErr: unknown) {
      const fetchErr = probeErr as {
        data?: { error?: { message?: string } }
        message?: string
        status?: number
      }
      const errMsg = String(fetchErr?.data?.error?.message || fetchErr?.message || '').toLowerCase()
      const isReject =
        errMsg.includes('image') ||
        errMsg.includes('vision') ||
        errMsg.includes('not support') ||
        fetchErr?.status === 400

      mediaStore.isVisionCapable = isReject ? false : null
      mediaStore.modelCheckMessage = isReject
        ? `Model "${activeModel}" rejected visual input.`
        : `Probe error: ${errMsg}`
      mediaStore.isCheckingModel = false
      return { isVision: false, modelName: activeModel }
    }
  }

  async function compressImageToDataUrl(
    imageUrl: string,
    maxDim = 1024,
    quality = 0.85,
  ): Promise<string> {
    return new Promise((resolve, reject) => {
      const img = new Image()
      img.crossOrigin = 'anonymous'

      const timer = setTimeout(() => {
        img.onload = null
        img.onerror = null
        reject(new Error('Image compression timed out'))
      }, 10000)

      img.onload = () => {
        clearTimeout(timer)
        let { width, height } = img

        if (width > maxDim || height > maxDim) {
          if (width > height) {
            height = Math.round((height * maxDim) / width)
            width = maxDim
          } else {
            width = Math.round((width * maxDim) / height)
            height = maxDim
          }
        }

        const canvas = document.createElement('canvas')
        canvas.width = width
        canvas.height = height
        const ctx = canvas.getContext('2d')
        if (!ctx) {
          reject(new Error('Canvas 2D context unavailable'))
          return
        }

        ctx.drawImage(img, 0, 0, width, height)
        resolve(canvas.toDataURL('image/jpeg', quality))
      }

      img.onerror = () => {
        clearTimeout(timer)
        reject(new Error(`Failed to load image for canvas optimization: ${imageUrl}`))
      }

      img.src = imageUrl
    })
  }

  async function extractVideoFrame(videoUrl: string, maxDim = 1024): Promise<string> {
    return new Promise((resolve, reject) => {
      const video = document.createElement('video')
      video.crossOrigin = 'anonymous'
      video.src = videoUrl
      video.muted = true

      const timer = setTimeout(() => {
        cleanup()
        reject(new Error('Timeout extracting video frame'))
      }, 8000)

      function cleanup() {
        clearTimeout(timer)
        video.onloadeddata = null
        video.onseeked = null
        video.onerror = null
      }

      video.onloadeddata = () => {
        video.currentTime = Math.min(1.0, (video.duration || 2) / 2)
      }

      video.onseeked = () => {
        try {
          let width = video.videoWidth || 640
          let height = video.videoHeight || 360

          if (width > maxDim || height > maxDim) {
            if (width > height) {
              height = Math.round((height * maxDim) / width)
              width = maxDim
            } else {
              width = Math.round((width * maxDim) / height)
              height = maxDim
            }
          }

          const canvas = document.createElement('canvas')
          canvas.width = width
          canvas.height = height
          const ctx = canvas.getContext('2d')
          if (!ctx) throw new Error('Could not get 2D canvas context')

          ctx.drawImage(video, 0, 0, width, height)
          const dataUrl = canvas.toDataURL('image/jpeg', 0.85)
          cleanup()
          resolve(dataUrl)
        } catch (e) {
          cleanup()
          reject(e)
        }
      }

      video.onerror = () => {
        cleanup()
        reject(new Error('Failed to load video element for frame capture'))
      }
    })
  }

  async function prepareMediaDataUrl(item: MediaItem): Promise<string> {
    if (item.mediaType === 'video' && item.thumbnailUrl) {
      try {
        return await extractVideoFrame(item.thumbnailUrl)
      } catch (err) {
        console.warn(
          '[useMediaComparator] Video frame extraction failed, falling back to base64:',
          err,
        )
      }
    }

    if (item.mediaType === 'image' && item.thumbnailUrl) {
      try {
        return await compressImageToDataUrl(item.thumbnailUrl)
      } catch (err) {
        console.warn(
          '[useMediaComparator] Image canvas optimization failed, falling back to raw base64:',
          err,
        )
      }
    }

    return await getMediaBase64(item.path)
  }

  async function compareMediaPair(
    itemA: MediaItem,
    itemB: MediaItem,
    instructionPrompt: string,
  ): Promise<ComparisonVerdict> {
    const targetModel = mediaStore.detectedModelName || CONFIGURED_MODEL

    if (mediaStore.isVisionCapable === false) {
      throw new Error(`The selected model "${targetModel}" does not support vision processing.`)
    }

    isComparing.value = true
    error.value = null

    const isLocalEndpoint = API_ENDPOINT.includes('localhost') || API_ENDPOINT.includes('127.0.0.1')

    try {
      const [mediaUrlA, mediaUrlB] = await Promise.all([
        isLocalEndpoint && itemA.path ? Promise.resolve(itemA.path) : prepareMediaDataUrl(itemA),
        isLocalEndpoint && itemB.path ? Promise.resolve(itemB.path) : prepareMediaDataUrl(itemB),
      ])

      const contentItemA =
        itemA.mediaType === 'video'
          ? { type: 'video_url', video_url: { url: mediaUrlA } }
          : { type: 'image_url', image_url: { url: mediaUrlA } }

      const contentItemB =
        itemB.mediaType === 'video'
          ? { type: 'video_url', video_url: { url: mediaUrlB } }
          : { type: 'image_url', image_url: { url: mediaUrlB } }

      const systemPrompt = `You are a visual comparison AI inspecting two media items: Item A (${itemA.mediaType}) and Item B (${itemB.mediaType}).

MANDATORY RULES:
1. Base your evaluation EXCLUSIVELY on the visible pixels, objects, geometry, and visual content inside Item A and Item B.
2. Do NOT speculate about file names, camera sequences, or metadata.
3. In your JSON response, you MUST first explicitly state what is visually depicted in Item A and Item B under visualSummaryA and visualSummaryB.
4. Output valid JSON adhering to this schema:
{
  "visualSummaryA": string,
  "visualSummaryB": string,
  "isMatch": boolean,
  "confidence": number,
  "similarityType": string,
  "explanation": string,
  "recommendedPrimary": "item_a" | "item_b"
}`

      const userTextPrompt = `Goal: ${instructionPrompt || 'Visually compare Item A and Item B for similarity.'}
Inspect the visual content of the two attached media items and output the JSON evaluation.`

      const requestBody = {
        model: targetModel,
        temperature: 0.1,
        messages: [
          { role: 'system', content: systemPrompt },
          {
            role: 'user',
            content: [{ type: 'text', text: userTextPrompt }, contentItemA, contentItemB],
          },
        ],
      }

      const data = await ofetch<ComparisonResponse>(API_ENDPOINT, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: requestBody,
        timeout: 600000,
        retry: 0,
      })

      const message = data.choices?.[0]?.message
      const rawContent = message?.content || data.choices?.[0]?.text || ''
      const reasoningContent = message?.reasoning_content || ''

      if (!rawContent && !reasoningContent) {
        throw new Error(`Empty response received from visual comparison model "${targetModel}".`)
      }

      const { parsed, thought } = extractJsonAndThought(rawContent, reasoningContent)

      let confidence = typeof parsed.confidence === 'number' ? parsed.confidence : 0.5
      if (confidence > 1.0 && confidence <= 100.0) {
        confidence = confidence / 100.0
      }

      const verdict: ComparisonVerdict = {
        isMatch: Boolean(parsed.isMatch),
        confidence: Math.min(Math.max(confidence, 0.0), 1.0),
        similarityType:
          typeof parsed.similarityType === 'string' ? parsed.similarityType : 'comparison',
        explanation:
          typeof parsed.explanation === 'string' ? parsed.explanation : 'No explanation provided.',
        visualSummaryA:
          typeof parsed.visualSummaryA === 'string' ? parsed.visualSummaryA : undefined,
        visualSummaryB:
          typeof parsed.visualSummaryB === 'string' ? parsed.visualSummaryB : undefined,
        thought: thought || undefined,
        recommendedPrimary: parsed.recommendedPrimary === 'item_b' ? 'item_b' : 'item_a',
      }

      return verdict
    } catch (err: unknown) {
      const fetchErr = err as { data?: { error?: { message?: string } }; message?: string }
      const msg =
        fetchErr?.data?.error?.message || (err instanceof Error ? err.message : String(err))
      console.error('[useMediaComparator] Execution error:', msg)
      error.value = msg
      throw err
    } finally {
      isComparing.value = false
    }
  }

  return {
    isComparing,
    error,
    checkVisionCapability,
    compareMediaPair,
    extractVideoFrame,
  }
}
import { ref } from 'vue'
import { ofetch } from 'ofetch'
import type { InferenceTaskRequest } from '@/types/inference'
import { useMediaStore } from '@/stores/mediaStore'

const BASE_API = import.meta.env.VITE_AI_API_URL
const CHAT_ENDPOINT = `${BASE_API}/v1/chat/completions`
const DIRECT_INFERENCE_ENDPOINT = `${BASE_API}/v1/inference`

interface ChatCompletionResponse {
  choices?: Array<{
    message?: {
      role?: string
      content?: string
      tool_calls?: Array<{
        id?: string
        type: 'function'
        function: {
          name: string
          arguments: string | Record<string, unknown>
        }
      }>
    }
  }>
}

interface DirectTaskResponse {
  name: string
  arguments: Record<string, unknown> | string
}

export function useInference() {
  const isExecuting = ref(false)
  const lastRawResponse = ref<string>('')
  const errorMessage = ref<string>('')
  const mediaStore = useMediaStore()

  async function executeTask<T = unknown>(task: InferenceTaskRequest): Promise<T> {
    isExecuting.value = true
    errorMessage.value = ''

    const activeModel = mediaStore.detectedModelName || 'qwen3.8-27b'

    try {
      // 1. Direct Engine Route (Section 8 of API reference: /v1/inference?engine=...)
      if (task.type === 'ToolCall') {
        const { prompt, schema } = task.payload
        try {
          const directData = await ofetch<DirectTaskResponse>(
            `${DIRECT_INFERENCE_ENDPOINT}?engine=${encodeURIComponent(activeModel)}`,
            {
              method: 'POST',
              headers: { 'Content-Type': 'application/json' },
              body: {
                type: 'tool_call',
                prompt,
                schema: schema[0] || {
                  type: 'object',
                  properties: {},
                },
              },
            },
          )

          lastRawResponse.value = JSON.stringify(directData)
          const args =
            typeof directData.arguments === 'string'
              ? JSON.parse(directData.arguments)
              : directData.arguments || {}
          return { name: directData.name, arguments: args } as T
        } catch {
          // Fallback to OpenAI-compatible /v1/chat/completions tool call (Section 4)
        }

        const data = await ofetch<ChatCompletionResponse>(CHAT_ENDPOINT, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: {
            model: activeModel,
            messages: [{ role: 'user', content: prompt }],
            tools: schema.map((tool) => ({
              type: 'function',
              function: {
                name: tool.name,
                description: tool.description,
                parameters: tool.parameters,
              },
            })),
            tool_choice: 'auto',
            temperature: 0.1,
          },
        })

        lastRawResponse.value = JSON.stringify(data)
        const toolCalls = data.choices?.[0]?.message?.tool_calls

        if (toolCalls && toolCalls.length > 0 && toolCalls[0]) {
          const call = toolCalls[0]
          return {
            name: call.function.name,
            arguments:
              typeof call.function.arguments === 'string'
                ? JSON.parse(call.function.arguments)
                : call.function.arguments,
          } as T
        }

        throw new Error('Endpoint did not return standard tool_calls object')
      }

      if (task.type === 'Chat') {
        const data = await ofetch<ChatCompletionResponse>(CHAT_ENDPOINT, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: {
            model: activeModel,
            messages: task.payload.messages,
          },
        })
        lastRawResponse.value = JSON.stringify(data)
        return data as T
      }

      throw new Error(`Task type "${task.type}" is not supported on this endpoint.`)
    } catch (err: unknown) {
      const fetchErr = err as { data?: { error?: { message?: string } }; message?: string }
      const msg =
        fetchErr?.data?.error?.message || (err instanceof Error ? err.message : String(err))
      console.error('[useInference] Execution failed:', msg)
      errorMessage.value = msg
      throw err
    } finally {
      isExecuting.value = false
    }
  }

  return {
    isExecuting,
    lastRawResponse,
    errorMessage,
    executeTask,
  }
}
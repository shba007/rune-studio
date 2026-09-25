import { ref } from 'vue'
import { ofetch } from 'ofetch'
import type { InferenceTaskRequest } from '@/types/inference'
import { useMediaStore } from '@/stores/mediaStore'

const API_ENDPOINT = import.meta.env.VITE_AI_API_URL || 'http://localhost:3423/v1/chat/completions'

interface ChatCompletionResponse {
  choices?: Array<{
    message?: {
      role?: string
      content?: string
      tool_calls?: Array<{
        function: {
          name: string
          arguments: string | Record<string, unknown>
        }
      }>
    }
  }>
}

function stripThinkingTrace(text: string): string {
  let cleaned = text.replace(/<think>[\s\S]*?<\/think>/gi, '').trim()
  if (cleaned.includes('</think>')) {
    cleaned = cleaned.split('</think>').pop()?.trim() || ''
  }
  return cleaned
}

export function useInference() {
  const isExecuting = ref(false)
  const lastRawResponse = ref<string>('')
  const errorMessage = ref<string>('')
  const mediaStore = useMediaStore()

  async function executeTask<T = unknown>(task: InferenceTaskRequest): Promise<T> {
    isExecuting.value = true
    errorMessage.value = ''

    const activeModel = mediaStore.detectedModelName || 'default'

    try {
      let body: Record<string, unknown>

      if (task.type === 'ToolCall') {
        const { prompt, schema } = task.payload
        body = {
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
        }
      } else if (task.type === 'Chat') {
        body = {
          model: activeModel,
          messages: task.payload.messages,
        }
      } else {
        throw new Error(`Task type "${task.type}" is not supported on this endpoint.`)
      }

      const data = await ofetch<ChatCompletionResponse>(API_ENDPOINT, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body,
        timeout: 60000,
        retry: 0,
      })

      lastRawResponse.value = JSON.stringify(data)

      if (task.type === 'ToolCall') {
        const message = data.choices?.[0]?.message
        const toolCalls = message?.tool_calls

        if (toolCalls && toolCalls.length > 0 && toolCalls[0]) {
          return {
            name: toolCalls[0].function.name,
            arguments:
              typeof toolCalls[0].function.arguments === 'string'
                ? JSON.parse(toolCalls[0].function.arguments)
                : toolCalls[0].function.arguments,
          } as T
        }

        const rawContent = stripThinkingTrace(message?.content || '')
        const toolMatch = rawContent.match(/<tool_call>([\s\S]*?)<\/tool_call>/i)
        const jsonTarget = toolMatch && toolMatch[1] ? toolMatch[1].trim() : rawContent

        const firstBrace = jsonTarget.indexOf('{')
        const firstBracket = jsonTarget.indexOf('[')
        const startIdx =
          firstBracket !== -1 && (firstBrace === -1 || firstBracket < firstBrace)
            ? firstBracket
            : firstBrace

        if (startIdx !== -1) {
          const parsed = JSON.parse(jsonTarget.substring(startIdx)) as
            | { name: string; arguments?: string | Record<string, unknown> }
            | Array<{ name: string; arguments?: string | Record<string, unknown> }>
          const item = Array.isArray(parsed) ? parsed[0] : parsed
          if (item) {
            return {
              name: item.name,
              arguments:
                typeof item.arguments === 'string'
                  ? JSON.parse(item.arguments)
                  : item.arguments || {},
            } as T
          }
        }

        throw new Error(rawContent || 'Model did not trigger any tool call.')
      }

      return data as T
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
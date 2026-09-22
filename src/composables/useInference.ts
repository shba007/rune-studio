import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { InferenceTaskRequest, InferenceTaskResponse } from '@/types/inference'

export function useInference() {
  const isExecuting = ref(false)
  const lastRawResponse = ref<string>('')
  const errorMessage = ref<string>('')

  async function executeTask<T = any>(
    task: InferenceTaskRequest,
    engineId?: string,
  ): Promise<T> {
    isExecuting.value = true
    errorMessage.value = ''

    try {
      const response = await invoke<InferenceTaskResponse<T>>('execute_inference_task', {
        engineId,
        task,
      })

      if (response.status === 'Error') {
        throw new Error(String(response.data))
      }

      lastRawResponse.value = JSON.stringify(response.data)
      return response.data
    } catch (err: any) {
      const msg = err?.message || String(err)
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
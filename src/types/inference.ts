export type TaskType = 'ToolCall' | 'Chat' | 'GenerateImage' | 'GenerateAudio'

export interface ChatMessage {
  role: 'system' | 'user' | 'assistant'
  content: string
}

export type InferenceTaskRequest =
  | {
      type: 'ToolCall'
      payload: {
        prompt: string
        schema: Record<string, unknown>[]
      }
    }
  | {
      type: 'Chat'
      payload: {
        messages: ChatMessage[]
      }
    }
  | {
      type: 'GenerateImage'
      payload: {
        prompt: string
        width?: number
        height?: number
      }
    }
  | {
      type: 'GenerateAudio'
      payload: {
        text: string
        voice_id?: string
      }
    }

export interface InferenceTaskResponse<T = unknown> {
  status: 'ToolCall' | 'Text' | 'ImageBase64' | 'AudioBuffer' | 'Error'
  data: T
}
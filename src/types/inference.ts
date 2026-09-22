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
        schema: Record<string, any>[]
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

export interface InferenceTaskResponse<T = any> {
  status: 'ToolCall' | 'Text' | 'ImageBase64' | 'AudioBuffer' | 'Error'
  data: T
}
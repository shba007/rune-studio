import type { IconName } from '@/icons'

export type ActionKind =
  | 'mouse'
  | 'keyboard'
  | 'screen'
  | 'video'
  | 'mic'
  | 'audio'
  | 'launch'
  | 'wait'

export const KIND_ICON: Record<ActionKind, IconName> = {
  mouse: 'mouse',
  keyboard: 'keyboard',
  screen: 'monitor',
  video: 'video',
  mic: 'mic',
  audio: 'headphones',
  launch: 'app',
  wait: 'wait',
}

export const TOOL_KIND: Record<string, ActionKind> = {
  click: 'mouse',
  click_element: 'mouse',
  scroll: 'mouse',
  drag: 'mouse',
  type_text: 'keyboard',
  press_keys: 'keyboard',
  observe: 'screen',
  screenshot: 'screen',
  record_screen: 'video',
  start_recording: 'video',
  stop_recording: 'video',
  list_capture_targets: 'screen',
  listen: 'mic',
  transcribe: 'audio',
  play_audio: 'audio',
  open_app: 'launch',
  wait: 'wait',
  ask_user: 'wait',
}

export const TOOL_LABEL: Record<string, string> = {
  click: 'Click',
  click_element: 'Click',
  scroll: 'Scroll',
  drag: 'Drag',
  type_text: 'Type',
  press_keys: 'Keys',
  observe: 'Look',
  screenshot: 'Capture',
  record_screen: 'Record',
  start_recording: 'Start recording',
  stop_recording: 'Save recording',
  list_capture_targets: 'Targets',
  listen: 'Listen',
  transcribe: 'Transcribe',
  play_audio: 'Play',
  open_app: 'Open',
  wait: 'Wait',
  ask_user: 'Ask',
}

export function kindOf(tool: string): ActionKind {
  return TOOL_KIND[tool] ?? 'mouse'
}
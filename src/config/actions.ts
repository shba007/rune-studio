import type { IconName } from '@/icons'

/**
 * What an agent action means, so the control overlay shows the same icon for the same kind of
 * thing. The icon is the *device* involved, not the individual tool:
 *
 * - mouse: anything with the cursor (click, scroll, drag)
 * - keyboard: anything typed or pressed
 * - screen: looking at the screen (a screenshot, flashed over the captured area)
 * - video: recording the screen
 * - mic: listening through the microphone
 * - audio: headphones, for recording or playing sound
 *
 * A new tool only needs a line in TOOL_KIND.
 */
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
  listen: 'Listen',
  transcribe: 'Transcribe',
  play_audio: 'Play',
  open_app: 'Open',
  wait: 'Wait',
  ask_user: 'Ask',
}

/** The device a tool uses, falling back to the mouse for tools that are not listed yet. */
export function kindOf(tool: string): ActionKind {
  return TOOL_KIND[tool] ?? 'mouse'
}
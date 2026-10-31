import type { IconName } from '@/icons'
import type { AnalysisSnapshot } from '@/types/protocol'

export interface PlaygroundPipeline {
  id: string
  label: string
  icon: IconName
  ready: boolean
  summary: string
}

export interface PlaygroundGroup {
  id: string
  label: string
  pipelines: PlaygroundPipeline[]
}

const later = (id: string, label: string, icon: IconName, summary: string): PlaygroundPipeline => ({
  id,
  label,
  icon,
  summary,
  ready: false,
})

export const PLAYGROUND_GROUPS: PlaygroundGroup[] = [
  {
    id: 'images',
    label: 'Images',
    pipelines: [
      {
        id: 'ask',
        label: 'Ask',
        icon: 'ask',
        ready: true,
        summary: 'Ask a question about a screen, a window or an image.',
      },
      {
        id: 'point',
        label: 'Point',
        icon: 'target',
        ready: true,
        summary: 'Find where something is and mark it on the image.',
      },
      {
        id: 'pose',
        label: 'Pose',
        icon: 'pose',
        ready: true,
        summary: 'Mark joints and landmarks for pose questions.',
      },
      {
        id: 'depth',
        label: 'Depth',
        icon: 'depth',
        ready: true,
        summary: 'Estimate relative depth with a dedicated depth model.',
      },
      later('create', 'Create', 'image', 'Generate a new image from a prompt.'),
      later('edit', 'Edit', 'edit', 'Change an image with an instruction.'),
      later('transform', 'Transform', 'transform', 'Redraw an image from your prompt.'),
      later('inpaint', 'Inpaint', 'inpaint', 'Regenerate a painted region.'),
      later('extend', 'Extend', 'extend', 'Grow the canvas and fill the edges.'),
      later('upscale', 'Upscale', 'upscale', 'Enlarge and re-detail an image.'),
      later('reference', 'Reference', 'reference', 'Generate guided by a reference image.'),
    ],
  },
  {
    id: 'audio',
    label: 'Audio',
    pipelines: [
      later('tts', 'Text to Speech', 'speaker', 'Turn text into speech with a built-in voice.'),
      later('clone', 'Clone', 'copy', 'Speak in the voice from a short recording.'),
      later('audio-edit', 'Edit', 'edit', 'Change words in a recording, same voice.'),
      later('convert', 'Convert', 'convert', 'Make a recording sound like another voice.'),
      later('music', 'Music', 'music', 'Songs, instrumentals and sound effects.'),
      later('separate', 'Separate', 'separate', 'Split a track into vocals and instruments.'),
      later('transcribe', 'Transcribe', 'captions', 'Turn speech into text.'),
    ],
  },
  {
    id: 'video',
    label: 'Video',
    pipelines: [later('video-create', 'Create', 'film', 'Generate a video from a prompt.')],
  },
]

export const DEFAULT_PIPELINE = 'ask'

export const findPipeline = (id: string) =>
  PLAYGROUND_GROUPS.flatMap((group) => group.pipelines).find((item) => item.id === id)

export type PlaygroundSource = 'screen' | 'window' | 'file'

export const SOURCE_OPTIONS: { value: PlaygroundSource; label: string }[] = [
  { value: 'screen', label: 'Screen' },
  { value: 'window', label: 'Window' },
  { value: 'file', label: 'File' },
]

export const QUESTION_HINTS: Record<string, string[]> = {
  ask: ['Summarise what is on screen', 'List the buttons you can see', 'Read the error message'],
  point: ['Where is the Save button?', 'Find the search field', 'Locate the close icon'],
  pose: ['Show the pose of the person', 'Mark the hands and elbows', 'Mark the face landmarks'],
}

export const DEPTH_PIPELINE = 'depth'
export const DEPTH_TASK = 'image-to-depth'

export interface DepthResult {
  original: string
  preview: string | null
  width: number
  height: number
  min: number
  max: number
  model: string
  elapsedMs: number
}

export interface DepthState {
  status: 'idle' | 'running' | 'done' | 'error'
  result: DepthResult | null
  error: string
}

export const IDLE_DEPTH: DepthState = { status: 'idle', result: null, error: '' }

export const depthModelIds = (models: { id: string; tasks?: string[] }[]) =>
  models.filter((model) => model.tasks?.includes(DEPTH_TASK)).map((model) => model.id)

export const RESOLUTION_OPTIONS = [
  { value: '720', label: '720 px' },
  { value: '1280', label: '1280 px' },
  { value: '1920', label: '1920 px' },
  { value: '2560', label: '2560 px' },
  { value: 'native', label: 'Native' },
]

const QUESTION_CAP = 500
const SHAPES: Record<string, { prefix: string; fallback: string }> = {
  point: {
    prefix: 'Locate: ',
    fallback: 'the main subject',
  },
  pose: {
    prefix: 'Pose: ',
    fallback: 'each visible person',
  },
}
const SHAPE_SUFFIX: Record<string, string> = {
  point: '. Give a box and a point at the centre for each match.',
  pose: '. Return a structure finding with named keypoints and connections for each.',
}

export function shapeQuestion(pipeline: string, text: string) {
  const shape = SHAPES[pipeline]
  if (!shape) return text.trim()
  const body = text.trim() || shape.fallback
  return `${shape.prefix}${body}${SHAPE_SUFFIX[pipeline]}`.slice(0, QUESTION_CAP)
}

export function unshapeQuestion(pipeline: string, shaped: string) {
  const shape = SHAPES[pipeline]
  const suffix = SHAPE_SUFFIX[pipeline]
  if (!shape || !suffix || !shaped.startsWith(shape.prefix) || !shaped.endsWith(suffix))
    return shaped
  const body = shaped.slice(shape.prefix.length, shaped.length - suffix.length)
  return body === shape.fallback ? '' : body
}

export const questionLimit = (pipeline: string) => (SHAPES[pipeline] ? 400 : QUESTION_CAP)

export function filterFindings(pipeline: string, snapshot: AnalysisSnapshot): AnalysisSnapshot {
  if (pipeline === 'point')
    return {
      ...snapshot,
      findings: snapshot.findings.filter((f) => f.box !== null || f.point !== null),
    }
  if (pipeline === 'pose')
    return {
      ...snapshot,
      findings: snapshot.findings.filter((f) => f.group === 'structure' || f.pose),
    }
  return snapshot
}
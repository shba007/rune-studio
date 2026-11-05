import type { IconName } from '@/icons'
import type { AnalysisSnapshot } from '@/types/protocol'
import type { AudioOperation, AudioOperationSupport } from '@/types/protocol'

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
        id: 'caption',
        label: 'Caption',
        icon: 'captions',
        ready: true,
        summary: 'Describe a screen, window or image.',
      },
      {
        id: 'point',
        label: 'Point',
        icon: 'target',
        ready: true,
        summary: 'Find where something is and mark it on the image.',
      },
      {
        id: 'detect',
        label: 'Detect objects',
        icon: 'scan',
        ready: true,
        summary: 'Detect objects and their locations in an image.',
      },
      {
        id: 'instance-segment',
        label: 'Segment objects',
        icon: 'scan',
        ready: true,
        summary: 'Detect individual objects and their masks.',
      },
      {
        id: 'oriented-detect',
        label: 'Detect oriented objects',
        icon: 'scan',
        ready: true,
        summary: 'Detect rotated objects with oriented bounding boxes.',
      },
      {
        id: 'segment',
        label: 'Segment scene',
        icon: 'scan',
        ready: true,
        summary: 'Identify semantic regions in an image.',
      },
      {
        id: 'pose',
        label: 'Pose',
        icon: 'pose',
        ready: true,
        summary: 'Detect people and estimate their pose keypoints.',
      },
      {
        id: 'depth',
        label: 'Depth',
        icon: 'depth',
        ready: true,
        summary: 'Estimate relative depth with a dedicated depth model.',
      },
      {
        id: 'develop-raw',
        label: 'Develop RAW',
        icon: 'image',
        ready: true,
        summary: 'Develop a camera-original ARW into a 16-bit PNG.',
      },
      {
        id: 'create',
        label: 'Create',
        icon: 'image',
        ready: true,
        summary: 'Generate a new image from a prompt.',
      },
      {
        id: 'edit',
        label: 'Edit',
        icon: 'edit',
        ready: true,
        summary: 'Change an image with an instruction.',
      },
      {
        id: 'variation',
        label: 'Variations',
        icon: 'image',
        ready: true,
        summary: 'Create a related variation of an image.',
      },
      later('transform', 'Transform', 'transform', 'Redraw an image from your prompt.'),
      later('inpaint', 'Inpaint', 'inpaint', 'Regenerate a painted region.'),
      later('extend', 'Extend', 'extend', 'Grow the canvas and fill the edges.'),
      {
        id: 'upscale',
        label: 'Upscale',
        icon: 'upscale',
        ready: true,
        summary: 'Enlarge an image 2× or 4× with an upscale model.',
      },
      {
        id: 'reference',
        label: 'Reference',
        icon: 'reference',
        ready: true,
        summary: 'Generate using two images and an instruction.',
      },
    ],
  },
  {
    id: 'audio',
    label: 'Audio',
    pipelines: [
      {
        id: 'tts',
        label: 'Create Voice',
        icon: 'speaker',
        ready: true,
        summary: 'Turn text into speech using the model’s default voice.',
      },
      later('clone', 'Clone', 'copy', 'Speak in the voice from a short recording.'),
      later('audio-edit', 'Edit', 'edit', 'Change words in a recording, same voice.'),
      later('convert', 'Convert', 'convert', 'Make a recording sound like another voice.'),
      later('music', 'Music', 'music', 'Songs, instrumentals and sound effects.'),
      later('separate', 'Separate', 'separate', 'Split a track into vocals and instruments.'),
      {
        id: 'transcribe',
        label: 'Transcribe',
        icon: 'captions',
        ready: true,
        summary: 'Turn speech from a WAV file into text.',
      },
      {
        id: 'translate-speech',
        label: 'Translate speech',
        icon: 'captions',
        ready: true,
        summary: 'Translate speech from a WAV file into text.',
      },
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

export type VisualOperation = 'detect' | 'segment' | 'pose' | 'instance-segment' | 'oriented-detect'
export const VISUAL_ROLES: Record<
  VisualOperation,
  'entities' | 'regions' | 'structure' | 'instances' | 'oriented'
> = {
  detect: 'entities',
  'instance-segment': 'instances',
  'oriented-detect': 'oriented',
  segment: 'regions',
  pose: 'structure',
}
export interface VisualResult {
  model: string
  data?: {
    label: string
    confidence: number
    box?: { x: number; y: number; width: number; height: number }
    mask?: string
    oriented_box?: { corners_pixels: [number, number][]; angle_radians: number }
    pose?: { name: string; x: number; y: number; confidence: number }[]
  }[]
  class_map?: string
  classes?: { id: number; label: string }[]
  usage?: { image_width: number; image_height: number }
}
export interface VisualOperationResult {
  image: string
  result: VisualResult
}

export const SOURCE_OPTIONS: { value: PlaygroundSource; label: string }[] = [
  { value: 'screen', label: 'Screen' },
  { value: 'window', label: 'Window' },
  { value: 'file', label: 'File' },
]

export const QUESTION_HINTS: Record<string, string[]> = {
  tts: [
    'Hello! Welcome to Rune Studio.',
    'Every great idea starts with a little curiosity.',
    'Take a deep breath and enjoy this moment.',
  ],
  ask: ['Summarise what is on screen', 'List the buttons you can see', 'Read the error message'],
  point: ['Where is the Save button?', 'Find the search field', 'Locate the close icon'],
  pose: ['Show the pose of the person', 'Mark the hands and elbows', 'Mark the face landmarks'],
  create: [
    'A lighthouse on a cliff at sunset, oil painting',
    'A small robot watering plants, soft studio light',
    'A quiet street in the rain, neon reflections',
  ],
  edit: [
    'Change the background to a sunny beach',
    'Make the lighting warmer',
    'Turn this into a watercolor painting',
  ],
  reference: [
    'Replace the person in image 2 with the person from image 1. Preserve the face, hairstyle and body shape from image 1, and the pose, clothes and setting from image 2. Match the scene lighting, shadows and perspective, with natural skin texture and realistic details.',
    'Apply the colors and style of image 2 to image 1.',
    'Combine the main subjects of both images in one scene.',
  ],
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

export interface CaptionResult {
  image: string
  caption: string
  model: string
  elapsedMs: number
}

export interface UpscaleResult {
  original: string
  output: string
  inputWidth: number
  inputHeight: number
  outputWidth: number
  outputHeight: number
  scale: number
  model: string
  elapsedMs: number
}

export interface CreateResult {
  image: string
  prompt: string
  size: string
  model: string
  elapsedMs: number
}

export interface EditResult {
  original: string
  image: string
  prompt: string
  model: string
  elapsedMs: number
}

export interface SpeechResult {
  audio: string
  text: string
  model: string
  elapsedMs: number
}

export const speechTextBytes = (text: string) => new TextEncoder().encode(text).length

export const CREATE_RESOLUTIONS: { value: string; label: string; icon: IconName }[] = [
  { value: '512', label: 'SD (512p)', icon: 'quality-low' },
  { value: '720', label: 'HD (720p)', icon: 'quality-medium' },
  { value: '1080', label: 'Full HD (1080p)', icon: 'quality-high' },
]

export const CREATE_ASPECTS: {
  value: string
  label: string
  icon: IconName
  iconClass?: string
}[] = [
  { value: '1:1', label: 'Square (1:1)', icon: 'square' },
  { value: '4:3', label: 'Landscape (4:3)', icon: 'rectangle' },
  { value: '16:9', label: 'Widescreen (16:9)', icon: 'monitor' },
  { value: '3:4', label: 'Portrait (3:4)', icon: 'rectangle', iconClass: 'rotate-90' },
  { value: '9:16', label: 'Tall (9:16)', icon: 'phone' },
]

export function createSize(resolution: string, aspect: string) {
  const [w = 1, h = 1] = aspect.split(':').map(Number)
  const short = Number(resolution)
  const long = Math.round((short * Math.max(w, h)) / Math.min(w, h) / 8) * 8
  return w >= h ? `${long}x${short}` : `${short}x${long}`
}

export interface RunState<T> {
  status: 'idle' | 'running' | 'done' | 'error'
  result: T | null
  error: string
}

export type DepthState = RunState<DepthResult>
export type CaptionState = RunState<CaptionResult>
export type UpscaleState = RunState<UpscaleResult>
export type CreateState = RunState<CreateResult>
export type EditState = RunState<EditResult>
export type SpeechState = RunState<SpeechResult>
export type TranscribeState = RunState<{ text: string; model: string; elapsedMs: number }>

export const idleRun = <T>(): RunState<T> => ({ status: 'idle', result: null, error: '' })

export interface ModelPipeline {
  task: string
  audioOperation?: AudioOperation
  command: string
  modelLabel: string
  none: string
  idle: string
}

export const MODEL_PIPELINES: Record<string, ModelPipeline> = {
  'instance-segment': {
    task: 'image-to-detection',
    command: 'playground_visual_branch',
    modelLabel: 'Instance segmentation model',
    none: 'No instance segmentation model',
    idle: 'Choose a source and an instance segmentation model, then press Run.',
  },
  'oriented-detect': {
    task: 'image-to-detection',
    command: 'playground_visual_branch',
    modelLabel: 'Oriented detection model',
    none: 'No oriented detection model',
    idle: 'Choose a source and an oriented detection model, then press Run.',
  },
  detect: {
    task: 'image-to-detection',
    command: 'playground_visual_branch',
    modelLabel: 'Detection model',
    none: 'No detection model',
    idle: 'Choose a source and detection model, then press Run.',
  },
  segment: {
    task: 'image-to-segmentation',
    command: 'playground_visual_branch',
    modelLabel: 'Segmentation model',
    none: 'No segmentation model',
    idle: 'Choose a source and segmentation model, then press Run.',
  },
  pose: {
    task: 'image-to-detection',
    command: 'playground_visual_branch',
    modelLabel: 'Pose model',
    none: 'No pose model',
    idle: 'Choose a source and pose model, then press Run.',
  },
  reference: {
    task: 'text-image-to-image',
    command: 'playground_reference',
    modelLabel: 'Reference model',
    none: 'No image edit model',
    idle: 'Choose two images, describe how to use them and press Run.',
  },
  variation: {
    task: 'text-image-to-image',
    command: 'playground_variation',
    modelLabel: 'Variation model',
    none: 'No image variation model',
    idle: 'Choose an image and a model, then press Run.',
  },
  edit: {
    task: 'text-image-to-image',
    command: 'playground_edit',
    modelLabel: 'Edit model',
    none: 'No image edit model',
    idle: 'Choose an image, describe a change and press Run.',
  },
  caption: {
    task: 'image-to-text',
    command: 'playground_caption',
    modelLabel: 'Caption model',
    none: 'No image-to-text model',
    idle: 'Choose a source and a vision model, then press Run.',
  },
  'translate-speech': {
    task: 'audio-to-text',
    audioOperation: 'speechTranslation',
    command: 'playground_translate_speech',
    modelLabel: 'Translation model',
    none: 'No verified translation model',
    idle: 'Choose a WAV file and a translation-capable model, then press Run.',
  },
  transcribe: {
    task: 'audio-to-text',
    audioOperation: 'transcription',
    command: 'playground_transcribe',
    modelLabel: 'Transcription model',
    none: 'No verified transcription model',
    idle: 'Choose a WAV file and a transcription model, then press Run.',
  },
  tts: {
    task: 'text-to-audio',
    audioOperation: 'speechGeneration',
    command: 'playground_speech',
    modelLabel: 'Voice model',
    none: 'No verified voice model',
    idle: 'Enter text and choose a voice model, then press Run.',
  },
  depth: {
    task: DEPTH_TASK,
    command: 'playground_depth',
    modelLabel: 'Depth model',
    none: 'No depth model',
    idle: 'Pick a source and a depth model, then press Run to see the depth map.',
  },
  create: {
    task: 'text-to-image',
    command: 'playground_create',
    modelLabel: 'Image model',
    none: 'No image model',
    idle: 'Describe an image, pick a size and press Run to create it.',
  },
  upscale: {
    task: 'image-upscale',
    command: 'playground_upscale',
    modelLabel: 'Upscale model',
    none: 'No upscale model',
    idle: 'Pick a source, a model and a scale, then press Run to see the upscaled image.',
  },
}

export const SCALE_OPTIONS = [
  { value: 2, label: '2×' },
  { value: 4, label: '4×' },
]

function matchesTask(candidate: string, task: string): boolean {
  if (task !== 'image-to-text') return candidate === task
  const parts = candidate.split('-to-')
  return parts.length === 2 && parts[1] === 'text' && !!parts[0]?.split('-').includes('image')
}

export const taskModelIds = (
  models: {
    id: string
    tasks?: string[]
    audioOperations?: Partial<Record<AudioOperation, AudioOperationSupport>>
  }[],
  task: string,
  operation?: AudioOperation,
) =>
  models
    .filter(
      (model) =>
        model.tasks?.some((candidate) => matchesTask(candidate, task)) &&
        (!operation || model.audioOperations?.[operation]?.status === 'supported'),
    )
    .map((model) => model.id)

export function visualOperationModels(
  models: { id: string; tasks?: string[]; rawCapabilities?: string[] }[],
  operation: VisualOperation,
) {
  const ids = taskModelIds(models, MODEL_PIPELINES[operation]!.task)
  return ids.filter((id) => {
    const capabilities = models.find((model) => model.id === id)?.rawCapabilities ?? []
    const has = (name: string) =>
      capabilities.some((capability) => capability.toLowerCase() === name)
    if (operation === 'pose') return has('pose estimation')
    if (operation === 'instance-segment') return has('instance segmentation')
    if (operation === 'oriented-detect') return has('oriented bounding boxes')
    if (operation === 'detect')
      return (
        !has('pose estimation') && !has('instance segmentation') && !has('oriented bounding boxes')
      )
    return true
  })
}

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
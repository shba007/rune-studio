import { EMPTY_SNAPSHOT } from '@/config/analysis'
import type { VisualResult } from '@/config/playground'
import type { AnalysisSnapshot, Box, Finding, Pose } from '@/types/protocol'

const poseLinks = [
  ['nose', 'left_eye'],
  ['nose', 'right_eye'],
  ['left_eye', 'left_ear'],
  ['right_eye', 'right_ear'],
  ['left_shoulder', 'right_shoulder'],
  ['left_shoulder', 'left_elbow'],
  ['left_elbow', 'left_wrist'],
  ['right_shoulder', 'right_elbow'],
  ['right_elbow', 'right_wrist'],
  ['left_shoulder', 'left_hip'],
  ['right_shoulder', 'right_hip'],
  ['left_hip', 'right_hip'],
  ['left_hip', 'left_knee'],
  ['left_knee', 'left_ankle'],
  ['right_hip', 'right_knee'],
  ['right_knee', 'right_ankle'],
] as const

function boxFromCenter(
  box: NonNullable<NonNullable<VisualResult['data']>[number]['box']>,
): Box | null {
  if (
    ![box.x, box.y, box.width, box.height].every(Number.isFinite) ||
    box.width <= 0 ||
    box.height <= 0
  )
    return null
  const clamp = (n: number) => Math.max(0, Math.min(1000, n * 1000))
  const result: Box = [
    clamp(box.x - box.width / 2),
    clamp(box.y - box.height / 2),
    clamp(box.x + box.width / 2),
    clamp(box.y + box.height / 2),
  ]
  return result[0] < result[2] && result[1] < result[3] ? result : null
}

function makePose(
  points: NonNullable<NonNullable<VisualResult['data']>[number]['pose']>,
): Pose | null {
  const valid = points
    .filter(
      (point) =>
        point.name.length > 0 &&
        point.name.length <= 24 &&
        Number.isFinite(point.x) &&
        point.x >= 0 &&
        point.x <= 1 &&
        Number.isFinite(point.y) &&
        point.y >= 0 &&
        point.y <= 1 &&
        Number.isFinite(point.confidence) &&
        point.confidence >= 0.5,
    )
    .slice(0, 32)
  if (!valid.length) return null
  const index = new Map(valid.map((point, i) => [point.name, i]))
  return {
    keypoints: valid.map((point) => ({
      label: point.name.replaceAll('_', ' '),
      x: point.x * 1000,
      y: point.y * 1000,
    })),
    connections: poseLinks.flatMap(([a, b]) => {
      const from = index.get(a),
        to = index.get(b)
      return from !== undefined && to !== undefined && from !== to
        ? [[from, to] as [number, number]]
        : []
    }),
  }
}

export function visualSnapshot(result: VisualResult, image: string): AnalysisSnapshot {
  const findings: Finding[] = (result.data ?? []).map((item, index) => {
    const pose = item.pose ? makePose(item.pose) : null
    const confidence = Number.isFinite(item.confidence)
      ? `${Math.round(item.confidence * 100)}% confidence`
      : 'Confidence unavailable'
    const details = item.pose
      ?.map((point) => `${point.name.replaceAll('_', ' ')}: ${Math.round(point.confidence * 100)}%`)
      .join(', ')
    return {
      n: index + 1,
      label: item.label,
      kind: 'image',
      group: item.pose ? 'structure' : 'entities',
      box: item.box ? boxFromCenter(item.box) : null,
      pose,
      description: details ? `${confidence}. Keypoints: ${details}` : confidence,
      evidence: '',
      point: null,
    }
  })
  return {
    ...EMPTY_SNAPSHOT,
    id: 'playground-visual',
    status: 'done',
    screenshot: image,
    model: result.model,
    findings,
    answer: findings.length
      ? `${findings.length} ${findings.length === 1 ? 'object' : 'objects'} detected.`
      : 'No objects detected.',
  }
}
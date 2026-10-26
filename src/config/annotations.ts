import type { CssRect } from './analysis'

// Keep callouts separate from their target geometry. Coordinates are CSS pixels
// so the same layout serves desktop overlays and resized session captures.
export function annotationLayout(
  rects: CssRect[],
  width: number,
  height: number,
  labels: string[] = [],
) {
  const labelHeight = 18
  const placed: CssRect[] = []
  return rects.map((rect, index) => {
    const labelWidth = Math.min(
      Math.max(64, (labels[index]?.length ?? 24) * 6 + 16),
      Math.max(0, width - 16),
    )
    const leftSide =
      rect.left >= labelWidth + 64 || rect.left + rect.width + labelWidth + 64 > width
    const anchor = {
      x: Math.max(0, Math.min(width, leftSide ? rect.left : rect.left + rect.width)),
      y: Math.max(0, Math.min(height, rect.top + Math.min(24, rect.height / 2))),
    }
    const preferredX = leftSide ? anchor.x - labelWidth - 64 : anchor.x + 64
    const candidates: CssRect[] = []
    for (let top = 8; top + labelHeight <= height - 8; top += labelHeight + 8) {
      for (const left of [
        Math.max(8, Math.min(width - labelWidth - 8, preferredX)),
        8,
        Math.max(8, width - labelWidth - 8),
      ]) {
        candidates.push({ left, top, width: labelWidth, height: labelHeight })
      }
    }
    const score = (label: CssRect) => {
      const overlapsTarget = rects.some(
        (target) =>
          label.left < target.left + target.width &&
          label.left + label.width > target.left &&
          label.top < target.top + target.height &&
          label.top + label.height > target.top,
      )
      return (
        (overlapsTarget ? 10000 : 0) +
        Math.hypot(label.left - preferredX, label.top + labelHeight - (anchor.y - 24))
      )
    }
    candidates.sort((a, b) => score(a) - score(b))
    const label = candidates.find((candidate) =>
      placed.every(
        (other) =>
          candidate.left >= other.left + other.width + 6 ||
          candidate.left + candidate.width + 6 <= other.left ||
          candidate.top >= other.top + other.height + 6 ||
          candidate.top + candidate.height + 6 <= other.top,
      ),
    )
    if (!label) return null // A tiny viewport cannot fit every callout safely.
    placed.push(label)
    const labelOnLeft = label.left + label.width / 2 < anchor.x
    const startX = labelOnLeft ? label.left : label.left + label.width
    const startY = label.top + label.height
    const elbowX = labelOnLeft ? label.left + label.width + 12 : label.left - 12
    const bendX = elbowX + (labelOnLeft ? 20 : -20)
    return {
      label,
      anchor,
      leader: `${startX},${startY} ${elbowX},${startY} ${bendX},${anchor.y} ${anchor.x},${anchor.y}`,
    }
  })
}
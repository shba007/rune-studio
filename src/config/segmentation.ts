export function classColor(id: number): [number, number, number] {
  const hue = ((id * 137.508 + 195) % 360) / 60
  const chroma = 0.68
  const secondary = chroma * (1 - Math.abs((hue % 2) - 1))
  const [r, g, b] =
    hue < 1
      ? [chroma, secondary, 0]
      : hue < 2
        ? [secondary, chroma, 0]
        : hue < 3
          ? [0, chroma, secondary]
          : hue < 4
            ? [0, secondary, chroma]
            : hue < 5
              ? [secondary, 0, chroma]
              : [chroma, 0, secondary]
  const offset = 0.14
  return [r, g, b].map((value) => Math.round((value + offset) * 255)) as [number, number, number]
}

export function colorClassMap(pixels: Uint8ClampedArray, classIds: ReadonlySet<number>): void {
  const palette = new Map(Array.from(classIds, (id) => [id, classColor(id)] as const))
  for (let i = 0; i < pixels.length; i += 4) {
    const id = pixels[i]!
    const color = palette.get(id)
    if (!color || pixels[i + 3] === 0) {
      pixels[i + 3] = 0
      continue
    }
    const [r, g, b] = color
    pixels[i] = r
    pixels[i + 1] = g
    pixels[i + 2] = b
    pixels[i + 3] = 255
  }
}
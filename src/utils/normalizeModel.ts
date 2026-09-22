// src/utils/normalizeModel.ts
import { Box3, Vector3, type Group } from 'three'

export function normalizeModel(model: Group): Vector3 {
  // 1. Measure raw dimensions
  const rawBox = new Box3().setFromObject(model)
  const rawSize = new Vector3()
  rawBox.getSize(rawSize)

  // 2. Scale proportionally so X is exactly 1.0 unit (1 meter)
  const normScale = rawSize.x > 0 ? 1.0 / rawSize.x : 1.0
  model.scale.setScalar(normScale)

  // 3. Center X/Z and ground bottom to Y = 0
  const normalizedBox = new Box3().setFromObject(model)
  const center = new Vector3()
  const finalSize = new Vector3()
  normalizedBox.getCenter(center)
  normalizedBox.getSize(finalSize)

  model.position.x = -center.x
  model.position.y = -normalizedBox.min.y
  model.position.z = -center.z

  return finalSize // Returns normalized [1.0, height, depth]
}
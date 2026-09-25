export type EntityType =
  | 'lighting'
  | 'surface'
  | 'backdrop'
  | 'printer'
  | 'monitor'
  | 'microchip'
  | 'satellite'
  | 'desk'
  | 'chair'
  | (string & {})

export interface SceneEntity {
  id: string
  type: EntityType
  position?: [number, number, number]
  rotation?: [number, number, number]
  scale?: [number, number, number] | number
  props?: Record<string, unknown>
}

export interface ScenePreset {
  id: string
  name: string
  description: string
  camera?: {
    position?: [number, number, number]
    lookAt?: [number, number, number]
    fov?: number
  }
  entities: SceneEntity[]
}
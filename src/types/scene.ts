export type EntityType =
  | 'lighting'
  | 'surface'       // Floor, PCB board, terrain
  | 'backdrop'      // Walls, device casing, skybox
  | 'printer'
  | 'monitor'
  | 'microchip'     // Inside electronic device
  | 'satellite'     // Space
  | 'desk'
  | 'chair'
  | (string & {})

export interface SceneEntity {
  id: string
  type: EntityType
  position?: [number, number, number]
  rotation?: [number, number, number]
  scale?: [number, number, number] | number
  props?: Record<string, any>
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
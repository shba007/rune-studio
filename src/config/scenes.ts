import type { ScenePreset } from '@/types/scene'

export const SCENE_PRESETS: Record<string, ScenePreset> = {
  'developer-studio': {
    id: 'developer-studio',
    name: 'Developer Studio',
    description: 'Physical desktop workspace with 3D table',
    camera: { position: [14, 14, 14], lookAt: [0, 1.2, 0], fov: 30 },
    entities: [
      {
        id: 'studio-lighting',
        type: 'lighting',
        props: { ambientIntensity: 0.9, directionalIntensity: 1.5 },
      },
      {
        id: 'studio-surface',
        type: 'surface',
        props: { size: [8, 8], color: '#64748b', showGrid: true },
      },
      {
        id: 'studio-backdrop',
        type: 'backdrop',
        props: { size: [8, 8], height: 6, color: '#cbd5e1' },
      },
      // The new 3D Table
      {
        id: 'main-table',
        type: 'table',
        position: [0, 0, 0], // Sits on floor at Y = 0
        scale: 0.6, // Adjust scale if needed
        rotation: [0, Math.PI / 2, 0],
      },
      // Place monitor & printer at tabletop height (check console for exact Y)
      {
        id: 'desk-monitor',
        type: 'monitor',
        position: [0, 0.738, 0], // e.g. Y = 0.75 if tabletop is 0.75m high
        scale: 0.6,
      },
      {
        id: 'desk-printer',
        type: 'printer',
        position: [-2.0, 0.421885, 0],
        scale: 0.44,
        rotation: [0, 0, 0],
      },
      {
        id: 'desk-chair',
        type: 'chair',
        position: [0, 0, 0.8],
        scale: 0.7,
        rotation: [0, Math.PI, 0],
      },
    ],
  },
}
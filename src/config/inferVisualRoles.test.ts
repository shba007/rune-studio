import { expect, it } from 'vite-plus/test'
import { updateVisualRoles } from './inferVisualRoles'

it('updates only visual role assignments and preserves unrelated model configuration', () => {
  const original = {
    schema_version: 3,
    models: [{ id: 'one', roles: ['visual_depth', 'voice'], extra: { device: 'cpu' } }],
    roles: { agent: 'one', visual_depth: 'one', visual_regions: 'old' },
    aliases: { primary: 'one' },
  }
  const result = JSON.parse(
    updateVisualRoles(JSON.stringify(original), {
      planner: 'one',
      entities: '',
      regions: '',
      structure: '',
      depth: '',
      instances: '',
      oriented: '',
    }),
  )
  expect(result).toEqual({
    schema_version: 3,
    models: [{ id: 'one', roles: ['voice'], extra: { device: 'cpu' } }],
    roles: { agent: 'one', visual_planner: 'one' },
    aliases: { primary: 'one' },
  })
})
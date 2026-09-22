<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted } from 'vue'
import { globalTools } from '@/services/toolRegistry';

const props = withDefaults(
  defineProps<{
    size?: [number, number]
    color?: string
    showGrid?: boolean
  }>(),
  {
    size: () => [8, 8],
    color: '#64748b',
    showGrid: true,
  },
)

const currentColor = ref(props.color)
watch(() => props.color, (c) => { currentColor.value = c })

let unregister: (() => void) | null = null

onMounted(() => {
  unregister = globalTools.register({
    name: 'set_surface_color',
    aliases: ['paint_surface', 'change_floor_color', 'change_substrate_color', 'surface_color'],
    description: 'Change the color/material of the ground, floor, or PCB substrate',
    parameters: {
      type: 'object',
      properties: {
        color: { type: 'string', description: 'Color name (e.g. coral, darkgreen, #064e3b) or hex' },
      },
      required: ['color'],
    },
    handler: (args) => {
      if (args.color) currentColor.value = args.color
    },
  })
})

onUnmounted(() => { unregister?.() })
</script>

<template>
  <TresGroup>
    <TresMesh :rotation="[-Math.PI / 2, 0, 0]" :position="[0, 0, 0]" receive-shadow>
      <TresPlaneGeometry :args="props.size" />
      <TresMeshStandardMaterial :color="currentColor" />
    </TresMesh>

    <TresGridHelper v-if="props.showGrid" :args="[props.size[0], props.size[1], '#475569', '#334155']"
      :position="[0, 0.01, 0]" />
  </TresGroup>
</template>
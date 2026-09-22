<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted } from 'vue'
import { DoubleSide } from 'three'
import { globalTools } from '@/services/toolRegistry';

const props = withDefaults(
  defineProps<{
    size?: [number, number]
    height?: number
    color?: string
  }>(),
  {
    size: () => [8, 8],
    height: 6,
    color: '#cbd5e1',
  },
)

const currentColor = ref(props.color)
watch(() => props.color, (c) => { currentColor.value = c })

let unregister: (() => void) | null = null

onMounted(() => {
  unregister = globalTools.register({
    name: 'set_backdrop_color',
    aliases: ['change_wall_color', 'set_casing_color', 'backdrop_color'],
    description: 'Change the color of the enclosure, walls, or device shielding',
    parameters: {
      type: 'object',
      properties: {
        color: { type: 'string', description: 'Color name or hex code' },
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
    <!-- Left Boundary -->
    <TresMesh :position="[0, props.height / 2, -props.size[1] / 2]">
      <TresPlaneGeometry :args="[props.size[0], props.height]" />
      <TresMeshStandardMaterial :color="currentColor" :side="DoubleSide" />
    </TresMesh>

    <!-- Right Boundary -->
    <TresMesh :position="[-props.size[0] / 2, props.height / 2, 0]" :rotation="[0, Math.PI / 2, 0]">
      <TresPlaneGeometry :args="[props.size[1], props.height]" />
      <TresMeshStandardMaterial :color="currentColor" :side="DoubleSide" />
    </TresMesh>
  </TresGroup>
</template>
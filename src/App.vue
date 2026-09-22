<script setup lang="ts">
import { TresCanvas } from '@tresjs/core'
import { useSceneManager } from '@/composables/useSceneManager'
import { useInference } from '@/composables/useInference'
import { globalTools } from '@/services/toolRegistry'
import SceneViewport from '@/components/SceneViewport.vue'
import CommandBar from '@/components/CommandBar.vue'

const { currentScene, currentSceneId, availableScenes, setScene } = useSceneManager()
const { isExecuting, lastRawResponse, errorMessage, executeTask } = useInference()

async function handleCommand(prompt: string) {
  try {
    const schema = globalTools.getSchema()
    const result = await executeTask({
      type: 'ToolCall',
      payload: { prompt, schema },
    })
    await globalTools.dispatch(result)
  } catch (err) {
    console.error('Task error:', err)
  }
}
</script>

<template>
  <main class="relative w-screen h-screen overflow-hidden bg-slate-950">
    <!-- Top-Left Environment Selector -->
    <div
      class="absolute top-4 left-4 z-20 flex items-center gap-1.5 bg-neutral-900/80 backdrop-blur-md border border-neutral-800 rounded-2xl p-1 shadow-lg">
      <button v-for="scene in availableScenes" :key="scene.id" @click="setScene(scene.id)"
        class="px-3 py-1.5 text-xs font-medium rounded-xl transition-all" :class="currentSceneId === scene.id
          ? 'bg-neutral-800 text-white shadow-sm'
          : 'text-neutral-400 hover:text-neutral-200'">
        {{ scene.name }}
      </button>
    </div>

    <!-- 3D Viewport -->
    <TresCanvas clear-color="#090d16" shadows>
      <SceneViewport :scene="currentScene" />
    </TresCanvas>

    <!-- Command Input -->
    <CommandBar :is-processing="isExecuting" :last-command="lastRawResponse" :error-message="errorMessage"
      @submit="handleCommand" />
  </main>
</template>
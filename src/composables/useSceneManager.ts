import { ref, computed } from "vue"
import { SCENE_PRESETS } from "@/config/scenes"
import type { ScenePreset } from "@/types/scene"

const currentSceneId = ref<string>('developer-studio')

export function useSceneManager() {
  const currentScene = computed<ScenePreset>(() => {
    return SCENE_PRESETS[currentSceneId.value] || SCENE_PRESETS['developer-studio']
  })

  function setScene(sceneId: string) {
    if (SCENE_PRESETS[sceneId]) {
      currentSceneId.value = sceneId
      console.log(`[SceneManager] Switched scene to: ${sceneId}`)
    } else {
      console.warn(`[SceneManager] Unknown scene: ${sceneId}`)
    }
  }

  return {
    currentSceneId,
    currentScene,
    availableScenes: Object.values(SCENE_PRESETS),
    setScene,
  }
}
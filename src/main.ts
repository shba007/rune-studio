import { createApp } from 'vue'

import 'unfonts.css'

import App from './App.vue'
import ControlOverlay from './components/ControlOverlay.vue'
import PlaygroundWindow from './components/PlaygroundWindow.vue'
import SettingsWindow from './components/SettingsWindow.vue'
import WorkspaceWindow from './components/WorkspaceWindow.vue'
import OnboardingWindow from './components/OnboardingWindow.vue'
import './style.css'

if (import.meta.env.DEV && !('__TAURI_INTERNALS__' in window)) {
  const { installTauriMock } = await import('./dev/mockTauri')
  installTauriMock()
}

const view = new URLSearchParams(location.search).get('view')
const overlay = view === 'overlay'
if (overlay) document.documentElement.classList.add('overlay-view')

if (['preview', 'settings', 'workspace', 'playground', 'onboarding'].includes(view ?? ''))
  document.documentElement.classList.add('window-view')

const root =
  view === 'onboarding'
    ? OnboardingWindow
    : view === 'overlay'
      ? ControlOverlay
      : view === 'preview'
        ? WorkspaceWindow
        : view === 'settings'
          ? SettingsWindow
          : view === 'workspace'
            ? WorkspaceWindow
            : view === 'playground'
              ? PlaygroundWindow
              : App
createApp(root).mount('#app')
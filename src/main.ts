import { createApp } from 'vue'

import 'unfonts.css'

import AnalysisView from './components/AnalysisView.vue'
import App from './App.vue'
import ControlOverlay from './components/ControlOverlay.vue'
import './style.css'

// In a plain browser (no Tauri) a dev build answers the backend commands with fixtures.
if (import.meta.env.DEV && !('__TAURI_INTERNALS__' in window)) {
  const { installTauriMock } = await import('./dev/mockTauri')
  installTauriMock()
}

// The same bundle serves the pill and the full-screen control overlay.
const view = new URLSearchParams(location.search).get('view')
// The full-screen overlay never shows the Vue devtools button (see style.css).
if (view === 'overlay') document.documentElement.classList.add('overlay-view')
// The analysis window is an ordinary opaque window, unlike the transparent pill (see style.css).
if (view === 'analysis') document.documentElement.classList.add('analysis-view')

createApp(view === 'overlay' ? ControlOverlay : view === 'analysis' ? AnalysisView : App).mount(
  '#app',
)
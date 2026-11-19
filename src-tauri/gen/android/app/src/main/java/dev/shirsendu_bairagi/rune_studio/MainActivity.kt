package dev.shirsendu_bairagi.rune_studio

import android.os.Bundle
import android.webkit.JavascriptInterface
import android.webkit.WebView
import androidx.activity.enableEdgeToEdge

class MainActivity : TauriActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
  }

  // The window is transparent, so a tap outside the orb should send Rune away and show
  // the app underneath again.
  override fun onWebViewCreate(webView: WebView) {
    webView.addJavascriptInterface(
      object {
        @JavascriptInterface
        fun close() {
          runOnUiThread { finishAndRemoveTask() }
        }
      },
      "RuneHost",
    )
  }
}

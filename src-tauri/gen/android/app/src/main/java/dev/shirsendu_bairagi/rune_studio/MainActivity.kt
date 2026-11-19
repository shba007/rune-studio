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

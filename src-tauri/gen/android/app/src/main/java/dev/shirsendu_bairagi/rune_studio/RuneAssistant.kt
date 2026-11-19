package dev.shirsendu_bairagi.rune_studio

import android.content.Intent
import android.os.Bundle
import android.service.voice.VoiceInteractionService
import android.service.voice.VoiceInteractionSession
import android.service.voice.VoiceInteractionSessionService
import android.speech.RecognitionService

class RuneVoiceInteractionService : VoiceInteractionService()

class RuneSessionService : VoiceInteractionSessionService() {
  override fun onNewSession(args: Bundle?): VoiceInteractionSession = RuneSession(this)
}

class RuneSession(private val service: RuneSessionService) : VoiceInteractionSession(service) {
  override fun onShow(args: Bundle?, showFlags: Int) {
    super.onShow(args, showFlags)
    val intent =
      Intent(service, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
    startAssistantActivity(intent)
    hide()
  }
}

class RuneRecognitionService : RecognitionService() {
  override fun onStartListening(intent: Intent?, listener: Callback?) {}
  override fun onCancel(listener: Callback?) {}
  override fun onStopListening(listener: Callback?) {}
}

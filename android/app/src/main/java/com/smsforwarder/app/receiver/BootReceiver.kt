package com.smsforwarder.app.receiver

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.util.Log
import com.smsforwarder.app.SmsForwarderApplication
import com.smsforwarder.app.service.SmsForwardWorker
import com.smsforwarder.app.service.SmsForwarderService

class BootReceiver : BroadcastReceiver() {

    override fun onReceive(context: Context, intent: Intent) {
        val action = intent.action
        Log.i(TAG, "BootReceiver received action: $action")

        val app = context.applicationContext as SmsForwarderApplication
        val prefs = app.preferences

        if (prefs.isForwardingEnabled) {
            // Start Foreground Service if enabled in settings
            if (prefs.isForegroundServiceEnabled) {
                try {
                    SmsForwarderService.start(context)
                } catch (e: Exception) {
                    Log.e(TAG, "Failed to start foreground service on boot: ${e.message}")
                }
            }

            // Flush any pending SMS queued while phone was off
            SmsForwardWorker.enqueue(context)
        }
    }

    companion object {
        private const val TAG = "BootReceiver"
    }
}

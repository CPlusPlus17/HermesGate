package com.smsforwarder.app.receiver

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import android.provider.Telephony
import android.telephony.SubscriptionManager
import android.util.Log
import androidx.core.content.ContextCompat
import com.smsforwarder.app.SmsForwarderApplication
import com.smsforwarder.app.data.SmsMessageEntity
import com.smsforwarder.app.service.SmsForwardWorker
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch

class SmsBroadcastReceiver : BroadcastReceiver() {

    override fun onReceive(context: Context, intent: Intent) {
        if (intent.action != Telephony.Sms.Intents.SMS_RECEIVED_ACTION) {
            return
        }

        val app = context.applicationContext as SmsForwarderApplication
        val prefs = app.preferences

        if (!prefs.isForwardingEnabled) {
            Log.d(TAG, "SMS Forwarding is currently disabled; skipping message.")
            return
        }

        val messages = Telephony.Sms.Intents.getMessagesFromIntent(intent)
        if (messages.isNullOrEmpty()) {
            return
        }

        // Reconstruct full message body from multipart fragments
        val sender = messages[0].displayOriginatingAddress ?: messages[0].originatingAddress ?: "Unknown"
        val timestamp = messages[0].timestampMillis
        val fullBodyBuilder = StringBuilder()
        for (sms in messages) {
            sms.displayMessageBody?.let { fullBodyBuilder.append(it) }
        }
        val fullBody = fullBodyBuilder.toString()

        // Extract SIM slot & subscription ID
        val simSlot = extractSimSlot(intent)
        val recipientNumber = determineRecipientNumber(context, intent, simSlot, prefs.customPhoneNumber)

        Log.i(TAG, "📥 Intercepted SMS for $recipientNumber from $sender on SIM $simSlot: ${fullBody.take(40)}...")

        val entity = SmsMessageEntity(
            recipientNumber = recipientNumber,
            sender = sender,
            body = fullBody,
            timestamp = timestamp,
            simSlot = simSlot,
            status = SmsMessageEntity.STATUS_PENDING
        )

        // Asynchronously save to Room DB & trigger immediate forward worker
        val pendingResult = goAsync()
        CoroutineScope(Dispatchers.IO).launch {
            try {
                app.database.smsDao().insert(entity)
                // Enqueue work manager to ensure reliable delivery
                SmsForwardWorker.enqueue(context)
            } catch (e: Exception) {
                Log.e(TAG, "Error saving SMS to local queue: ${e.message}", e)
            } finally {
                pendingResult.finish()
            }
        }
    }

    private fun extractSimSlot(intent: Intent): Int {
        val extras = intent.extras ?: return 0
        // Various manufacturer extras for SIM slot
        return when {
            extras.containsKey("slot") -> extras.getInt("slot", 0)
            extras.containsKey("simSlot") -> extras.getInt("simSlot", 0)
            extras.containsKey("sim_slot") -> extras.getInt("sim_slot", 0)
            extras.containsKey("android.telephony.extra.SLOT_INDEX") -> extras.getInt("android.telephony.extra.SLOT_INDEX", 0)
            extras.containsKey("subscription") -> {
                val subId = extras.getInt("subscription", -1)
                if (subId >= 0) subId % 2 else 0
            }
            else -> 0
        }
    }

    private fun determineRecipientNumber(
        context: Context,
        intent: Intent,
        simSlot: Int,
        configuredNumber: String
    ): String {
        // 1. Try reading real phone number from SubscriptionManager if permission granted
        if (ContextCompat.checkSelfPermission(context, android.Manifest.permission.READ_PHONE_STATE) == PackageManager.PERMISSION_GRANTED) {
            try {
                val subscriptionManager = context.getSystemService(Context.TELEPHONY_SUBSCRIPTION_SERVICE) as? SubscriptionManager
                val subList = subscriptionManager?.activeSubscriptionInfoList
                if (!subList.isNullOrEmpty()) {
                    val subInfo = subList.find { it.simSlotIndex == simSlot } ?: subList[0]
                    var detectedNumber = subInfo.number
                    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU && detectedNumber.isNullOrEmpty()) {
                        detectedNumber = subscriptionManager.getPhoneNumber(subInfo.subscriptionId)
                    }
                    if (!detectedNumber.isNullOrEmpty()) {
                        return detectedNumber
                    }
                }
            } catch (e: Exception) {
                Log.w(TAG, "Could not auto-read SIM phone number: ${e.message}")
            }
        }

        // 2. Use user configured custom phone number if available
        if (configuredNumber.isNotEmpty()) {
            return configuredNumber
        }

        // 3. Fallback: label with SIM slot
        return "SIM_${simSlot + 1}_DEVICE"
    }

    companion object {
        private const val TAG = "SmsBroadcastReceiver"
    }
}

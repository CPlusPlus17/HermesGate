package com.smsforwarder.app.service

import android.content.Context
import android.util.Log
import androidx.work.*
import com.smsforwarder.app.SmsForwarderApplication
import com.smsforwarder.app.data.SmsMessageEntity
import com.smsforwarder.app.network.ForwarderApiClient
import com.smsforwarder.app.network.SmsForwardRequest
import java.util.concurrent.TimeUnit

class SmsForwardWorker(
    context: Context,
    workerParams: WorkerParameters
) : CoroutineWorker(context, workerParams) {

    private val apiClient = ForwarderApiClient()

    override suspend fun doWork(): Result {
        val app = applicationContext as SmsForwarderApplication
        val prefs = app.preferences
        val dao = app.database.smsDao()

        if (!prefs.isForwardingEnabled) {
            Log.i(TAG, "SMS Forwarding is disabled in preferences.")
            return Result.success()
        }

        val pendingMessages = dao.getMessagesByStatus(SmsMessageEntity.STATUS_PENDING)
        if (pendingMessages.isEmpty()) {
            return Result.success()
        }

        Log.i(TAG, "Processing ${pendingMessages.size} pending SMS messages...")
        var hasFailures = false

        for (sms in pendingMessages) {
            val payload = SmsForwardRequest(
                recipientNumber = sms.recipientNumber,
                sender = sms.sender,
                body = sms.body,
                simSlot = sms.simSlot,
                deviceId = prefs.deviceId,
                metadata = mapOf("retryCount" to sms.retryCount)
            )

            val forwardResult = apiClient.forwardSms(
                baseUrl = prefs.serverUrl,
                deviceToken = prefs.deviceToken,
                payload = payload
            )

            forwardResult.onSuccess { response ->
                Log.i(TAG, "Successfully forwarded SMS ID ${sms.id}. Server ID: ${response.messageId}")
                val updated = sms.copy(
                    status = SmsMessageEntity.STATUS_SENT,
                    serverMessageId = response.messageId,
                    errorMessage = null
                )
                dao.update(updated)
                prefs.lastForwardTime = System.currentTimeMillis()
            }.onFailure { error ->
                Log.e(TAG, "Failed to forward SMS ID ${sms.id}: ${error.message}")
                val nextRetry = sms.retryCount + 1
                val updated = sms.copy(
                    status = if (nextRetry >= MAX_RETRIES) SmsMessageEntity.STATUS_FAILED else SmsMessageEntity.STATUS_PENDING,
                    retryCount = nextRetry,
                    errorMessage = error.message
                )
                dao.update(updated)
                hasFailures = true
            }
        }

        return if (hasFailures && runAttemptCount < MAX_RETRIES) {
            Result.retry()
        } else {
            Result.success()
        }
    }

    companion object {
        private const val TAG = "SmsForwardWorker"
        private const val MAX_RETRIES = 5

        fun enqueue(context: Context) {
            val constraints = Constraints.Builder()
                .setRequiredNetworkType(NetworkType.CONNECTED)
                .build()

            val request = OneTimeWorkRequestBuilder<SmsForwardWorker>()
                .setConstraints(constraints)
                .setBackoffCriteria(
                    BackoffPolicy.EXPONENTIAL,
                    10,
                    TimeUnit.SECONDS
                )
                .build()

            WorkManager.getInstance(context).enqueueUniqueWork(
                "SmsForwardUniqueWork",
                ExistingWorkPolicy.REPLACE,
                request
            )
        }
    }
}

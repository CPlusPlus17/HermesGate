package com.smsforwarder.app.network

import com.google.gson.Gson
import com.google.gson.annotations.SerializedName
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import java.io.IOException
import java.util.concurrent.TimeUnit

data class SmsForwardRequest(
    @SerializedName("recipient_number") val recipientNumber: String,
    @SerializedName("sender") val sender: String,
    @SerializedName("body") val body: String,
    @SerializedName("sim_slot") val simSlot: Int,
    @SerializedName("device_id") val deviceId: String,
    @SerializedName("metadata") val metadata: Map<String, Any>? = null
)

data class SmsForwardResponse(
    val success: Boolean,
    @SerializedName("message_id") val messageId: String?,
    @SerializedName("extracted_code") val extractedCode: String?,
    val error: String?,
    val message: String?
)

data class HealthResponse(
    val status: String,
    val service: String?,
    val version: String?
)

class ForwarderApiClient {
    private val client: OkHttpClient = OkHttpClient.Builder()
        .connectTimeout(15, TimeUnit.SECONDS)
        .readTimeout(15, TimeUnit.SECONDS)
        .writeTimeout(15, TimeUnit.SECONDS)
        .build()

    private val gson = Gson()
    private val jsonMediaType = "application/json; charset=utf-8".toMediaType()

    suspend fun checkHealth(baseUrl: String): Result<HealthResponse> = withContext(Dispatchers.IO) {
        try {
            val url = "${baseUrl.trim().removeSuffix("/")}/api/v1/health"
            val request = Request.Builder()
                .url(url)
                .get()
                .build()

            client.newCall(request).execute().use { response ->
                if (!response.isSuccessful) {
                    return@withContext Result.failure(IOException("HTTP ${response.code}: ${response.message}"))
                }
                val body = response.body?.string() ?: ""
                val health = gson.fromJson(body, HealthResponse::class.java)
                Result.success(health)
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    suspend fun forwardSms(
        baseUrl: String,
        deviceToken: String,
        payload: SmsForwardRequest
    ): Result<SmsForwardResponse> = withContext(Dispatchers.IO) {
        try {
            val url = "${baseUrl.trim().removeSuffix("/")}/api/v1/sms/forward"
            val jsonString = gson.toJson(payload)
            val requestBody = jsonString.toRequestBody(jsonMediaType)

            val requestBuilder = Request.Builder()
                .url(url)
                .post(requestBody)
                .addHeader("Content-Type", "application/json")

            if (deviceToken.isNotEmpty()) {
                requestBuilder.addHeader("X-Device-Token", deviceToken)
                requestBuilder.addHeader("Authorization", "Bearer $deviceToken")
            }

            client.newCall(requestBuilder.build()).execute().use { response ->
                val body = response.body?.string() ?: ""
                if (!response.isSuccessful) {
                    return@withContext Result.failure(IOException("Server error ${response.code}: $body"))
                }
                val forwardResponse = gson.fromJson(body, SmsForwardResponse::class.java)
                Result.success(forwardResponse)
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }
}

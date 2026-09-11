package com.smsforwarder.app.data

import androidx.room.Entity
import androidx.room.PrimaryKey

@Entity(tableName = "sms_queue")
data class SmsMessageEntity(
    @PrimaryKey(autoGenerate = true)
    val id: Long = 0,
    val recipientNumber: String,
    val sender: String,
    val body: String,
    val timestamp: Long,
    val simSlot: Int = 0,
    val status: String = STATUS_PENDING, // PENDING, SENT, FAILED
    val retryCount: Int = 0,
    val errorMessage: String? = null,
    val serverMessageId: String? = null
) {
    companion object {
        const val STATUS_PENDING = "PENDING"
        const val STATUS_SENT = "SENT"
        const val STATUS_FAILED = "FAILED"
    }
}

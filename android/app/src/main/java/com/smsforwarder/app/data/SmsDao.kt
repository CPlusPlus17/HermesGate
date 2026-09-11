package com.smsforwarder.app.data

import androidx.room.*
import kotlinx.coroutines.flow.Flow

@Dao
interface SmsDao {
    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun insert(sms: SmsMessageEntity): Long

    @Update
    suspend fun update(sms: SmsMessageEntity)

    @Query("SELECT * FROM sms_queue WHERE status = :status ORDER BY timestamp ASC")
    suspend fun getMessagesByStatus(status: String): List<SmsMessageEntity>

    @Query("SELECT * FROM sms_queue ORDER BY timestamp DESC LIMIT 100")
    fun getAllMessagesFlow(): Flow<List<SmsMessageEntity>>

    @Query("SELECT * FROM sms_queue ORDER BY timestamp DESC LIMIT 50")
    suspend fun getRecentMessages(): List<SmsMessageEntity>

    @Query("DELETE FROM sms_queue WHERE status = 'SENT' AND timestamp < :cutoffTimestamp")
    suspend fun cleanOldSentMessages(cutoffTimestamp: Long)
}

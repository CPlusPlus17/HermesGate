package com.smsforwarder.app.data

import android.content.Context
import android.content.SharedPreferences
import android.os.Build
import java.util.UUID

class PreferencesManager(context: Context) {
    private val prefs: SharedPreferences = context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)

    var serverUrl: String
        get() = prefs.getString(KEY_SERVER_URL, "http://192.168.1.100:8080") ?: "http://192.168.1.100:8080"
        set(value) = prefs.edit().putString(KEY_SERVER_URL, value.trim().removeSuffix("/")).apply()

    var deviceToken: String
        get() = prefs.getString(KEY_DEVICE_TOKEN, "") ?: ""
        set(value) = prefs.edit().putString(KEY_DEVICE_TOKEN, value.trim()).apply()

    var deviceId: String
        get() {
            var id = prefs.getString(KEY_DEVICE_ID, null)
            if (id.isNullOrEmpty()) {
                id = "android_${UUID.randomUUID().toString().substring(0, 8)}"
                prefs.edit().putString(KEY_DEVICE_ID, id).apply()
            }
            return id
        }
        set(value) = prefs.edit().putString(KEY_DEVICE_ID, value.trim()).apply()

    var deviceName: String
        get() = prefs.getString(KEY_DEVICE_NAME, "${Build.MANUFACTURER} ${Build.MODEL}".trim()) ?: "${Build.MANUFACTURER} ${Build.MODEL}".trim()
        set(value) = prefs.edit().putString(KEY_DEVICE_NAME, value.trim()).apply()

    var customPhoneNumber: String
        get() = prefs.getString(KEY_PHONE_NUMBER, "") ?: ""
        set(value) = prefs.edit().putString(KEY_PHONE_NUMBER, value.trim()).apply()

    var isForwardingEnabled: Boolean
        get() = prefs.getBoolean(KEY_FORWARDING_ENABLED, true)
        set(value) = prefs.edit().putBoolean(KEY_FORWARDING_ENABLED, value).apply()

    var isForegroundServiceEnabled: Boolean
        get() = prefs.getBoolean(KEY_FOREGROUND_ENABLED, true)
        set(value) = prefs.edit().putBoolean(KEY_FOREGROUND_ENABLED, value).apply()

    var lastForwardTime: Long
        get() = prefs.getLong(KEY_LAST_FORWARD_TIME, 0L)
        set(value) = prefs.edit().putLong(KEY_LAST_FORWARD_TIME, value).apply()

    companion object {
        private const val PREFS_NAME = "sms_forwarder_prefs"
        private const val KEY_SERVER_URL = "server_url"
        private const val KEY_DEVICE_TOKEN = "device_token"
        private const val KEY_DEVICE_ID = "device_id"
        private const val KEY_DEVICE_NAME = "device_name"
        private const val KEY_PHONE_NUMBER = "phone_number"
        private const val KEY_FORWARDING_ENABLED = "forwarding_enabled"
        private const val KEY_FOREGROUND_ENABLED = "foreground_enabled"
        private const val KEY_LAST_FORWARD_TIME = "last_forward_time"
    }
}

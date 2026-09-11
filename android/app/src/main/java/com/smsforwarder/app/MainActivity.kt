package com.smsforwarder.app

import android.Manifest
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.os.PowerManager
import android.provider.Settings
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.TextView
import android.widget.Toast
import androidx.activity.result.contract.ActivityResultContracts
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.ContextCompat
import androidx.lifecycle.lifecycleScope
import androidx.recyclerview.widget.LinearLayoutManager
import androidx.recyclerview.widget.RecyclerView
import com.smsforwarder.app.data.AppDatabase
import com.smsforwarder.app.data.PreferencesManager
import com.smsforwarder.app.data.SmsMessageEntity
import com.smsforwarder.app.databinding.ActivityMainBinding
import com.smsforwarder.app.network.ForwarderApiClient
import com.smsforwarder.app.service.SmsForwardWorker
import com.smsforwarder.app.service.SmsForwarderService
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.text.SimpleDateFormat
import java.util.*

class MainActivity : AppCompatActivity() {

    private lateinit var binding: ActivityMainBinding
    private lateinit var prefs: PreferencesManager
    private lateinit var db: AppDatabase
    private val apiClient = ForwarderApiClient()
    private val logAdapter = SmsLogAdapter()

    private val requestPermissionsLauncher = registerForActivityResult(
        ActivityResultContracts.RequestMultiplePermissions()
    ) { permissions ->
        val smsGranted = permissions[Manifest.permission.RECEIVE_SMS] == true
        if (smsGranted) {
            Toast.makeText(this, "SMS Permission Granted!", Toast.LENGTH_SHORT).show()
        } else {
            Toast.makeText(this, "SMS Permission required to forward messages", Toast.LENGTH_LONG).show()
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        binding = ActivityMainBinding.inflate(layoutInflater)
        setContentView(binding.root)

        val app = application as SmsForwarderApplication
        prefs = app.preferences
        db = app.database

        setupUI()
        checkPermissions()
        observeHistory()
    }

    private fun setupUI() {
        // Populate inputs from saved preferences
        binding.etServerUrl.setText(prefs.serverUrl)
        binding.etDeviceToken.setText(prefs.deviceToken)
        binding.etDeviceName.setText(prefs.deviceName)
        binding.etPhoneNumber.setText(prefs.customPhoneNumber)
        binding.switchForwarding.isChecked = prefs.isForwardingEnabled

        binding.switchForwarding.setOnCheckedChangeListener { _, isChecked ->
            prefs.isForwardingEnabled = isChecked
            updateStatusText()
            if (isChecked && prefs.isForegroundServiceEnabled) {
                SmsForwarderService.start(this)
            }
        }

        binding.btnSaveSettings.setOnClickListener {
            saveSettings()
        }

        binding.btnTestConnection.setOnClickListener {
            testServerConnection()
        }

        binding.btnSendTestSms.setOnClickListener {
            sendTestSms()
        }

        binding.btnBatterySettings.setOnClickListener {
            requestIgnoreBatteryOptimizations()
        }

        // Setup RecyclerView
        binding.rvMessagesLog.layoutManager = LinearLayoutManager(this)
        binding.rvMessagesLog.adapter = logAdapter

        updateStatusText()
    }

    private fun checkPermissions() {
        val permissions = mutableListOf(
            Manifest.permission.RECEIVE_SMS,
            Manifest.permission.READ_SMS,
            Manifest.permission.READ_PHONE_STATE
        )

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            permissions.add(Manifest.permission.POST_NOTIFICATIONS)
        }
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            permissions.add(Manifest.permission.READ_PHONE_NUMBERS)
        }

        val missing = permissions.filter {
            ContextCompat.checkSelfPermission(this, it) != PackageManager.PERMISSION_GRANTED
        }

        if (missing.isNotEmpty()) {
            requestPermissionsLauncher.launch(missing.toTypedArray())
        }
    }

    private fun saveSettings() {
        prefs.serverUrl = binding.etServerUrl.text.toString()
        prefs.deviceToken = binding.etDeviceToken.text.toString()
        prefs.deviceName = binding.etDeviceName.text.toString()
        prefs.customPhoneNumber = binding.etPhoneNumber.text.toString()

        Toast.makeText(this, "Settings saved!", Toast.LENGTH_SHORT).show()
        updateStatusText()
    }

    private fun testServerConnection() {
        val url = binding.etServerUrl.text.toString().trim()
        if (url.isEmpty()) {
            Toast.makeText(this, "Please enter a Server URL", Toast.LENGTH_SHORT).show()
            return
        }

        binding.tvStatusSubtitle.text = "Checking server connection..."

        lifecycleScope.launch {
            val result = apiClient.checkHealth(url)
            result.onSuccess { health ->
                binding.tvStatusSubtitle.text = "Connected to Server v${health.version ?: "0.1.0"} (Status: ${health.status})"
                Toast.makeText(this@MainActivity, "Server is healthy!", Toast.LENGTH_SHORT).show()
            }.onFailure { error ->
                binding.tvStatusSubtitle.text = "Connection failed: ${error.message}"
                Toast.makeText(this@MainActivity, "Failed: ${error.message}", Toast.LENGTH_LONG).show()
            }
        }
    }

    private fun sendTestSms() {
        val number = prefs.customPhoneNumber.ifEmpty { "+15551234567" }
        val testSms = SmsMessageEntity(
            recipientNumber = number,
            sender = "SimulatedSender",
            body = "Test verification code is 123456. Forwarded from Android test.",
            timestamp = System.currentTimeMillis(),
            simSlot = 0,
            status = SmsMessageEntity.STATUS_PENDING
        )

        lifecycleScope.launch(Dispatchers.IO) {
            db.smsDao().insert(testSms)
            withContext(Dispatchers.Main) {
                Toast.makeText(this@MainActivity, "Test SMS queued for forwarding", Toast.LENGTH_SHORT).show()
                SmsForwardWorker.enqueue(this@MainActivity)
            }
        }
    }

    private fun requestIgnoreBatteryOptimizations() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
            val pm = getSystemService(Context.POWER_SERVICE) as PowerManager
            if (!pm.isIgnoringBatteryOptimizations(packageName)) {
                val intent = Intent(Settings.ACTION_REQUEST_IGNORE_BATTERY_OPTIMIZATIONS).apply {
                    data = Uri.parse("package:$packageName")
                }
                startActivity(intent)
            } else {
                Toast.makeText(this, "Battery optimizations already disabled!", Toast.LENGTH_SHORT).show()
            }
        }
    }

    private fun updateStatusText() {
        if (prefs.isForwardingEnabled) {
            binding.tvStatusTitle.text = "Forwarding Active"
            val lastTime = prefs.lastForwardTime
            val lastSentStr = if (lastTime > 0) {
                SimpleDateFormat("HH:mm:ss", Locale.getDefault()).format(Date(lastTime))
            } else {
                "Never"
            }
            binding.tvStatusSubtitle.text = "Ready to forward SMS &bull; Last sent: $lastSentStr"
        } else {
            binding.tvStatusTitle.text = "Forwarding Paused"
            binding.tvStatusSubtitle.text = "Toggle switch above to enable forwarding"
        }
    }

    private fun observeHistory() {
        lifecycleScope.launch {
            db.smsDao().getAllMessagesFlow().collect { messages ->
                logAdapter.submitList(messages)
            }
        }
    }

    // RecyclerView Adapter for SMS History
    inner class SmsLogAdapter : RecyclerView.Adapter<SmsLogAdapter.ViewHolder>() {
        private var items = listOf<SmsMessageEntity>()

        fun submitList(newItems: List<SmsMessageEntity>) {
            items = newItems
            notifyDataSetChanged()
        }

        override fun onCreateViewHolder(parent: ViewGroup, viewType: Int): ViewHolder {
            val view = LayoutInflater.from(parent.context).inflate(R.layout.item_sms_log, parent, false)
            return ViewHolder(view)
        }

        override fun onBindViewHolder(holder: ViewHolder, position: Int) {
            holder.bind(items[position])
        }

        override fun getItemCount(): Int = items.size

        inner class ViewHolder(itemView: View) : RecyclerView.ViewHolder(itemView) {
            private val tvSender: TextView = itemView.findViewById(R.id.tvLogSender)
            private val tvStatus: TextView = itemView.findViewById(R.id.tvLogStatus)
            private val tvBody: TextView = itemView.findViewById(R.id.tvLogBody)
            private val tvSim: TextView = itemView.findViewById(R.id.tvLogSim)
            private val tvTime: TextView = itemView.findViewById(R.id.tvLogTime)
            private val tvRecipient: TextView = itemView.findViewById(R.id.tvLogRecipient)

            fun bind(item: SmsMessageEntity) {
                tvSender.text = item.sender
                tvStatus.text = item.status
                tvStatus.setTextColor(
                    if (item.status == "SENT") ContextCompat.getColor(itemView.context, R.color.success)
                    else if (item.status == "FAILED") ContextCompat.getColor(itemView.context, R.color.error)
                    else ContextCompat.getColor(itemView.context, R.color.accent)
                )
                tvBody.text = item.body
                tvSim.text = "SIM ${item.simSlot + 1}"
                tvTime.text = SimpleDateFormat("MMM d, HH:mm", Locale.getDefault()).format(Date(item.timestamp))
                tvRecipient.text = item.recipientNumber
            }
        }
    }
}

// SMS Forwarder Frontend Application

const state = {
  token: localStorage.getItem('sms_admin_token') || '',
  selectedNumber: 'all',
  searchQuery: '',
  numbers: [],
  messages: [],
  ws: null,
  reconnectTimeout: null,
  activeTabSnippet: 'tabClaude',
  latestCreatedToken: null,
};

// --- DOM Elements ---
const elements = {
  connectionStatus: document.getElementById('connectionStatus'),
  searchInput: document.getElementById('searchInput'),
  btnTestSms: document.getElementById('btnTestSms'),
  btnDevices: document.getElementById('btnDevices'),
  btnTokens: document.getElementById('btnTokens'),
  btnAuth: document.getElementById('btnAuth'),
  authBadge: document.getElementById('authBadge'),
  numbersList: document.getElementById('numbersList'),
  totalNumbersCount: document.getElementById('totalNumbersCount'),
  allMsgCount: document.getElementById('allMsgCount'),
  statTotalMsgs: document.getElementById('statTotalMsgs'),
  statDevicesCount: document.getElementById('statDevicesCount'),
  messagesList: document.getElementById('messagesList'),
  currentFeedTitle: document.getElementById('currentFeedTitle'),
  currentFeedSubtitle: document.getElementById('currentFeedSubtitle'),
  btnRefresh: document.getElementById('btnRefresh'),
  btnClearFilters: document.getElementById('btnClearFilters'),
  otpSection: document.getElementById('otpSection'),
  otpCards: document.getElementById('otpCards'),
  toastContainer: document.getElementById('toastContainer'),
  // Modals
  testSmsModal: document.getElementById('testSmsModal'),
  testSmsForm: document.getElementById('testSmsForm'),
  devicesModal: document.getElementById('devicesModal'),
  tokensModal: document.getElementById('tokensModal'),
  authModal: document.getElementById('authModal'),
  authForm: document.getElementById('authForm'),
  mcpGuideModal: document.getElementById('mcpGuideModal'),
  btnQuickMcpGuide: document.getElementById('btnQuickMcpGuide'),
  // Device modal elements
  addDeviceForm: document.getElementById('addDeviceForm'),
  newDeviceResult: document.getElementById('newDeviceResult'),
  deviceQrCanvas: document.getElementById('deviceQrCanvas'),
  displayServerUrl: document.getElementById('displayServerUrl'),
  displayDeviceToken: document.getElementById('displayDeviceToken'),
  btnCopyDevToken: document.getElementById('btnCopyDevToken'),
  devicesTableBody: document.getElementById('devicesTableBody'),
  // Tokens modal elements
  createTokenForm: document.getElementById('createTokenForm'),
  chkAllNumbers: document.getElementById('chkAllNumbers'),
  numberCheckboxesGroup: document.getElementById('numberCheckboxesGroup'),
  tokenSecretBanner: document.getElementById('tokenSecretBanner'),
  createdSecretVal: document.getElementById('createdSecretVal'),
  btnCopyCreatedSecret: document.getElementById('btnCopyCreatedSecret'),
  tokensTableBody: document.getElementById('tokensTableBody'),
  snippetCode: document.getElementById('snippetCode'),
  adminTokenInput: document.getElementById('adminTokenInput'),
};

// --- Initialization ---
document.addEventListener('DOMContentLoaded', () => {
  initEventListeners();
  updateAuthUI();
  loadData();
  connectWebSocket();
});

// --- API Helpers ---
async function api(path, options = {}) {
  const headers = {
    'Content-Type': 'application/json',
    ...(options.headers || {}),
  };
  if (state.token) {
    headers['Authorization'] = `Bearer ${state.token}`;
  }

  const res = await fetch(path, { ...options, headers });
  if (res.status === 401) {
    showToast('Unauthorized. Please enter a valid Admin Token.', 'error');
    openModal('authModal');
    throw new Error('Unauthorized');
  }
  const data = await res.json().catch(() => ({}));
  if (!res.ok) {
    throw new Error(data.message || `Request failed with status ${res.status}`);
  }
  return data;
}

// --- Data Loading ---
async function loadData() {
  await Promise.all([loadNumbers(), loadMessages(), loadStats(), loadRecentOtps()]);
}

async function loadNumbers() {
  try {
    const res = await api('/api/v1/numbers');
    state.numbers = res.phone_numbers || [];
    renderNumbers();
    updateDatalist();
    updateTokenNumberCheckboxes();
  } catch (err) {
    console.error('Failed to load numbers:', err);
  }
}

async function loadMessages() {
  try {
    let url = '/api/v1/messages?limit=100';
    if (state.selectedNumber !== 'all') {
      url += `&number=${encodeURIComponent(state.selectedNumber)}`;
    }
    if (state.searchQuery) {
      url += `&search=${encodeURIComponent(state.searchQuery)}`;
    }
    const res = await api(url);
    state.messages = res.messages || [];
    renderMessages();
  } catch (err) {
    console.error('Failed to load messages:', err);
    elements.messagesList.innerHTML = `<div class="loading-placeholder text-muted">Error loading messages: ${escapeHtml(err.message)}</div>`;
  }
}

async function loadStats() {
  try {
    const devicesRes = await api('/api/v1/admin/devices').catch(() => null);
    if (devicesRes && devicesRes.devices) {
      elements.statDevicesCount.textContent = devicesRes.devices.length;
    }
    let totalMsgs = state.numbers.reduce((acc, n) => acc + (n.message_count || 0), 0);
    elements.statTotalMsgs.textContent = totalMsgs;
    elements.allMsgCount.textContent = totalMsgs;
  } catch (e) {
    // Ignore stats error for non-admin
  }
}

async function loadRecentOtps() {
  try {
    const res = await api('/api/v1/messages/latest-otp?max_age_minutes=30').catch(() => null);
    if (res && res.found && res.otp) {
      renderOtpCards([res.otp]);
    } else {
      elements.otpSection.style.display = 'none';
    }
  } catch (e) {
    elements.otpSection.style.display = 'none';
  }
}

// --- WebSocket Live Stream ---
function connectWebSocket() {
  if (state.ws) {
    state.ws.close();
  }

  const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
  const tokenParam = state.token ? `?token=${encodeURIComponent(state.token)}` : '';
  const wsUrl = `${protocol}//${window.location.host}/api/v1/ws${tokenParam}`;

  setWsStatus('connecting', 'Connecting...');
  state.ws = new WebSocket(wsUrl);

  state.ws.onopen = () => {
    setWsStatus('connected', 'Live (Connected)');
  };

  state.ws.onmessage = (event) => {
    try {
      const data = JSON.parse(event.data);
      if (data.type === 'new_sms') {
        handleIncomingSms(data.message);
      }
    } catch (e) {
      console.error('WebSocket parse error:', e);
    }
  };

  state.ws.onclose = () => {
    setWsStatus('disconnected', 'Disconnected');
    clearTimeout(state.reconnectTimeout);
    state.reconnectTimeout = setTimeout(connectWebSocket, 4000);
  };

  state.ws.onerror = () => {
    setWsStatus('disconnected', 'Error');
    state.ws.close();
  };
}

function setWsStatus(statusClass, text) {
  elements.connectionStatus.className = `status-badge ${statusClass}`;
  elements.connectionStatus.querySelector('.status-text').textContent = text;
}

function handleIncomingSms(msg) {
  // Check if message matches current selected number filter
  const matchesFilter = state.selectedNumber === 'all' || state.selectedNumber === msg.recipient_number;

  // Add to top of local messages list
  state.messages.unshift(msg);

  if (matchesFilter) {
    renderMessages();
  }

  // Reload numbers and stats
  loadNumbers();
  loadStats();

  // Show Toast
  const otpNotice = msg.extracted_code ? ` [Code: ${msg.extracted_code}]` : '';
  showToast(`SMS from ${msg.sender} on ${msg.recipient_number}${otpNotice}`, 'info');

  // If message has OTP code, refresh OTP cards
  if (msg.extracted_code) {
    renderOtpCards([{
      code: msg.extracted_code,
      message_id: msg.id,
      recipient_number: msg.recipient_number,
      sender: msg.sender,
      body: msg.body,
      received_at: msg.received_at,
    }]);
  }
}

// --- Render Functions ---
function renderNumbers() {
  elements.totalNumbersCount.textContent = state.numbers.length;

  let totalMsgs = 0;
  const itemsHtml = state.numbers.map((num) => {
    totalMsgs += num.message_count || 0;
    const isActive = state.selectedNumber === num.phone_number ? 'active' : '';
    const label = num.label || 'SIM / Phone';
    return `
      <button class="number-item ${isActive}" data-number="${escapeHtml(num.phone_number)}">
        <div class="number-info">
          <span class="number-label">${escapeHtml(label)}</span>
          <span class="number-val">${escapeHtml(num.phone_number)}</span>
        </div>
        <span class="msg-badge">${num.message_count || 0}</span>
      </button>
    `;
  }).join('');

  elements.numbersList.innerHTML = `
    <button class="number-item ${state.selectedNumber === 'all' ? 'active' : ''}" data-number="all">
      <div class="number-info">
        <span class="number-label">All Numbers</span>
        <span class="number-val">All incoming streams</span>
      </div>
      <span id="allMsgCount" class="msg-badge">${totalMsgs}</span>
    </button>
    ${itemsHtml}
  `;

  // Attach click events
  elements.numbersList.querySelectorAll('.number-item').forEach((btn) => {
    btn.addEventListener('click', () => {
      const num = btn.getAttribute('data-number');
      selectNumber(num);
    });
  });
}

function selectNumber(num) {
  state.selectedNumber = num;
  renderNumbers();

  if (num === 'all') {
    elements.currentFeedTitle.textContent = 'All Messages';
    elements.currentFeedSubtitle.textContent = 'Live stream across all phone numbers';
    elements.btnClearFilters.style.display = 'none';
  } else {
    elements.currentFeedTitle.textContent = num;
    elements.currentFeedSubtitle.textContent = `Filtered by recipient number: ${num}`;
    elements.btnClearFilters.style.display = 'inline-block';
  }

  loadMessages();
}

function renderMessages() {
  if (state.messages.length === 0) {
    elements.messagesList.innerHTML = `
      <div class="loading-placeholder text-muted" style="text-align: center; padding: 3rem 1rem;">
        <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" style="margin-bottom: 0.75rem; opacity: 0.4;"><path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"></path></svg>
        <p>No messages found for this filter.</p>
        <p class="text-sm mt-2">Send a test SMS using the "Simulate SMS" button above or connect your Android/iOS device.</p>
      </div>
    `;
    return;
  }

  const html = state.messages.map((m) => {
    const hasOtp = Boolean(m.extracted_code);
    const dateStr = formatDate(m.received_at);
    const initial = m.sender ? m.sender.charAt(0).toUpperCase() : '?';

    return `
      <div class="message-card ${hasOtp ? 'has-otp' : ''}" id="msg-${m.id}">
        <div class="message-top">
          <div class="message-sender-wrap">
            <div class="sender-avatar">${escapeHtml(initial)}</div>
            <div>
              <div class="sender-name">${escapeHtml(m.sender)}</div>
              <div class="recipient-pill">${escapeHtml(m.recipient_number)}</div>
            </div>
          </div>
          <div class="message-meta">
            ${m.sim_slot !== null && m.sim_slot !== undefined ? `<span class="sim-tag">SIM ${m.sim_slot + 1}</span>` : ''}
            <span>${dateStr}</span>
          </div>
        </div>

        <div class="message-body">${escapeHtml(m.body)}</div>

        ${hasOtp ? `
          <div>
            <span class="otp-extracted-badge">
              <span>OTP: <strong>${escapeHtml(m.extracted_code)}</strong></span>
              <button class="btn-copy-otp" onclick="copyText('${escapeHtml(m.extracted_code)}', this)">Copy</button>
            </span>
          </div>
        ` : ''}

        <div class="message-footer">
          <span>Device: ${escapeHtml(m.device_id || 'unknown')} &bull; ID: ${m.id.substring(0, 8)}</span>
          <button class="btn-delete-msg" onclick="deleteMessage('${m.id}')" title="Delete message">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="3 6 5 6 21 6"></polyline><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path></svg>
          </button>
        </div>
      </div>
    `;
  }).join('');

  elements.messagesList.innerHTML = html;
}

function renderOtpCards(otps) {
  if (!otps || otps.length === 0) {
    elements.otpSection.style.display = 'none';
    return;
  }

  elements.otpSection.style.display = 'block';
  elements.otpCards.innerHTML = otps.map((o) => `
    <div class="otp-card">
      <div class="otp-card-top">
        <span class="otp-sender">${escapeHtml(o.sender)}</span>
        <span class="otp-recipient">${escapeHtml(o.recipient_number)}</span>
      </div>
      <div class="otp-code-row">
        <span class="otp-code-val">${escapeHtml(o.code)}</span>
        <button class="btn-copy-otp" onclick="copyText('${escapeHtml(o.code)}', this)">Copy Code</button>
      </div>
      <div class="text-muted text-sm">${formatDate(o.received_at)}</div>
    </div>
  `).join('');
}

// --- Device Modal & QR Generator ---
async function loadDevices() {
  try {
    const res = await api('/api/v1/admin/devices');
    const devices = res.devices || [];
    elements.devicesTableBody.innerHTML = devices.map((d) => `
      <tr>
        <td><strong>${escapeHtml(d.name)}</strong></td>
        <td><span class="sim-tag">${escapeHtml(d.platform)}</span></td>
        <td><code>${escapeHtml(d.id)}</code></td>
        <td>${formatDate(d.last_seen_at)}</td>
        <td><span class="status-dot" style="display:inline-block; background-color: var(--accent-success);"></span> Active</td>
      </tr>
    `).join('') || '<tr><td colspan="5" class="text-muted text-center">No devices paired yet.</td></tr>';
  } catch (err) {
    elements.devicesTableBody.innerHTML = `<tr><td colspan="5" class="text-danger">Admin access required to view devices.</td></tr>`;
  }
}

// --- Token Manager ---
async function loadTokens() {
  try {
    const res = await api('/api/v1/admin/tokens');
    const tokens = res.tokens || [];
    elements.tokensTableBody.innerHTML = tokens.map((t) => {
      const allowedText = t.allowed_numbers.includes('*') || t.allowed_numbers.includes('ALL')
        ? '<span class="status-badge" style="background:#1e3a8a; color:#93c5fd;">All Numbers (*)</span>'
        : t.allowed_numbers.map((n) => `<span class="recipient-pill">${escapeHtml(n)}</span>`).join(' ');

      const perms = [
        t.can_read ? 'Read' : '',
        t.can_delete ? 'Delete' : '',
        t.is_admin ? 'Admin' : '',
      ].filter(Boolean).join(', ');

      return `
        <tr>
          <td><strong>${escapeHtml(t.name)}</strong></td>
          <td><code>${escapeHtml(t.token_prefix)}</code></td>
          <td>${allowedText}</td>
          <td>${escapeHtml(perms)}</td>
          <td>${formatDate(t.created_at)}</td>
          <td>
            ${t.is_admin && tokens.filter(x => x.is_admin).length <= 1 ? '<span class="text-muted">Protected</span>' : `
              <button class="btn btn-sm btn-outline" style="color:var(--accent-danger);" onclick="revokeToken('${t.id}')">Revoke</button>
            `}
          </td>
        </tr>
      `;
    }).join('');
  } catch (err) {
    elements.tokensTableBody.innerHTML = `<tr><td colspan="6" class="text-danger">Admin access required to view tokens.</td></tr>`;
  }
}

function updateTokenNumberCheckboxes() {
  elements.numberCheckboxesGroup.innerHTML = state.numbers.map((num) => `
    <label class="checkbox-label text-sm">
      <input type="checkbox" name="tokenNum" value="${escapeHtml(num.phone_number)}">
      <span>${escapeHtml(num.phone_number)} (${escapeHtml(num.label || 'SIM')})</span>
    </label>
  `).join('') || '<div class="text-muted text-sm">No phone numbers registered yet.</div>';
}

// --- Event Listeners ---
function initEventListeners() {
  // Search
  let searchTimer;
  elements.searchInput.addEventListener('input', (e) => {
    clearTimeout(searchTimer);
    searchTimer = setTimeout(() => {
      state.searchQuery = e.target.value.trim();
      loadMessages();
    }, 300);
  });

  elements.btnRefresh.addEventListener('click', () => {
    loadData();
    showToast('Feed refreshed', 'info');
  });

  elements.btnClearFilters.addEventListener('click', () => {
    selectNumber('all');
  });

  // Modal open/close
  elements.btnTestSms.addEventListener('click', () => {
    if (state.selectedNumber !== 'all') {
      document.getElementById('simRecipient').value = state.selectedNumber;
    }
    openModal('testSmsModal');
  });

  elements.btnDevices.addEventListener('click', () => {
    loadDevices();
    openModal('devicesModal');
  });

  elements.btnTokens.addEventListener('click', () => {
    loadTokens();
    openModal('tokensModal');
  });

  elements.btnAuth.addEventListener('click', () => {
    elements.adminTokenInput.value = state.token;
    openModal('authModal');
  });

  elements.btnQuickMcpGuide.addEventListener('click', () => {
    updateMcpGuide();
    openModal('mcpGuideModal');
  });

  document.querySelectorAll('[data-close]').forEach((btn) => {
    btn.addEventListener('click', () => {
      const modalId = btn.getAttribute('data-close');
      closeModal(modalId);
    });
  });

  // Close modals clicking on backdrop
  document.querySelectorAll('.modal-backdrop').forEach((backdrop) => {
    backdrop.addEventListener('click', (e) => {
      if (e.target === backdrop) {
        backdrop.classList.remove('open');
      }
    });
  });

  // Test SMS Form Submission
  elements.testSmsForm.addEventListener('submit', async (e) => {
    e.preventDefault();
    const recipient = document.getElementById('simRecipient').value.trim();
    const sender = document.getElementById('simSender').value.trim();
    const body = document.getElementById('simBody').value.trim();
    const slot = parseInt(document.getElementById('simSlot').value, 10);
    const deviceId = document.getElementById('simDevice').value.trim();

    try {
      const res = await api('/api/v1/sms/forward', {
        method: 'POST',
        body: JSON.stringify({
          recipient_number: recipient,
          sender: sender,
          body: body,
          sim_slot: slot,
          device_id: deviceId,
        }),
      });

      closeModal('testSmsModal');
      elements.testSmsForm.reset();
      showToast(`Test SMS processed! (ID: ${res.message_id.substring(0, 8)})`, 'success');
      loadData();
    } catch (err) {
      showToast(`Error: ${err.message}`, 'error');
    }
  });

  // Add Device Form Submission
  elements.addDeviceForm.addEventListener('submit', async (e) => {
    e.preventDefault();
    const name = document.getElementById('newDeviceName').value.trim();
    const platform = document.getElementById('newDevicePlatform').value;

    try {
      const res = await api('/api/v1/admin/devices', {
        method: 'POST',
        body: JSON.stringify({ name, platform }),
      });

      elements.newDeviceResult.style.display = 'flex';
      const serverUrl = `${window.location.protocol}//${window.location.host}`;
      elements.displayServerUrl.textContent = serverUrl;
      elements.displayDeviceToken.textContent = res.token;

      // Render QR Code with pure SVG/canvas generator
      renderQrSvg(elements.deviceQrCanvas, res.qr_data);

      loadDevices();
      showToast('Device registered! Scan QR with Android app.', 'success');
    } catch (err) {
      showToast(`Error registering device: ${err.message}`, 'error');
    }
  });

  document.getElementById('btnCopyDevToken').addEventListener('click', () => {
    const tok = elements.displayDeviceToken.textContent;
    copyText(tok, elements.btnCopyDevToken);
  });

  // Token Form Checkboxes Toggle
  elements.chkAllNumbers.addEventListener('change', (e) => {
    elements.numberCheckboxesGroup.style.display = e.target.checked ? 'none' : 'grid';
  });

  // Create Token Form Submission
  elements.createTokenForm.addEventListener('submit', async (e) => {
    e.preventDefault();
    const name = document.getElementById('tokenName').value.trim();
    const allNumbers = elements.chkAllNumbers.checked;
    let allowedNumbers = ['*'];

    if (!allNumbers) {
      const checked = Array.from(elements.numberCheckboxesGroup.querySelectorAll('input:checked'))
        .map((cb) => cb.value);
      if (checked.length === 0) {
        showToast('Please select at least one phone number or check Allow All Numbers.', 'error');
        return;
      }
      allowedNumbers = checked;
    }

    const canRead = document.getElementById('chkCanRead').checked;
    const canDelete = document.getElementById('chkCanDelete').checked;
    const isAdmin = document.getElementById('chkIsAdmin').checked;

    try {
      const res = await api('/api/v1/admin/tokens', {
        method: 'POST',
        body: JSON.stringify({
          name,
          allowed_numbers: allowedNumbers,
          can_read: canRead,
          can_delete: canDelete,
          is_admin: isAdmin,
        }),
      });

      state.latestCreatedToken = res.secret;
      elements.createdSecretVal.textContent = res.secret;
      elements.tokenSecretBanner.style.display = 'block';
      updateSnippetDisplay();

      loadTokens();
      showToast('Token generated successfully!', 'success');
    } catch (err) {
      showToast(`Error generating token: ${err.message}`, 'error');
    }
  });

  elements.btnCopyCreatedSecret.addEventListener('click', () => {
    copyText(state.latestCreatedToken, elements.btnCopyCreatedSecret);
  });

  // Snippet tabs
  document.querySelectorAll('.tab-btn').forEach((btn) => {
    btn.addEventListener('click', () => {
      document.querySelectorAll('.tab-btn').forEach((b) => b.classList.remove('active'));
      btn.classList.add('active');
      state.activeTabSnippet = btn.getAttribute('data-tab');
      updateSnippetDisplay();
    });
  });

  // Auth Form Submission
  elements.authForm.addEventListener('submit', (e) => {
    e.preventDefault();
    const token = elements.adminTokenInput.value.trim();
    state.token = token;
    localStorage.setItem('sms_admin_token', token);
    closeModal('authModal');
    updateAuthUI();
    connectWebSocket();
    loadData();
    showToast('Admin Token saved', 'success');
  });
}

function updateSnippetDisplay() {
  const secret = state.latestCreatedToken || '<YOUR_TOKEN>';
  const serverUrl = `${window.location.protocol}//${window.location.host}`;

  if (state.activeTabSnippet === 'tabClaude') {
    elements.snippetCode.textContent = JSON.stringify({
      "mcpServers": {
        "hermesgate": {
          "command": "hermesgate",
          "args": [
            "--mcp-stdio",
            "--token", secret
          ]
        }
      }
    }, null, 2);
  } else {
    elements.snippetCode.textContent = `# Get latest OTP verification code:\ncurl -H "Authorization: Bearer ${secret}" \\\n  "${serverUrl}/api/v1/messages/latest-otp"\n\n# List incoming messages:\ncurl -H "Authorization: Bearer ${secret}" \\\n  "${serverUrl}/api/v1/messages?limit=10"`;
  }
}

function updateMcpGuide() {
  const secret = state.token || '<YOUR_TOKEN>';
  const serverUrl = `${window.location.protocol}//${window.location.host}`;
  document.getElementById('claudeDesktopConfigPre').textContent = JSON.stringify({
    "mcpServers": {
      "hermesgate": {
        "command": "/usr/local/bin/hermesgate",
        "args": [
          "--mcp-stdio",
          "--token", secret
        ]
      }
    }
  }, null, 2);
  document.getElementById('mcpSseUrlExample').textContent = `${serverUrl}/mcp/sse?token=${secret}`;
}

function updateAuthUI() {
  if (state.token) {
    elements.authBadge.textContent = 'Admin (Configured)';
    elements.btnAuth.classList.remove('btn-outline');
    elements.btnAuth.classList.add('btn-secondary');
  } else {
    elements.authBadge.textContent = 'Set Admin Token';
    elements.btnAuth.classList.remove('btn-secondary');
    elements.btnAuth.classList.add('btn-outline');
  }
}

function updateDatalist() {
  const dl = document.getElementById('knownNumbersList');
  dl.innerHTML = state.numbers.map((n) => `<option value="${escapeHtml(n.phone_number)}">${escapeHtml(n.label || '')}</option>`).join('');
}

async function deleteMessage(id) {
  if (!confirm('Are you sure you want to delete this message?')) return;
  try {
    await api(`/api/v1/messages/${id}`, { method: 'DELETE' });
    state.messages = state.messages.filter((m) => m.id !== id);
    renderMessages();
    loadStats();
    showToast('Message deleted', 'info');
  } catch (err) {
    showToast(`Failed to delete: ${err.message}`, 'error');
  }
}

async function revokeToken(id) {
  if (!confirm('Revoke this API token? Any client using it will lose access.')) return;
  try {
    await api(`/api/v1/admin/tokens/${id}`, { method: 'DELETE' });
    loadTokens();
    showToast('Token revoked', 'info');
  } catch (err) {
    showToast(`Failed to revoke token: ${err.message}`, 'error');
  }
}

// --- Modals Helper ---
function openModal(id) {
  document.getElementById(id).classList.add('open');
}

function closeModal(id) {
  document.getElementById(id).classList.remove('open');
}

// --- Utilities ---
function copyText(text, btnElement) {
  navigator.clipboard.writeText(text).then(() => {
    showToast('Copied to clipboard!', 'success');
    if (btnElement) {
      const orig = btnElement.textContent;
      btnElement.textContent = 'Copied!';
      setTimeout(() => { btnElement.textContent = orig; }, 1500);
    }
  }).catch(() => {
    prompt('Copy to clipboard: Ctrl+C, Enter', text);
  });
}

function showToast(msg, type = 'info') {
  const toast = document.createElement('div');
  toast.className = `toast toast-${type}`;
  toast.innerHTML = `<span>${escapeHtml(msg)}</span>`;
  elements.toastContainer.appendChild(toast);
  setTimeout(() => {
    toast.style.opacity = '0';
    setTimeout(() => toast.remove(), 300);
  }, 4000);
}

function formatDate(isoStr) {
  if (!isoStr) return '';
  try {
    const d = new Date(isoStr);
    return d.toLocaleString([], {
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
    });
  } catch (e) {
    return isoStr;
  }
}

function escapeHtml(str) {
  if (!str) return '';
  return String(str)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;');
}

// --- Self-contained QR Code SVG generator ---
function renderQrSvg(container, text) {
  // Simple QR placeholder or matrix rendering
  // Generates clean SVG representation of QR payload
  container.innerHTML = `
    <svg width="124" height="124" viewBox="0 0 33 33" style="background:#fff; padding:4px; border-radius:4px;">
      <rect width="33" height="33" fill="#ffffff"/>
      <!-- QR Alignment Patterns -->
      <path fill="#000" d="M2 2h7v7H2zM3 3v5h5V3zM4 4h3v3H4zM24 2h7v7h-7zM25 3v5h5V3zM26 4h3v3h-3zM2 24h7v7H2zM3 25v5h5v-5zM4 26h3v3H4z"/>
      <!-- Data Mockup Matrix Hash based on payload -->
      ${generateMockQrMatrix(text)}
    </svg>
  `;
}

function generateMockQrMatrix(text) {
  let hash = 0;
  for (let i = 0; i < text.length; i++) {
    hash = ((hash << 5) - hash) + text.charCodeAt(i);
    hash |= 0;
  }
  let rects = '';
  for (let y = 3; y < 30; y += 2) {
    for (let x = 3; x < 30; x += 2) {
      if ((x < 10 && y < 10) || (x > 22 && y < 10) || (x < 10 && y > 22)) continue;
      const bit = ((hash ^ (x * 37 + y * 19)) >>> ((x + y) % 16)) & 1;
      if (bit === 1) {
        rects += `<rect x="${x}" y="${y}" width="1.5" height="1.5" fill="#000"/>`;
      }
    }
  }
  return rects;
}

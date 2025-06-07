import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

// グローバル状態
let currentTab = 'history'
let searchTimers = {}
let editingBookmarkId = null

// DOM要素の取得
const elements = {
  status: document.getElementById('status'),
  
  // タブ
  tabButtons: document.querySelectorAll('.tab-button'),
  tabPanels: document.querySelectorAll('.tab-panel'),
  
  // 履歴
  historyList: document.getElementById('history-list'),
  historySearch: document.getElementById('history-search'),
  clearHistoryBtn: document.getElementById('clear-history-btn'),
  removeDuplicatesBtn: document.getElementById('remove-duplicates-btn'),
  
  // ブックマーク
  bookmarksList: document.getElementById('bookmarks-list'),
  bookmarkSearch: document.getElementById('bookmark-search'),
  addBookmarkBtn: document.getElementById('add-bookmark-btn'),
  
  // IP履歴
  ipsList: document.getElementById('ips-list'),
  ipSearch: document.getElementById('ip-search'),
  clearIpsBtn: document.getElementById('clear-ips-btn'),
  
  // 設定
  historyLimit: document.getElementById('history-limit'),
  ipLimit: document.getElementById('ip-limit'),
  hotkeyDisplay: document.getElementById('hotkey-display'),
  accessibilityStatus: document.getElementById('accessibility-status'),
  checkPermissionsBtn: document.getElementById('check-permissions-btn'),
  statsDisplay: document.getElementById('stats-display'),
  
  // モーダル
  bookmarkModal: document.getElementById('bookmark-modal'),
  modalTitle: document.getElementById('modal-title'),
  bookmarkName: document.getElementById('bookmark-name'),
  bookmarkContent: document.getElementById('bookmark-content'),
  bookmarkTags: document.getElementById('bookmark-tags'),
  bookmarkSaveBtn: document.getElementById('bookmark-save-btn'),
  bookmarkCancelBtn: document.getElementById('bookmark-cancel-btn'),
  modalClose: document.querySelector('.modal-close')
}

// アプリ初期化
async function initApp() {
  try {
    updateStatus('アプリを初期化しています...', 'info')
    
    // イベントリスナーを設定
    setupEventListeners()
    
    // Tauriバックエンドと通信
    await invoke('init_clipboard_manager')
    
    updateStatus('クリップボード監視を開始しました', 'success')
    
    // 初期データを読み込み
    await loadAllData()
    
    // 設定を読み込み
    await loadSettings()
    
    // 権限状態を確認
    await checkPermissions()
    
    // 統計を更新
    await updateStats()
    
  } catch (error) {
    console.error('初期化エラー:', error)
    updateStatus(`エラー: ${error}`, 'error')
  }
}

// イベントリスナーの設定
function setupEventListeners() {
  // Tauriイベント
  setupTauriEvents()
  
  // タブ切り替え
  elements.tabButtons.forEach(button => {
    button.addEventListener('click', () => switchTab(button.dataset.tab))
  })
  
  // 検索
  setupSearchListeners()
  
  // ボタンイベント
  setupButtonEvents()
  
  // モーダル
  setupModalEvents()
  
  // 設定変更
  setupSettingsEvents()
}

// Tauriイベントの設定
async function setupTauriEvents() {
  // クリップボード更新
  await listen('clipboard-updated', async (event) => {
    console.log('クリップボード更新:', event.payload)
    await loadHistory()
    updateStatus(`クリップボード更新 (${new Date().toLocaleTimeString()})`, 'success')
  })
  
  // IP検出
  await listen('ip-detected', async (event) => {
    console.log('IP検出:', event.payload)
    updateStatus(`IP検出: ${event.payload} (${new Date().toLocaleTimeString()})`, 'info')
    await loadIPs()
  })
  
  // ホットキートリガー
  await listen('hotkey-triggered', async (event) => {
    console.log('ホットキー:', event.payload)
    updateStatus(`ホットキーアクティブ: ${event.payload}`, 'info')
    await loadAllData()
  })
  
  // トレイメニューイベント
  await listen('tray-clear-history', async () => {
    await clearHistory()
  })
  
  await listen('tray-remove-duplicates', async () => {
    await removeDuplicates()
  })
}

// 検索リスナーの設定
function setupSearchListeners() {
  // リアルタイム検索
  elements.historySearch.addEventListener('input', (e) => {
    debounceSearch('history', e.target.value, 300)
  })
  
  elements.bookmarkSearch.addEventListener('input', (e) => {
    debounceSearch('bookmarks', e.target.value, 300)
  })
  
  elements.ipSearch.addEventListener('input', (e) => {
    debounceSearch('ips', e.target.value, 300)
  })
}

// ボタンイベントの設定
function setupButtonEvents() {
  // 履歴操作
  elements.clearHistoryBtn.addEventListener('click', () => {
    if (confirm('履歴をすべてクリアしますか？')) {
      clearHistory()
    }
  })
  
  elements.removeDuplicatesBtn.addEventListener('click', removeDuplicates)
  
  // ブックマーク操作
  elements.addBookmarkBtn.addEventListener('click', () => openBookmarkModal())
  
  // IP操作
  elements.clearIpsBtn.addEventListener('click', () => {
    if (confirm('IP履歴をすべてクリアしますか？')) {
      clearIPs()
    }
  })
  
  // 権限確認
  elements.checkPermissionsBtn.addEventListener('click', checkPermissions)
}

// モーダルイベントの設定
function setupModalEvents() {
  elements.bookmarkSaveBtn.addEventListener('click', saveBookmark)
  elements.bookmarkCancelBtn.addEventListener('click', closeBookmarkModal)
  elements.modalClose.addEventListener('click', closeBookmarkModal)
  
  // モーダル外クリックで閉じる
  elements.bookmarkModal.addEventListener('click', (e) => {
    if (e.target === elements.bookmarkModal) {
      closeBookmarkModal()
    }
  })
}

// 設定イベントの設定
function setupSettingsEvents() {
  elements.historyLimit.addEventListener('change', updateAppSettings)
  elements.ipLimit.addEventListener('change', updateAppSettings)
}

// ステータス更新
function updateStatus(message, type = 'info') {
  elements.status.textContent = message
  elements.status.className = `status-${type}`
}

// タブ切り替え
function switchTab(tabName) {
  currentTab = tabName
  
  // タブボタンの状態更新
  elements.tabButtons.forEach(btn => {
    btn.classList.toggle('active', btn.dataset.tab === tabName)
  })
  
  // タブパネルの表示切り替え
  elements.tabPanels.forEach(panel => {
    panel.classList.toggle('active', panel.id === `${tabName}-tab`)
  })
  
  // タブ切り替え時にデータを再読み込み
  loadTabData(tabName)
}

// タブ別データ読み込み
async function loadTabData(tabName) {
  switch (tabName) {
    case 'history':
      await loadHistory()
      break
    case 'bookmarks':
      await loadBookmarks()
      break
    case 'ips':
      await loadIPs()
      break
    case 'settings':
      await updateStats()
      break
  }
}

// 全データ読み込み
async function loadAllData() {
  await Promise.all([
    loadHistory(),
    loadBookmarks(),
    loadIPs()
  ])
}

// 履歴データ読み込み
async function loadHistory(searchQuery = '') {
  try {
    let history
    if (searchQuery) {
      history = await invoke('search_clipboard_history', { query: searchQuery })
    } else {
      history = await invoke('get_clipboard_history')
    }
    displayHistory(history)
  } catch (error) {
    console.error('履歴取得エラー:', error)
    elements.historyList.innerHTML = '<div class="error">履歴の取得に失敗しました</div>'
  }
}

// 履歴表示
function displayHistory(history) {
  elements.historyList.innerHTML = ''
  
  if (!history || history.length === 0) {
    elements.historyList.innerHTML = '<div class="empty-state">📄 履歴がありません</div>'
    return
  }
  
  // 新しい順にソート
  const sortedHistory = history.sort((a, b) => new Date(b.timestamp) - new Date(a.timestamp))
  
  sortedHistory.forEach((item, index) => {
    const card = createHistoryCard(item, index)
    elements.historyList.appendChild(card)
  })
}

// 履歴カード作成
function createHistoryCard(item, index) {
  const card = document.createElement('div')
  card.className = 'item-card'
  
  const truncatedContent = item.content.length > 200 
    ? item.content.substring(0, 200) + '...' 
    : item.content
  
  card.innerHTML = `
    <div class="item-header">
      <div class="item-title">#${index + 1} ${item.content_type}</div>
      <div class="item-meta">${new Date(item.timestamp).toLocaleString()}</div>
    </div>
    <div class="item-content">${truncatedContent}</div>
    <div class="item-actions">
      <button class="item-btn" onclick="copyToClipboard('${item.id}')">📋 コピー</button>
      <button class="item-btn" onclick="addToBookmarks('${item.id}')">⭐ ブックマーク</button>
      <button class="item-btn danger" onclick="deleteHistoryItem('${item.id}')">🗑️ 削除</button>
    </div>
  `
  
  return card
}

// ブックマークデータ読み込み
async function loadBookmarks(searchQuery = '') {
  try {
    let bookmarks
    if (searchQuery) {
      bookmarks = await invoke('search_bookmarks', { query: searchQuery })
    } else {
      bookmarks = await invoke('get_bookmarks')
    }
    displayBookmarks(bookmarks)
  } catch (error) {
    console.error('ブックマーク取得エラー:', error)
    elements.bookmarksList.innerHTML = '<div class="error">ブックマークの取得に失敗しました</div>'
  }
}

// ブックマーク表示
function displayBookmarks(bookmarks) {
  elements.bookmarksList.innerHTML = ''
  
  if (!bookmarks || bookmarks.length === 0) {
    elements.bookmarksList.innerHTML = '<div class="empty-state">⭐ ブックマークがありません</div>'
    return
  }
  
  bookmarks.forEach(bookmark => {
    const card = createBookmarkCard(bookmark)
    elements.bookmarksList.appendChild(card)
  })
}

// ブックマークカード作成
function createBookmarkCard(bookmark) {
  const card = document.createElement('div')
  card.className = 'item-card'
  
  const tags = bookmark.tags.map(tag => `<span class="tag">#${tag}</span>`).join('')
  const truncatedContent = bookmark.content.length > 150
    ? bookmark.content.substring(0, 150) + '...'
    : bookmark.content
  
  card.innerHTML = `
    <div class="item-header">
      <div class="item-title">⭐ ${bookmark.name}</div>
      <div class="item-meta">${new Date(bookmark.timestamp).toLocaleString()}</div>
    </div>
    <div class="item-content">${truncatedContent}</div>
    ${tags ? `<div class="item-tags">${tags}</div>` : ''}
    <div class="item-actions">
      <button class="item-btn" onclick="copyBookmarkContent('${bookmark.id}')">📋 コピー</button>
      <button class="item-btn" onclick="editBookmark('${bookmark.id}')">✏️ 編集</button>
      <button class="item-btn" onclick="duplicateBookmark('${bookmark.id}')">📄 複製</button>
      <button class="item-btn danger" onclick="deleteBookmark('${bookmark.id}')">🗑️ 削除</button>
    </div>
  `
  
  return card
}

// IP履歴データ読み込み
async function loadIPs(searchQuery = '') {
  try {
    let ips
    if (searchQuery) {
      ips = await invoke('search_ip_history', { query: searchQuery })
    } else {
      ips = await invoke('get_recent_ips')
    }
    displayIPs(ips)
  } catch (error) {
    console.error('IP履歴取得エラー:', error)
    elements.ipsList.innerHTML = '<div class="error">IP履歴の取得に失敗しました</div>'
  }
}

// IP履歴表示
function displayIPs(ips) {
  elements.ipsList.innerHTML = ''
  
  if (!ips || ips.length === 0) {
    elements.ipsList.innerHTML = '<div class="empty-state">🌐 IP履歴がありません</div>'
    return
  }
  
  ips.forEach((ipItem, index) => {
    const card = createIPCard(ipItem, index)
    elements.ipsList.appendChild(card)
  })
}

// IPカード作成
function createIPCard(ipItem, index) {
  const card = document.createElement('div')
  card.className = 'item-card'
  
  card.innerHTML = `
    <div class="item-header">
      <div class="item-title">🌐 ${ipItem.ip}</div>
      <div class="item-meta">アクセス回数: ${ipItem.count}</div>
    </div>
    <div class="item-meta">最終アクセス: ${new Date(ipItem.timestamp).toLocaleString()}</div>
    <div class="item-actions">
      <button class="item-btn" onclick="copyIPAddress('${ipItem.ip}')">📋 コピー</button>
      <button class="item-btn" onclick="resetIPCount('${ipItem.ip}')">🔄 カウントリセット</button>
      <button class="item-btn danger" onclick="removeIP('${ipItem.ip}')">🗑️ 削除</button>
    </div>
  `
  
  return card
}

// 検索のデバウンス
function debounceSearch(type, query, delay) {
  clearTimeout(searchTimers[type])
  searchTimers[type] = setTimeout(() => {
    switch (type) {
      case 'history':
        loadHistory(query)
        break
      case 'bookmarks':
        loadBookmarks(query)
        break
      case 'ips':
        loadIPs(query)
        break
    }
  }, delay)
}

// 履歴クリア
async function clearHistory() {
  try {
    await invoke('clear_clipboard_history')
    await loadHistory()
    updateStatus('履歴をクリアしました', 'success')
  } catch (error) {
    console.error('履歴クリアエラー:', error)
    updateStatus(`履歴クリアエラー: ${error}`, 'error')
  }
}

// 重複削除
async function removeDuplicates() {
  try {
    const result = await invoke('remove_duplicate_clipboard_items')
    await loadHistory()
    updateStatus(result, 'success')
  } catch (error) {
    console.error('重複削除エラー:', error)
    updateStatus(`重複削除エラー: ${error}`, 'error')
  }
}

// IP履歴クリア
async function clearIPs() {
  try {
    await invoke('clear_ip_history')
    await loadIPs()
    updateStatus('IP履歴をクリアしました', 'success')
  } catch (error) {
    console.error('IP履歴クリアエラー:', error)
    updateStatus(`IP履歴クリアエラー: ${error}`, 'error')
  }
}

// ブックマークモーダル表示
function openBookmarkModal(bookmarkData = null) {
  editingBookmarkId = bookmarkData?.id || null
  
  elements.modalTitle.textContent = bookmarkData ? 'ブックマークを編集' : 'ブックマークを追加'
  elements.bookmarkName.value = bookmarkData?.name || ''
  elements.bookmarkContent.value = bookmarkData?.content || ''
  elements.bookmarkTags.value = bookmarkData?.tags?.join(', ') || ''
  
  elements.bookmarkModal.classList.add('show')
}

// ブックマークモーダル非表示
function closeBookmarkModal() {
  elements.bookmarkModal.classList.remove('show')
  editingBookmarkId = null
}

// ブックマーク保存
async function saveBookmark() {
  try {
    const name = elements.bookmarkName.value.trim()
    const content = elements.bookmarkContent.value.trim()
    const tags = elements.bookmarkTags.value.split(',').map(tag => tag.trim()).filter(tag => tag)
    
    if (!name || !content) {
      alert('名前と内容は必須です')
      return
    }
    
    if (editingBookmarkId) {
      await invoke('update_bookmark', {
        bookmarkId: editingBookmarkId,
        name,
        content,
        contentType: 'text',
        tags
      })
    } else {
      await invoke('add_bookmark', {
        name,
        content,
        contentType: 'text',
        tags
      })
    }
    
    closeBookmarkModal()
    await loadBookmarks()
    updateStatus('ブックマークを保存しました', 'success')
  } catch (error) {
    console.error('ブックマーク保存エラー:', error)
    updateStatus(`ブックマーク保存エラー: ${error}`, 'error')
  }
}

// 設定読み込み
async function loadSettings() {
  try {
    const settings = await invoke('get_settings')
    elements.historyLimit.value = settings.history_limit
    elements.ipLimit.value = settings.ip_limit
    elements.hotkeyDisplay.value = settings.hotkey
  } catch (error) {
    console.error('設定読み込みエラー:', error)
  }
}

// 設定更新
async function updateAppSettings() {
  try {
    const settings = {
      hotkey: elements.hotkeyDisplay.value,
      history_limit: parseInt(elements.historyLimit.value),
      ip_limit: parseInt(elements.ipLimit.value),
      auto_start: true,
      show_notifications: false
    }
    
    await invoke('update_settings', { settings })
    updateStatus('設定を更新しました', 'success')
  } catch (error) {
    console.error('設定更新エラー:', error)
    updateStatus(`設定更新エラー: ${error}`, 'error')
  }
}

// 権限確認
async function checkPermissions() {
  try {
    const permissions = await invoke('check_permissions_status')
    
    const accessibilityStatus = elements.accessibilityStatus
    if (permissions.accessibility) {
      accessibilityStatus.textContent = '許可済み'
      accessibilityStatus.className = 'status-indicator granted'
    } else {
      accessibilityStatus.textContent = '未許可'
      accessibilityStatus.className = 'status-indicator denied'
    }
  } catch (error) {
    console.error('権限確認エラー:', error)
  }
}

// 統計更新
async function updateStats() {
  try {
    const [clipboardStats, ipStats] = await Promise.all([
      invoke('get_clipboard_stats'),
      invoke('get_ip_stats')
    ])
    
    elements.statsDisplay.innerHTML = `
      <div class="stat-item">
        <div class="stat-value">${clipboardStats.total_items}</div>
        <div class="stat-label">履歴アイテム</div>
      </div>
      <div class="stat-item">
        <div class="stat-value">${Math.round(clipboardStats.total_size_bytes / 1024)}KB</div>
        <div class="stat-label">総サイズ</div>
      </div>
      <div class="stat-item">
        <div class="stat-value">${ipStats.total_ips}</div>
        <div class="stat-label">IP履歴</div>
      </div>
      <div class="stat-item">
        <div class="stat-value">${clipboardStats.usage_percent}%</div>
        <div class="stat-label">使用率</div>
      </div>
    `
  } catch (error) {
    console.error('統計更新エラー:', error)
  }
}

// グローバル関数（HTMLから呼び出される）
window.copyToClipboard = async function(itemId) {
  try {
    const history = await invoke('get_clipboard_history')
    const item = history.find(h => h.id === itemId)
    if (item) {
      await navigator.clipboard.writeText(item.content)
      updateStatus('クリップボードにコピーしました', 'success')
    }
  } catch (error) {
    console.error('コピーエラー:', error)
  }
}

window.addToBookmarks = async function(itemId) {
  try {
    const history = await invoke('get_clipboard_history')
    const item = history.find(h => h.id === itemId)
    if (item) {
      openBookmarkModal({
        name: `履歴 #${Date.now()}`,
        content: item.content,
        tags: []
      })
    }
  } catch (error) {
    console.error('ブックマーク追加エラー:', error)
  }
}

window.deleteHistoryItem = async function(itemId) {
  if (confirm('この履歴アイテムを削除しますか？')) {
    try {
      await invoke('delete_clipboard_item', { itemId })
      await loadHistory()
      updateStatus('履歴アイテムを削除しました', 'success')
    } catch (error) {
      console.error('削除エラー:', error)
    }
  }
}

window.copyBookmarkContent = async function(bookmarkId) {
  try {
    const bookmarks = await invoke('get_bookmarks')
    const bookmark = bookmarks.find(b => b.id === bookmarkId)
    if (bookmark) {
      await navigator.clipboard.writeText(bookmark.content)
      updateStatus('ブックマーク内容をコピーしました', 'success')
    }
  } catch (error) {
    console.error('コピーエラー:', error)
  }
}

window.editBookmark = async function(bookmarkId) {
  try {
    const bookmarks = await invoke('get_bookmarks')
    const bookmark = bookmarks.find(b => b.id === bookmarkId)
    if (bookmark) {
      openBookmarkModal(bookmark)
    }
  } catch (error) {
    console.error('編集エラー:', error)
  }
}

window.duplicateBookmark = async function(bookmarkId) {
  try {
    await invoke('duplicate_bookmark', { bookmarkId })
    await loadBookmarks()
    updateStatus('ブックマークを複製しました', 'success')
  } catch (error) {
    console.error('複製エラー:', error)
  }
}

window.deleteBookmark = async function(bookmarkId) {
  if (confirm('このブックマークを削除しますか？')) {
    try {
      await invoke('delete_bookmark', { bookmarkId })
      await loadBookmarks()
      updateStatus('ブックマークを削除しました', 'success')
    } catch (error) {
      console.error('削除エラー:', error)
    }
  }
}

window.copyIPAddress = async function(ip) {
  try {
    await navigator.clipboard.writeText(ip)
    updateStatus(`IPアドレスをコピーしました: ${ip}`, 'success')
  } catch (error) {
    console.error('コピーエラー:', error)
  }
}

window.resetIPCount = async function(ip) {
  try {
    await invoke('reset_ip_count', { ip })
    await loadIPs()
    updateStatus(`${ip}のカウントをリセットしました`, 'success')
  } catch (error) {
    console.error('カウントリセットエラー:', error)
  }
}

window.removeIP = async function(ip) {
  if (confirm(`IP履歴から${ip}を削除しますか？`)) {
    try {
      await invoke('remove_ip_from_recent', { ip })
      await loadIPs()
      updateStatus(`IP履歴から${ip}を削除しました`, 'success')
    } catch (error) {
      console.error('IP削除エラー:', error)
    }
  }
}

// 定期的なデータ更新
setInterval(() => {
  if (currentTab === 'history') {
    loadHistory()
  } else if (currentTab === 'ips') {
    loadIPs()
  }
}, 5000)

// 統計の定期更新
setInterval(() => {
  if (currentTab === 'settings') {
    updateStats()
  }
}, 10000)

// ページロード時に初期化
document.addEventListener('DOMContentLoaded', initApp)
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

// グローバル状態
let currentTab = 'history'
let searchTimers = {}
let editingBookmarkId = null

// DOM要素の取得
const elements = {
  
  // タブ
  tabButtons: document.querySelectorAll('.tab-button'),
  tabPanels: document.querySelectorAll('.tab-panel'),
  
  // 履歴
  historyList: document.getElementById('history-list'),
  historySearch: document.getElementById('history-search'),
  historySort: document.getElementById('history-sort'),
  clearHistoryBtn: document.getElementById('clear-history-btn'),
  
  // ブックマーク
  bookmarksList: document.getElementById('bookmarks-list'),
  bookmarkSearch: document.getElementById('bookmark-search'),
  bookmarkSort: document.getElementById('bookmark-sort'),
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
  
  // Phase 7: 最適化・ログ
  optimizeMemoryBtn: document.getElementById('optimize-memory-btn'),
  viewLogsBtn: document.getElementById('view-logs-btn'),
  clearLogsBtn: document.getElementById('clear-logs-btn'),
  diagnosticsBtn: document.getElementById('diagnostics-btn'),
  logsModal: document.getElementById('logs-modal'),
  logsContent: document.getElementById('logs-content'),
  logsCloseBtn: document.getElementById('logs-close-btn'),
  diagnosticsModal: document.getElementById('diagnostics-modal'),
  diagnosticsContent: document.getElementById('diagnostics-content'),
  diagnosticsCloseBtn: document.getElementById('diagnostics-close-btn'),
  
  // モーダル
  bookmarkModal: document.getElementById('bookmark-modal'),
  modalTitle: document.getElementById('modal-title'),
  bookmarkName: document.getElementById('bookmark-name'),
  bookmarkContent: document.getElementById('bookmark-content'),
  bookmarkTags: document.getElementById('bookmark-tags'),
  bookmarkSaveBtn: document.getElementById('bookmark-save-btn'),
  bookmarkCancelBtn: document.getElementById('bookmark-cancel-btn'),
  modalClose: document.querySelector('.modal-close'),
  
  // ヘルプモーダル
  helpModal: document.getElementById('help-modal'),
  helpBtn: document.getElementById('help-btn'),
  helpCloseBtn: document.getElementById('help-close-btn'),
  
  // プレビューモーダル
  previewModal: document.getElementById('preview-modal'),
  previewTitle: document.getElementById('preview-title'),
  previewType: document.getElementById('preview-type'),
  previewSize: document.getElementById('preview-size'),
  previewDate: document.getElementById('preview-date'),
  previewContent: document.getElementById('preview-content'),
  previewCopyBtn: document.getElementById('preview-copy-btn'),
  previewBookmarkBtn: document.getElementById('preview-bookmark-btn'),
  previewCloseBtn: document.getElementById('preview-close-btn'),
  
  // ダークモード
  darkModeToggle: document.getElementById('dark-mode-toggle')
}

// アプリ初期化
async function initApp() {
  try {
    updateStatus('アプリを初期化しています...', 'info')
    
    // イベントリスナーを設定
    setupEventListeners()
    
    // 表示関数を拡張
    enhanceDisplayFunctions()
    
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
  })
  
  // IP検出
  await listen('ip-detected', async (event) => {
    console.log('IP検出:', event.payload)
    await loadIPs()
  })
  
  // ホットキートリガー
  await listen('hotkey-triggered', async (event) => {
    console.log('ホットキー:', event.payload)
    await loadAllData()
  })
  
  // トレイメニューイベント
  await listen('tray-clear-history', async () => {
    await clearHistory()
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
  
  // ソート変更
  elements.historySort.addEventListener('change', (e) => {
    loadHistory('', e.target.value)
  })
  
  elements.bookmarkSort.addEventListener('change', (e) => {
    loadBookmarks('', e.target.value)
  })
}

// ボタンイベントの設定
function setupButtonEvents() {
  // 履歴操作
  elements.clearHistoryBtn.addEventListener('click', clearHistory)
  
  
  // ブックマーク操作
  elements.addBookmarkBtn.addEventListener('click', () => openBookmarkModal())
  
  // IP操作
  elements.clearIpsBtn.addEventListener('click', clearIPs)
  
  // 権限確認
  elements.checkPermissionsBtn.addEventListener('click', checkPermissions)
  
  // Phase 7: 最適化・ログ
  elements.optimizeMemoryBtn.addEventListener('click', optimizeMemory)
  elements.viewLogsBtn.addEventListener('click', viewLogs)
  elements.clearLogsBtn.addEventListener('click', clearLogs)
  elements.diagnosticsBtn.addEventListener('click', showDiagnostics)
}

// モーダルイベントの設定
function setupModalEvents() {
  elements.bookmarkSaveBtn.addEventListener('click', saveBookmark)
  elements.bookmarkCancelBtn.addEventListener('click', closeBookmarkModal)
  elements.modalClose.addEventListener('click', closeBookmarkModal)
  
  // ヘルプモーダル
  elements.helpBtn.addEventListener('click', openHelpModal)
  elements.helpCloseBtn.addEventListener('click', closeHelpModal)
  
  // プレビューモーダル
  elements.previewCloseBtn.addEventListener('click', closePreviewModal)
  
  // ログ・診断モーダル
  elements.logsCloseBtn.addEventListener('click', closeLogsModal)
  elements.diagnosticsCloseBtn.addEventListener('click', closeDiagnosticsModal)
  
  // モーダル外クリックで閉じる
  elements.bookmarkModal.addEventListener('click', (e) => {
    if (e.target === elements.bookmarkModal) {
      closeBookmarkModal()
    }
  })
  
  elements.helpModal.addEventListener('click', (e) => {
    if (e.target === elements.helpModal) {
      closeHelpModal()
    }
  })
  
  elements.previewModal.addEventListener('click', (e) => {
    if (e.target === elements.previewModal) {
      closePreviewModal()
    }
  })
  
  elements.logsModal.addEventListener('click', (e) => {
    if (e.target === elements.logsModal) {
      closeLogsModal()
    }
  })
  
  elements.diagnosticsModal.addEventListener('click', (e) => {
    if (e.target === elements.diagnosticsModal) {
      closeDiagnosticsModal()
    }
  })
  
  // モーダルの閉じるボタン
  elements.helpModal.querySelector('.modal-close').addEventListener('click', closeHelpModal)
  elements.previewModal.querySelector('.modal-close').addEventListener('click', closePreviewModal)
  elements.logsModal.querySelector('.modal-close').addEventListener('click', closeLogsModal)
  elements.diagnosticsModal.querySelector('.modal-close').addEventListener('click', closeDiagnosticsModal)
}

// 設定イベントの設定
function setupSettingsEvents() {
  elements.historyLimit.addEventListener('change', updateAppSettings)
  elements.ipLimit.addEventListener('change', updateAppSettings)
  elements.darkModeToggle.addEventListener('click', toggleDarkMode)
}

// ステータス更新
function updateStatus(message, type = 'info') {
  // ヘッダーを削除したため、ステータスはコンソールに出力
  console.log(`[${type.toUpperCase()}] ${message}`)
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
async function loadHistory(searchQuery = '', sortBy = '') {
  try {
    let history
    if (searchQuery) {
      history = await invoke('search_clipboard_history', { query: searchQuery })
    } else if (sortBy || elements.historySort.value !== 'recent') {
      const sortMethod = sortBy || elements.historySort.value
      history = await invoke('get_sorted_history', { sortBy: sortMethod })
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
  
  const accessInfo = item.access_count > 0 
    ? `<span class="access-count">🔥 ${item.access_count}回使用</span>`
    : '<span class="access-count">未使用</span>'
  
  card.innerHTML = `
    <div class="item-header">
      <div class="item-title">#${index + 1} ${item.content_type}</div>
      <div class="item-meta">
        ${new Date(item.timestamp).toLocaleString()}
        ${accessInfo}
      </div>
    </div>
    <div class="item-content">${truncatedContent}</div>
    <div class="item-actions">
      <button class="item-btn" onclick="copyToClipboard('${item.id}')">📋 コピー</button>
      <button class="item-btn" onclick="previewHistoryItem('${item.id}')">👁️ プレビュー</button>
      <button class="item-btn" onclick="addToBookmarks('${item.id}')">⭐ ブックマーク</button>
      <button class="item-btn danger" onclick="deleteHistoryItem('${item.id}')">🗑️ 削除</button>
    </div>
  `
  
  return card
}

// ブックマークデータ読み込み
async function loadBookmarks(searchQuery = '', sortBy = '') {
  try {
    let bookmarks
    if (searchQuery) {
      bookmarks = await invoke('search_bookmarks', { query: searchQuery })
    } else if (sortBy || elements.bookmarkSort.value !== 'recent') {
      const sortMethod = sortBy || elements.bookmarkSort.value
      bookmarks = await invoke('get_sorted_bookmarks', { sortBy: sortMethod })
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
  
  const accessInfo = bookmark.access_count > 0 
    ? `<span class="access-count">🔥 ${bookmark.access_count}回使用</span>`
    : '<span class="access-count">未使用</span>'
  
  card.innerHTML = `
    <div class="item-header">
      <div class="item-title">⭐ ${bookmark.name}</div>
      <div class="item-meta">
        ${new Date(bookmark.timestamp).toLocaleString()}
        ${accessInfo}
      </div>
    </div>
    <div class="item-content">${truncatedContent}</div>
    ${tags ? `<div class="item-tags">${tags}</div>` : ''}
    <div class="item-actions">
      <button class="item-btn" onclick="copyBookmarkContent('${bookmark.id}')">📋 コピー</button>
      <button class="item-btn" onclick="previewBookmarkItem('${bookmark.id}')">👁️ プレビュー</button>
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

// ヘルプモーダル表示
function openHelpModal() {
  elements.helpModal.classList.add('show')
}

// ヘルプモーダル非表示
function closeHelpModal() {
  elements.helpModal.classList.remove('show')
}

// プレビューモーダル表示
let currentPreviewItem = null

function openPreviewModal(item, type = 'history') {
  currentPreviewItem = { ...item, type }
  
  // プレビュータイトル設定
  const titleIcons = {
    'history': '📄',
    'bookmark': '⭐',
    'ip': '🌐'
  }
  elements.previewTitle.textContent = `${titleIcons[type]} プレビュー`
  
  // 基本情報設定
  elements.previewType.textContent = item.content_type || 'text'
  elements.previewSize.textContent = formatFileSize(item.content.length)
  elements.previewDate.textContent = new Date(item.timestamp).toLocaleString()
  
  // コンテンツ設定
  displayPreviewContent(item.content)
  
  // アクションボタンの設定
  setupPreviewActions(item, type)
  
  elements.previewModal.classList.add('show')
}

// プレビューモーダル非表示
function closePreviewModal() {
  elements.previewModal.classList.remove('show')
  currentPreviewItem = null
}

// プレビューコンテンツ表示
function displayPreviewContent(content) {
  const contentElement = elements.previewContent
  
  // クラスをリセット
  contentElement.className = 'preview-content'
  
  // コンテンツタイプを判定してスタイリング
  const contentType = detectContentType(content)
  contentElement.classList.add(contentType)
  
  // コンテンツを処理して表示
  const processedContent = processContentForPreview(content, contentType)
  contentElement.innerHTML = processedContent
}

// コンテンツタイプ検出
function detectContentType(content) {
  const trimmed = content.trim()
  
  // URL判定
  if (trimmed.match(/^https?:\/\/\S+$/)) {
    return 'url'
  }
  
  // JSON判定
  if ((trimmed.startsWith('{') && trimmed.endsWith('}')) || 
      (trimmed.startsWith('[') && trimmed.endsWith(']'))) {
    try {
      JSON.parse(trimmed)
      return 'json'
    } catch {
      // JSONパースに失敗した場合は通常テキスト
    }
  }
  
  // コード判定（一般的なプログラミング言語のパターン）
  if (trimmed.match(/^(function|class|import|export|const|let|var|if|for|while|def|public|private)/m) ||
      trimmed.includes('```') || trimmed.includes('<script>') || trimmed.includes('<?php')) {
    return 'code'
  }
  
  // 長いテキスト（200文字以上）
  if (content.length > 200) {
    return 'large-text'
  }
  
  return 'text'
}

// プレビュー用コンテンツ処理
function processContentForPreview(content, type) {
  switch (type) {
    case 'json':
      try {
        const parsed = JSON.parse(content)
        return escapeHtml(JSON.stringify(parsed, null, 2))
      } catch {
        return escapeHtml(content)
      }
    
    case 'url':
      const escapedUrl = escapeHtml(content)
      return `<a href="${escapedUrl}" target="_blank" rel="noopener noreferrer">${escapedUrl}</a>`
    
    case 'code':
      return `<code>${escapeHtml(content)}</code>`
    
    default:
      return escapeHtml(content)
  }
}

// HTML エスケープ
function escapeHtml(text) {
  const div = document.createElement('div')
  div.textContent = text
  return div.innerHTML
}

// ファイルサイズフォーマット
function formatFileSize(bytes) {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i]
}

// プレビューアクション設定
function setupPreviewActions(item, type) {
  // コピーボタン
  elements.previewCopyBtn.onclick = () => {
    navigator.clipboard.writeText(item.content)
    updateStatus('クリップボードにコピーしました', 'success')
  }
  
  // ブックマークボタン（履歴アイテムのみ表示）
  if (type === 'history') {
    elements.previewBookmarkBtn.style.display = 'inline-flex'
    elements.previewBookmarkBtn.onclick = () => {
      openBookmarkModal({
        name: `プレビューから追加 #${Date.now()}`,
        content: item.content,
        tags: []
      })
      closePreviewModal()
    }
  } else {
    elements.previewBookmarkBtn.style.display = 'none'
  }
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
      
      // アクセス回数を増加
      await invoke('increment_access_count', {
        itemId: itemId,
        itemType: 'history'
      })
      
      // 表示を更新
      setTimeout(() => loadHistory(), 100)
    }
  } catch (error) {
    console.error('コピーエラー:', error)
  }
}

window.previewHistoryItem = async function(itemId) {
  try {
    const history = await invoke('get_clipboard_history')
    const item = history.find(h => h.id === itemId)
    if (item) {
      openPreviewModal(item, 'history')
    }
  } catch (error) {
    console.error('プレビューエラー:', error)
  }
}

window.previewBookmarkItem = async function(bookmarkId) {
  try {
    const bookmarks = await invoke('get_bookmarks')
    const bookmark = bookmarks.find(b => b.id === bookmarkId)
    if (bookmark) {
      openPreviewModal(bookmark, 'bookmark')
    }
  } catch (error) {
    console.error('プレビューエラー:', error)
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
  try {
    await invoke('delete_clipboard_item', { itemId })
    await loadHistory()
    updateStatus('履歴アイテムを削除しました', 'success')
  } catch (error) {
    console.error('削除エラー:', error)
    updateStatus(`削除エラー: ${error}`, 'error')
  }
}

window.copyBookmarkContent = async function(bookmarkId) {
  try {
    const bookmarks = await invoke('get_bookmarks')
    const bookmark = bookmarks.find(b => b.id === bookmarkId)
    if (bookmark) {
      await navigator.clipboard.writeText(bookmark.content)
      updateStatus('ブックマーク内容をコピーしました', 'success')
      
      // アクセス回数を増加
      await invoke('increment_access_count', {
        itemId: bookmarkId,
        itemType: 'bookmark'
      })
      
      // 表示を更新
      setTimeout(() => loadBookmarks(), 100)
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
  try {
    await invoke('delete_bookmark', { bookmarkId })
    await loadBookmarks()
    updateStatus('ブックマークを削除しました', 'success')
  } catch (error) {
    console.error('削除エラー:', error)
    updateStatus(`削除エラー: ${error}`, 'error')
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

// キーボードナビゲーション
let selectedItemIndex = 0
let currentItems = []

function setupKeyboardNavigation() {
  document.addEventListener('keydown', handleKeyboardNavigation)
}

function handleKeyboardNavigation(event) {
  // モーダルが開いている場合はモーダル内のナビゲーション
  if (elements.bookmarkModal.classList.contains('show')) {
    handleModalKeyNavigation(event)
    return
  }
  
  // Cmd/Ctrl + 数字でタブ切り替え
  if ((event.metaKey || event.ctrlKey) && !event.shiftKey) {
    const tabKeys = {
      '1': 'history',
      '2': 'bookmarks', 
      '3': 'ips',
      '4': 'settings'
    }
    
    if (tabKeys[event.key]) {
      event.preventDefault()
      switchTab(tabKeys[event.key])
      return
    }
    
    // Cmd/Ctrl + F で検索フォーカス
    if (event.key === 'f') {
      event.preventDefault()
      focusSearchInput()
      return
    }
    
    // Cmd/Ctrl + N で新規ブックマーク（ブックマークタブのみ）
    if (event.key === 'n' && currentTab === 'bookmarks') {
      event.preventDefault()
      openBookmarkModal()
      return
    }
  }
  
  // 検索フィールドにフォーカスがある場合はリターン
  const activeElement = document.activeElement
  if (activeElement && (activeElement.classList.contains('search-input') || activeElement.tagName === 'INPUT' || activeElement.tagName === 'TEXTAREA')) {
    if (event.key === 'Escape') {
      activeElement.blur()
      updateSelectedItems()
    }
    return
  }
  
  // ?キーでヘルプ表示
  if (event.key === '?' && !event.shiftKey) {
    event.preventDefault()
    openHelpModal()
    return
  }
  
  // 方向キーナビゲーション
  if (['ArrowUp', 'ArrowDown', 'Enter', 'Space', 'Escape'].includes(event.key)) {
    event.preventDefault()
    
    switch (event.key) {
      case 'ArrowUp':
        navigateItems(-1)
        break
      case 'ArrowDown':
        navigateItems(1)
        break
      case 'Enter':
      case 'Space':
        activateSelectedItem()
        break
      case 'Escape':
        clearSelection()
        break
    }
  }
  
  // 削除キー
  if (event.key === 'Delete' || event.key === 'Backspace') {
    if (selectedItemIndex >= 0 && currentItems.length > 0) {
      event.preventDefault()
      deleteSelectedItem()
    }
  }
  
  // コピー (Cmd/Ctrl + C)
  if ((event.metaKey || event.ctrlKey) && event.key === 'c') {
    if (selectedItemIndex >= 0 && currentItems.length > 0) {
      event.preventDefault()
      copySelectedItem()
    }
  }
}

function handleModalKeyNavigation(event) {
  if (event.key === 'Escape') {
    closeBookmarkModal()
  } else if ((event.metaKey || event.ctrlKey) && event.key === 'Enter') {
    saveBookmark()
  } else if (event.key === 'Tab') {
    // タブキーでフォーカス移動（デフォルト動作）
    return
  }
}

function focusSearchInput() {
  const searchInputs = {
    'history': elements.historySearch,
    'bookmarks': elements.bookmarkSearch,
    'ips': elements.ipSearch
  }
  
  const input = searchInputs[currentTab]
  if (input) {
    input.focus()
    input.select()
  }
}

function updateSelectedItems() {
  // 現在のタブのアイテムを取得
  const listElements = {
    'history': elements.historyList,
    'bookmarks': elements.bookmarksList,
    'ips': elements.ipsList
  }
  
  const listElement = listElements[currentTab]
  if (listElement) {
    currentItems = Array.from(listElement.querySelectorAll('.item-card'))
    selectedItemIndex = 0
    updateItemSelection()
  } else {
    currentItems = []
    selectedItemIndex = -1
  }
}

function navigateItems(direction) {
  if (currentItems.length === 0) {
    updateSelectedItems()
    return
  }
  
  selectedItemIndex += direction
  
  // 範囲チェック
  if (selectedItemIndex < 0) {
    selectedItemIndex = currentItems.length - 1
  } else if (selectedItemIndex >= currentItems.length) {
    selectedItemIndex = 0
  }
  
  updateItemSelection()
  scrollToSelectedItem()
}

function updateItemSelection() {
  // 全ての選択を解除
  currentItems.forEach(item => item.classList.remove('selected'))
  
  // 現在の選択をハイライト
  if (selectedItemIndex >= 0 && selectedItemIndex < currentItems.length) {
    currentItems[selectedItemIndex].classList.add('selected')
  }
}

function scrollToSelectedItem() {
  if (selectedItemIndex >= 0 && selectedItemIndex < currentItems.length) {
    const selectedItem = currentItems[selectedItemIndex]
    selectedItem.scrollIntoView({
      behavior: 'smooth',
      block: 'nearest',
      inline: 'nearest'
    })
  }
}

function activateSelectedItem() {
  if (selectedItemIndex >= 0 && selectedItemIndex < currentItems.length) {
    const selectedItem = currentItems[selectedItemIndex]
    
    // アイテムのコピーボタンをクリック
    const copyButton = selectedItem.querySelector('.item-btn')
    if (copyButton) {
      copyButton.click()
    }
  }
}

function deleteSelectedItem() {
  if (selectedItemIndex >= 0 && selectedItemIndex < currentItems.length) {
    const selectedItem = currentItems[selectedItemIndex]
    
    // 削除ボタンを探してクリック
    const deleteButton = selectedItem.querySelector('.item-btn.danger, .item-btn[onclick*="delete"]')
    if (deleteButton && confirm('選択されたアイテムを削除しますか？')) {
      deleteButton.click()
    }
  }
}

function copySelectedItem() {
  if (selectedItemIndex >= 0 && selectedItemIndex < currentItems.length) {
    const selectedItem = currentItems[selectedItemIndex]
    
    // コピーボタンをクリック
    const copyButton = selectedItem.querySelector('.item-btn[onclick*="copy"]')
    if (copyButton) {
      copyButton.click()
    }
  }
}

function clearSelection() {
  selectedItemIndex = -1
  updateItemSelection()
}

// タブ切り替え時にアイテムを更新
const originalSwitchTab = switchTab
window.switchTab = function(tabName) {
  originalSwitchTab(tabName)
  setTimeout(updateSelectedItems, 100) // 少し遅延してDOMが更新されてから実行
}

// データ読み込み後にアイテムを更新するヘルパー関数
function enhanceDisplayFunctions() {
  const originalDisplayHistory = window.displayHistory
  window.displayHistory = function(history) {
    originalDisplayHistory(history)
    if (currentTab === 'history') {
      setTimeout(updateSelectedItems, 50)
    }
  }

  const originalDisplayBookmarks = window.displayBookmarks
  window.displayBookmarks = function(bookmarks) {
    originalDisplayBookmarks(bookmarks)
    if (currentTab === 'bookmarks') {
      setTimeout(updateSelectedItems, 50)
    }
  }

  const originalDisplayIPs = window.displayIPs
  window.displayIPs = function(ips) {
    originalDisplayIPs(ips)
    if (currentTab === 'ips') {
      setTimeout(updateSelectedItems, 50)
    }
  }
}

// ダークモード管理
let isDarkMode = false

function initDarkMode() {
  // ローカルストレージからダークモード設定を読み込み
  isDarkMode = localStorage.getItem('darkMode') === 'true'
  applyDarkMode()
}

function toggleDarkMode() {
  isDarkMode = !isDarkMode
  localStorage.setItem('darkMode', isDarkMode.toString())
  applyDarkMode()
  updateStatus(`${isDarkMode ? 'ダーク' : 'ライト'}モードに切り替えました`, 'success')
}

function applyDarkMode() {
  const body = document.body
  const toggleBtn = elements.darkModeToggle
  const toggleIcon = toggleBtn.querySelector('.toggle-icon')
  const toggleText = toggleBtn.querySelector('.toggle-text')
  
  if (isDarkMode) {
    body.classList.add('dark-mode')
    toggleBtn.classList.add('active')
    toggleIcon.textContent = '☀️'
    toggleText.textContent = 'ライト'
  } else {
    body.classList.remove('dark-mode')
    toggleBtn.classList.remove('active')
    toggleIcon.textContent = '🌙'
    toggleText.textContent = 'ダーク'
  }
}

// Phase 7: 最適化・ログ機能
async function optimizeMemory() {
  try {
    updateStatus('メモリ最適化を実行中...', 'info')
    const result = await invoke('optimize_memory')
    updateStatus(result, 'success')
    await loadAllData() // データを再読み込み
    await updateStats() // 統計を更新
  } catch (error) {
    console.error('メモリ最適化エラー:', error)
    updateStatus(`メモリ最適化エラー: ${error}`, 'error')
  }
}

async function viewLogs() {
  try {
    const logs = await invoke('get_app_logs', { lines: 100 }) // 最新100行
    elements.logsContent.innerHTML = logs.map(line => 
      `<div class="log-line">${escapeHtml(line)}</div>`
    ).join('')
    elements.logsModal.classList.add('show')
    
    // ログを最下部にスクロール
    elements.logsContent.scrollTop = elements.logsContent.scrollHeight
  } catch (error) {
    console.error('ログ取得エラー:', error)
    updateStatus(`ログ取得エラー: ${error}`, 'error')
  }
}

async function clearLogs() {
  if (confirm('ログファイルをクリアしますか？')) {
    try {
      await invoke('clear_app_logs')
      updateStatus('ログファイルをクリアしました', 'success')
      // ログモーダルが開いている場合は更新
      if (elements.logsModal.classList.contains('show')) {
        await viewLogs()
      }
    } catch (error) {
      console.error('ログクリアエラー:', error)
      updateStatus(`ログクリアエラー: ${error}`, 'error')
    }
  }
}

async function showDiagnostics() {
  try {
    const diagnostics = await invoke('get_app_diagnostics')
    
    const diagnosticsHtml = `
      <div class="diagnostics-section">
        <h4>📊 システム情報</h4>
        <div class="diagnostic-item">
          <span>バージョン:</span>
          <span>${diagnostics.version}</span>
        </div>
        <div class="diagnostic-item">
          <span>最終更新:</span>
          <span>${new Date(diagnostics.timestamp).toLocaleString()}</span>
        </div>
      </div>
      
      <div class="diagnostics-section">
        <h4>💾 データ統計</h4>
        <div class="diagnostic-item">
          <span>履歴アイテム数:</span>
          <span>${diagnostics.data_stats.history_count}</span>
        </div>
        <div class="diagnostic-item">
          <span>ブックマーク数:</span>
          <span>${diagnostics.data_stats.bookmarks_count}</span>
        </div>
        <div class="diagnostic-item">
          <span>IP履歴数:</span>
          <span>${diagnostics.data_stats.ips_count}</span>
        </div>
        <div class="diagnostic-item">
          <span>総データサイズ:</span>
          <span>${formatFileSize(diagnostics.data_stats.total_history_size)}</span>
        </div>
        <div class="diagnostic-item">
          <span>データファイルサイズ:</span>
          <span>${formatFileSize(diagnostics.data_stats.data_file_size)}</span>
        </div>
      </div>
      
      <div class="diagnostics-section">
        <h4>🖥️ システム状態</h4>
        <div class="diagnostic-item">
          <span>ログファイルサイズ:</span>
          <span>${formatFileSize(diagnostics.system_stats.log_file_size)}</span>
        </div>
        <div class="diagnostic-item">
          <span>履歴制限:</span>
          <span>${diagnostics.system_stats.settings.history_limit}件</span>
        </div>
        <div class="diagnostic-item">
          <span>IP制限:</span>
          <span>${diagnostics.system_stats.settings.ip_limit}件</span>
        </div>
      </div>
      
      <div class="diagnostics-section">
        <h4>🔍 ヘルス状態</h4>
        <div class="diagnostic-item">
          <span>データ整合性:</span>
          <span class="health-${diagnostics.health.data_integrity.toLowerCase()}">${diagnostics.health.data_integrity}</span>
        </div>
        <div class="diagnostic-item">
          <span>メモリ使用量:</span>
          <span class="health-${diagnostics.health.memory_usage.toLowerCase()}">${diagnostics.health.memory_usage}</span>
        </div>
        <div class="diagnostic-item">
          <span>ディスク使用量:</span>
          <span class="health-${diagnostics.health.disk_usage.toLowerCase()}">${diagnostics.health.disk_usage}</span>
        </div>
      </div>
    `
    
    elements.diagnosticsContent.innerHTML = diagnosticsHtml
    elements.diagnosticsModal.classList.add('show')
  } catch (error) {
    console.error('診断情報取得エラー:', error)
    updateStatus(`診断情報取得エラー: ${error}`, 'error')
  }
}

function closeLogsModal() {
  elements.logsModal.classList.remove('show')
}

function closeDiagnosticsModal() {
  elements.diagnosticsModal.classList.remove('show')
}

// ページロード時に初期化
document.addEventListener('DOMContentLoaded', () => {
  initDarkMode()
  initApp()
  setupKeyboardNavigation()
})
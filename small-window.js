import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

// グローバル状態
let currentTab = 'history'
let appData = null

// DOM要素
const elements = {
  tabs: document.querySelectorAll('.small-tab'),
  historyList: document.getElementById('small-history'),
  bookmarksList: document.getElementById('small-bookmarks'),
  ipsList: document.getElementById('small-ips')
}

// 初期化
document.addEventListener('DOMContentLoaded', async () => {
  console.log('スモールウィンドウ初期化開始')
  
  // タブクリックイベント
  elements.tabs.forEach(tab => {
    tab.addEventListener('click', () => {
      const tabName = tab.dataset.tab
      switchTab(tabName)
    })
  })

  // ESCキーでウィンドウを閉じる
  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') {
      closeWindow()
    }
  })


  // データ読み込み
  await loadData()
  
  // リアルタイム更新のリスナー
  setupEventListeners()
  
  // フォーカス管理を設定
  setupFocusManagement()
})

// タブ切り替え
function switchTab(tabName) {
  currentTab = tabName
  
  // タブボタンの状態更新
  elements.tabs.forEach(tab => {
    tab.classList.toggle('active', tab.dataset.tab === tabName)
  })
  
  // コンテンツの表示切り替え
  elements.historyList.style.display = tabName === 'history' ? 'block' : 'none'
  elements.bookmarksList.style.display = tabName === 'bookmarks' ? 'block' : 'none'
  elements.ipsList.style.display = tabName === 'ips' ? 'block' : 'none'
  
  // データ表示更新
  displayCurrentTab()
}

// データ読み込み
async function loadData() {
  try {
    appData = await invoke('get_app_data')
    console.log('データ読み込み完了:', appData)
    displayCurrentTab()
  } catch (error) {
    console.error('データ読み込みエラー:', error)
    showError('データの読み込みに失敗しました')
  }
}

// 現在のタブに応じてデータ表示
function displayCurrentTab() {
  switch (currentTab) {
    case 'history':
      displayHistory()
      break
    case 'bookmarks':
      displayBookmarks()
      break
    case 'ips':
      displayIPs()
      break
  }
}

// 履歴表示
function displayHistory() {
  if (!appData || !appData.history) {
    showEmptyState(elements.historyList, '📋', '履歴がありません')
    return
  }

  // 最新順にソート（最大15件）
  const recentItems = appData.history
    .sort((a, b) => new Date(b.timestamp) - new Date(a.timestamp))
    .slice(0, 15)

  if (recentItems.length === 0) {
    showEmptyState(elements.historyList, '📋', '履歴がありません')
    return
  }

  const html = recentItems.map(item => {
    const preview = item.content.length > 50 
      ? item.content.substring(0, 50) + '...' 
      : item.content
    
    const timeAgo = getTimeAgo(item.timestamp)
    const size = formatFileSize(item.size || item.content.length)
    
    return `
      <div class="small-item" onclick="copyAndPaste('${escapeHtml(item.content)}')" title="${escapeHtml(item.content)}">
        <span class="small-item-icon">📄</span>
        <div class="small-item-content">
          <div class="small-item-text">${escapeHtml(preview)}</div>
          <div class="small-item-meta">${timeAgo}</div>
        </div>
        <span class="small-item-size">${size}</span>
      </div>
    `
  }).join('')

  elements.historyList.innerHTML = html
}

// ブックマーク表示
function displayBookmarks() {
  if (!appData || !appData.bookmarks) {
    showEmptyState(elements.bookmarksList, '⭐', 'ブックマークがありません')
    return
  }

  // 使用頻度順にソート（最大10件）
  const topBookmarks = appData.bookmarks
    .sort((a, b) => (b.access_count || 0) - (a.access_count || 0))
    .slice(0, 10)

  if (topBookmarks.length === 0) {
    showEmptyState(elements.bookmarksList, '⭐', 'ブックマークがありません')
    return
  }

  const html = topBookmarks.map(bookmark => {
    const preview = bookmark.content.length > 40 
      ? bookmark.content.substring(0, 40) + '...' 
      : bookmark.content
    
    const accessCount = bookmark.access_count || 0
    
    return `
      <div class="small-item" onclick="copyAndPaste('${escapeHtml(bookmark.content)}')" title="${escapeHtml(bookmark.content)}">
        <span class="small-item-icon">⭐</span>
        <div class="small-item-content">
          <div class="small-item-text">${escapeHtml(bookmark.name)}</div>
          <div class="small-item-meta">${escapeHtml(preview)}</div>
        </div>
        <span class="small-item-size">${accessCount}回</span>
      </div>
    `
  }).join('')

  elements.bookmarksList.innerHTML = html
}

// IP履歴表示
function displayIPs() {
  if (!appData || !appData.recent_ips) {
    showEmptyState(elements.ipsList, '🌐', 'IP履歴がありません')
    return
  }

  // 使用回数順にソート
  const sortedIPs = appData.recent_ips
    .sort((a, b) => (b.count || 0) - (a.count || 0))

  if (sortedIPs.length === 0) {
    showEmptyState(elements.ipsList, '🌐', 'IP履歴がありません')
    return
  }

  const html = sortedIPs.map(ipItem => {
    const timeAgo = getTimeAgo(ipItem.timestamp)
    const count = ipItem.count || 0
    
    return `
      <div class="small-item" onclick="copyAndPaste('${ipItem.ip}')" title="${ipItem.ip}">
        <span class="small-item-icon">🌐</span>
        <div class="small-item-content">
          <div class="small-item-text">${ipItem.ip}</div>
          <div class="small-item-meta">${timeAgo}</div>
        </div>
        <span class="small-item-size">${count}回</span>
      </div>
    `
  }).join('')

  elements.ipsList.innerHTML = html
}

// コピー＆貼り付け
async function copyAndPaste(content) {
  try {
    console.log('コピー＆貼り付け:', content.substring(0, 50))
    
    // アクセス回数を増加
    await invoke('increment_access_count', { content })
    
    // ウィンドウを閉じる
    await closeWindow()
    
    // 少し待ってから貼り付け（ウィンドウが閉じるのを待つ）
    setTimeout(async () => {
      try {
        await invoke('paste_content', { content })
        console.log('貼り付け処理完了')
      } catch (error) {
        console.error('貼り付けエラー:', error)
        // 貼り付けに失敗した場合はクリップボードにコピーだけ実行
        await invoke('add_clipboard_item', { content })
      }
    }, 200)
    
  } catch (error) {
    console.error('コピー＆貼り付けエラー:', error)
  }
}

// ウィンドウを閉じる
async function closeWindow() {
  try {
    await invoke('hide_small_window')
  } catch (error) {
    console.error('ウィンドウクローズエラー:', error)
  }
}

// 空の状態表示
function showEmptyState(container, icon, message) {
  container.innerHTML = `
    <div class="empty-state">
      <div class="empty-icon">${icon}</div>
      <div>${message}</div>
    </div>
  `
}

// エラー表示
function showError(message) {
  const container = currentTab === 'history' ? elements.historyList 
    : currentTab === 'bookmarks' ? elements.bookmarksList 
    : elements.ipsList
    
  container.innerHTML = `
    <div class="empty-state">
      <div class="empty-icon">⚠️</div>
      <div>${message}</div>
    </div>
  `
}

// イベントリスナー設定
function setupEventListeners() {
  try {
    // クリップボード更新イベント
    listen('clipboard-updated', async () => {
      console.log('クリップボード更新検出')
      await loadData()
    })
  } catch (error) {
    console.warn('イベントリスナー設定をスキップ:', error)
    // イベントリスナーが設定できない場合は定期的にデータを更新
    setInterval(loadData, 5000) // 5秒ごとに更新
  }
}

// ユーティリティ関数
function escapeHtml(text) {
  if (typeof text !== 'string') return ''
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;')
}

function getTimeAgo(timestamp) {
  const now = new Date()
  const time = new Date(timestamp)
  const diffMs = now - time
  const diffMins = Math.floor(diffMs / (1000 * 60))
  const diffHours = Math.floor(diffMs / (1000 * 60 * 60))
  const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24))

  if (diffMins < 1) return 'たった今'
  if (diffMins < 60) return `${diffMins}分前`
  if (diffHours < 24) return `${diffHours}時間前`
  if (diffDays < 7) return `${diffDays}日前`
  return time.toLocaleDateString('ja-JP')
}

function formatFileSize(bytes) {
  if (bytes < 1024) return `${bytes}B`
  if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)}KB`
  return `${Math.round(bytes / (1024 * 1024))}MB`
}

// フォーカス管理機能
function setupFocusManagement() {
  // ウィンドウフォーカス失ったら閉じる
  window.addEventListener('blur', () => {
    console.log('ウィンドウがフォーカスを失いました')
    setTimeout(() => {
      closeWindow()
    }, 100) // 少し遅延してから閉じる
  })
  
  // ウィンドウ外クリックでも閉じる（既存機能を残す）
  document.addEventListener('click', (e) => {
    if (!e.target.closest('.small-container')) {
      closeWindow()
    }
  })
}

// グローバル関数として公開
window.copyAndPaste = copyAndPaste
window.closeWindow = closeWindow
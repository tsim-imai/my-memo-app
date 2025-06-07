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

  // ウィンドウ外クリックで閉じる（グローバルで追跡する変数が必要）
  window.isDragging = false
  
  document.addEventListener('click', (e) => {
    // ドラッグ中は無視
    if (window.isDragging) return
    
    // small-containerの外をクリックした場合
    if (!e.target.closest('.small-container')) {
      closeWindow()
    }
  })

  // データ読み込み
  await loadData()
  
  // リアルタイム更新のリスナー
  setupEventListeners()
  
  // ドラッグ移動機能を設定
  setupWindowDrag()
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

// ドラッグ移動機能
function setupWindowDrag() {
  const header = document.querySelector('.small-header')
  let isDragging = false
  let dragOffset = { x: 0, y: 0 }
  let windowPosition = { x: 0, y: 0 }

  // 初期ウィンドウ位置を取得
  const initPosition = async () => {
    try {
      const position = await invoke('get_window_position')
      windowPosition = position
    } catch (error) {
      console.warn('初期位置取得失敗:', error)
    }
  }

  header.addEventListener('mousedown', async (e) => {
    // クローズボタンやタブのクリックは無視
    if (e.target.closest('.close-button') || e.target.closest('.small-tab')) {
      return
    }
    
    isDragging = true
    window.isDragging = true
    
    // 現在のウィンドウ位置を取得
    try {
      windowPosition = await invoke('get_window_position')
    } catch (error) {
      console.warn('位置取得失敗:', error)
    }
    
    // ドラッグ開始点を記録
    dragOffset.x = e.clientX
    dragOffset.y = e.clientY
    
    // マウスカーソルを変更
    document.body.style.cursor = 'grabbing'
    header.style.cursor = 'grabbing'
    
    e.preventDefault()
    e.stopPropagation()
  })

  document.addEventListener('mousemove', async (e) => {
    if (!isDragging) return
    
    try {
      // マウスの移動量を計算
      const deltaX = e.clientX - dragOffset.x
      const deltaY = e.clientY - dragOffset.y
      
      // 新しいウィンドウ位置を計算
      const newX = windowPosition.x + deltaX
      const newY = windowPosition.y + deltaY
      
      // 画面境界を取得して制限
      const screenBounds = await invoke('get_screen_bounds')
      const windowWidth = 400 // ウィンドウ幅
      const windowHeight = 500 // ウィンドウ高さ
      
      // 画面内に制限
      const boundedX = Math.max(0, Math.min(newX, screenBounds.width - windowWidth))
      const boundedY = Math.max(0, Math.min(newY, screenBounds.height - windowHeight))
      
      // ウィンドウ位置を更新
      await invoke('set_window_position', {
        x: boundedX,
        y: boundedY
      })
      
      // 新しい位置を記録
      windowPosition.x = boundedX
      windowPosition.y = boundedY
      dragOffset.x = e.clientX
      dragOffset.y = e.clientY
      
    } catch (error) {
      console.error('ウィンドウ移動エラー:', error)
    }
    
    e.preventDefault()
    e.stopPropagation()
  })

  document.addEventListener('mouseup', (e) => {
    if (isDragging) {
      isDragging = false
      window.isDragging = false
      document.body.style.cursor = ''
      header.style.cursor = 'move'
    }
    e.preventDefault()
    e.stopPropagation()
  })

  // 初期化
  initPosition()
}

// グローバル関数として公開
window.copyAndPaste = copyAndPaste
window.closeWindow = closeWindow
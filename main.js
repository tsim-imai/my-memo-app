import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

let status = document.getElementById('status')
let historyDiv = document.getElementById('history')
let ipsDiv = document.getElementById('recent-ips')

async function initApp() {
  try {
    status.textContent = 'アプリを初期化しています...'
    
    // クリップボード更新イベントをリッスン
    await listen('clipboard-updated', async (event) => {
      console.log('クリップボード更新:', event.payload)
      await loadHistory()
      status.textContent = `クリップボード監視中 (最新: ${new Date().toLocaleTimeString()})`
    })
    
    // IP検出イベントをリッスン
    await listen('ip-detected', async (event) => {
      console.log('IP検出:', event.payload)
      status.textContent = `IP検出: ${event.payload} (${new Date().toLocaleTimeString()})`
      await loadRecentIPs()
    })
    
    // Tauriバックエンドと通信
    await invoke('init_clipboard_manager')
    
    status.textContent = 'クリップボード監視を開始しました'
    
    // 履歴を取得して表示
    await loadHistory()
    
    // IP履歴を取得して表示
    await loadRecentIPs()
    
  } catch (error) {
    console.error('初期化エラー:', error)
    status.textContent = `エラー: ${error}`
  }
}

async function loadHistory() {
  try {
    const history = await invoke('get_clipboard_history')
    displayHistory(history)
  } catch (error) {
    console.error('履歴取得エラー:', error)
  }
}

function displayHistory(history) {
  historyDiv.innerHTML = ''
  
  if (!history || history.length === 0) {
    historyDiv.innerHTML = '<p>履歴がありません</p>'
    return
  }
  
  history.forEach((item, index) => {
    const div = document.createElement('div')
    div.style.cssText = `
      padding: 10px;
      margin: 5px 0;
      border: 1px solid #ddd;
      border-radius: 4px;
      background: #f9f9f9;
    `
    
    div.innerHTML = `
      <strong>#${index + 1}</strong> (${item.content_type})
      <br>
      <span style="font-size: 12px; color: #666;">${new Date(item.timestamp).toLocaleString()}</span>
      <br>
      <div style="margin-top: 5px; font-family: monospace; background: white; padding: 5px; border-radius: 2px;">
        ${item.content.substring(0, 100)}${item.content.length > 100 ? '...' : ''}
      </div>
    `
    
    historyDiv.appendChild(div)
  })
}

// ページロード時に初期化
document.addEventListener('DOMContentLoaded', initApp)

async function loadRecentIPs() {
  try {
    const recentIPs = await invoke('get_recent_ips')
    displayRecentIPs(recentIPs)
  } catch (error) {
    console.error('IP履歴取得エラー:', error)
  }
}

function displayRecentIPs(ips) {
  if (!ipsDiv) {
    console.warn('recent-ips要素が見つかりません')
    return
  }
  
  ipsDiv.innerHTML = ''
  
  if (!ips || ips.length === 0) {
    ipsDiv.innerHTML = '<h3>最近のIP履歴</h3><p>IP履歴がありません</p>'
    return
  }
  
  const title = document.createElement('h3')
  title.textContent = '最近のIP履歴'
  ipsDiv.appendChild(title)
  
  ips.forEach((ipItem, index) => {
    const div = document.createElement('div')
    div.style.cssText = `
      padding: 8px;
      margin: 3px 0;
      border: 1px solid #ccc;
      border-radius: 3px;
      background: #f0f8ff;
    `
    
    div.innerHTML = `
      <strong>#${index + 1} ${ipItem.ip}</strong> (count: ${ipItem.count})
      <br>
      <span style="font-size: 11px; color: #666;">${new Date(ipItem.timestamp).toLocaleString()}</span>
    `
    
    ipsDiv.appendChild(div)
  })
}

// 定期的に履歴を更新
setInterval(() => {
  loadHistory()
  loadRecentIPs()
}, 2000)
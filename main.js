import { invoke } from '@tauri-apps/api/core'

let status = document.getElementById('status')
let historyDiv = document.getElementById('history')

async function initApp() {
  try {
    status.textContent = 'アプリを初期化しています...'
    
    // Tauriバックエンドと通信
    await invoke('init_clipboard_manager')
    
    status.textContent = 'クリップボード監視を開始しました'
    
    // 履歴を取得して表示
    await loadHistory()
    
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
      <strong>#${index + 1}</strong> (${item.type})
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

// 定期的に履歴を更新
setInterval(loadHistory, 2000)
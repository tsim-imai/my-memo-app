use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardItem {
    pub id: String,
    pub content: String,
    pub content_type: String,
    pub timestamp: DateTime<Utc>,
    pub size: usize,
    #[serde(default)]
    pub access_count: u32,
    #[serde(default)]
    pub last_accessed: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookmarkItem {
    pub id: String,
    pub name: String,
    pub content: String,
    pub content_type: String,
    pub timestamp: DateTime<Utc>,
    pub tags: Vec<String>,
    #[serde(default)]
    pub access_count: u32,
    #[serde(default)]
    pub last_accessed: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpHistoryItem {
    pub ip: String,
    pub timestamp: DateTime<Utc>,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub hotkey: String,
    pub history_limit: usize,
    pub ip_limit: usize,
    pub auto_start: bool,
    pub show_notifications: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            hotkey: "cmd+shift+v".to_string(),
            history_limit: 50,
            ip_limit: 10,
            auto_start: true,
            show_notifications: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppData {
    pub version: String,
    pub history: Vec<ClipboardItem>,
    pub bookmarks: Vec<BookmarkItem>,
    pub recent_ips: Vec<IpHistoryItem>,
    pub settings: AppSettings,
}

impl Default for AppData {
    fn default() -> Self {
        Self {
            version: "1.0.0".to_string(),
            history: Vec::new(),
            bookmarks: Vec::new(),
            recent_ips: Vec::new(),
            settings: AppSettings::default(),
        }
    }
}
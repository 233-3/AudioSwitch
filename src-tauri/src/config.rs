//! 本地配置的读写、校验与迁移。
//!
//! - 配置只保存「用户意图」：左键行为、参与循环的设备及其顺序与启用状态；
//! - 运行时的临时状态（当前图标、切换锁、缓存快照）绝不写入配置；
//! - 配置文件损坏时备份为 `config.corrupt-<时间戳>.json` 并回落到默认配置，
//!   保证程序总能启动；
//! - `version` 字段为未来的结构升级预留迁移入口。

use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// 当前配置结构版本。
pub const CONFIG_VERSION: u32 = 1;

/// 默认界面语言。
pub const DEFAULT_LANGUAGE: &str = "zh";
/// 默认界面主题。
pub const DEFAULT_THEME: &str = "light";

/// 支持的主题取值。
pub const THEMES: [&str; 2] = ["light", "dark"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConfiguredDevice {
    /// Windows 音频端点 ID，所有操作都以它为准，不使用名称或下标。
    pub endpoint_id: String,
    /// 最近一次成功枚举时看到的显示名称，仅用于设备缺失时提示用户。
    #[serde(default)]
    pub last_known_name: String,
    /// 是否参与循环切换。
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct AppConfig {
    pub version: u32,
    /// 左键单击托盘图标是否执行切换（false 表示打开主窗口）。
    pub left_click_switch: bool,
    /// 参与循环的设备，数组顺序即用户定义的循环顺序。
    pub devices: Vec<ConfiguredDevice>,
    /// 界面语言：`zh` / `en`（缺失或非法时回落到默认值）。
    #[serde(default = "default_language")]
    pub language: String,
    /// 界面主题：`light` / `dark`（缺失或非法时回落到默认值）。
    #[serde(default = "default_theme")]
    pub theme: String,
}

fn default_enabled() -> bool {
    true
}

fn default_language() -> String {
    DEFAULT_LANGUAGE.to_string()
}

fn default_theme() -> String {
    DEFAULT_THEME.to_string()
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION,
            // 左键单击默认打开主界面：避免误触切换影响正在播放的音频
            left_click_switch: false,
            devices: Vec::new(),
            language: default_language(),
            theme: default_theme(),
        }
    }
}

impl AppConfig {
    /// 规范化：去除重复端点、补全版本号、校验语言与主题取值。
    ///
    /// 返回被删除的重复端点数量，便于日志记录。
    pub fn normalize(&mut self) -> usize {
        if self.version == 0 {
            self.version = CONFIG_VERSION;
        }

        // 语言与主题必须落在已知取值内（旧配置文件、手工编辑都可能写出别的值）
        self.language = match crate::i18n::Lang::parse(&self.language) {
            Some(lang) => lang.as_key().to_string(),
            None => default_language(),
        };
        let theme = self.theme.trim().to_ascii_lowercase();
        self.theme = if THEMES.contains(&theme.as_str()) {
            theme
        } else {
            default_theme()
        };

        let mut seen = HashSet::new();
        let before = self.devices.len();
        self.devices.retain(|device| {
            let id = device.endpoint_id.trim();
            !id.is_empty() && seen.insert(device.endpoint_id.clone())
        });

        before - self.devices.len()
    }

    /// 解析后的界面语言（已由 [`AppConfig::normalize`] 保证合法）。
    pub fn lang(&self) -> crate::i18n::Lang {
        crate::i18n::Lang::parse(&self.language).unwrap_or(crate::i18n::Lang::Zh)
    }

    pub fn find(&self, endpoint_id: &str) -> Option<&ConfiguredDevice> {
        self.devices.iter().find(|d| d.endpoint_id == endpoint_id)
    }

    pub fn contains(&self, endpoint_id: &str) -> bool {
        self.find(endpoint_id).is_some()
    }

    /// 加入循环列表（追加到末尾）。已存在时返回 false。
    pub fn add_device(&mut self, endpoint_id: &str, name: &str) -> bool {
        if self.contains(endpoint_id) {
            return false;
        }
        self.devices.push(ConfiguredDevice {
            endpoint_id: endpoint_id.to_string(),
            last_known_name: name.to_string(),
            enabled: true,
        });
        true
    }

    /// 从循环列表移除。设备的断开不会走到这里，只有用户显式删除才会。
    pub fn remove_device(&mut self, endpoint_id: &str) -> bool {
        let before = self.devices.len();
        self.devices.retain(|d| d.endpoint_id != endpoint_id);
        self.devices.len() != before
    }

    pub fn set_enabled(&mut self, endpoint_id: &str, enabled: bool) -> bool {
        match self
            .devices
            .iter_mut()
            .find(|d| d.endpoint_id == endpoint_id)
        {
            Some(device) => {
                device.enabled = enabled;
                true
            }
            None => false,
        }
    }

    /// 上移/下移。`offset` 为 -1 或 1。
    pub fn move_device(&mut self, endpoint_id: &str, offset: i32) -> bool {
        let Some(index) = self
            .devices
            .iter()
            .position(|d| d.endpoint_id == endpoint_id)
        else {
            return false;
        };
        let target = index as i32 + offset;
        if target < 0 || target as usize >= self.devices.len() {
            return false;
        }
        self.devices.swap(index, target as usize);
        true
    }

    /// 按给定顺序重排（用于前端拖拽场景）。未出现在列表中的设备保持原有相对位置追加在末尾。
    pub fn reorder(&mut self, endpoint_ids: &[String]) {
        let mut ordered: Vec<ConfiguredDevice> = Vec::with_capacity(self.devices.len());
        for id in endpoint_ids {
            if let Some(pos) = self.devices.iter().position(|d| &d.endpoint_id == id) {
                ordered.push(self.devices.remove(pos));
            }
        }
        ordered.append(&mut self.devices);
        self.devices = ordered;
    }

    /// 记录设备最近一次被枚举到的名称。
    pub fn update_known_name(&mut self, endpoint_id: &str, name: &str) -> bool {
        match self
            .devices
            .iter_mut()
            .find(|d| d.endpoint_id == endpoint_id)
        {
            Some(device) if device.last_known_name != name => {
                device.last_known_name = name.to_string();
                true
            }
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigLoadStatus {
    /// 配置文件不存在，已创建默认配置
    Created,
    /// 正常读取
    Loaded,
    /// 文件损坏，已备份并恢复默认配置
    Recovered,
}

impl ConfigLoadStatus {
    pub fn as_key(self) -> &'static str {
        match self {
            ConfigLoadStatus::Created => "created",
            ConfigLoadStatus::Loaded => "loaded",
            ConfigLoadStatus::Recovered => "recovered",
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("写入配置文件失败：{0}")]
    Write(String),
    #[error("配置文件目录不可用：{0}")]
    Directory(String),
}

pub struct ConfigStore {
    path: PathBuf,
}

impl ConfigStore {
    pub fn new(dir: &Path) -> Self {
        Self {
            path: dir.join("config.json"),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 读取配置；任何异常都不会阻止程序启动。
    pub fn load(&self) -> (AppConfig, ConfigLoadStatus) {
        if !self.path.exists() {
            let config = AppConfig::default();
            if let Err(err) = self.save(&config) {
                log::warn!("创建默认配置文件失败：{err}");
            }
            return (config, ConfigLoadStatus::Created);
        }

        let raw = match fs::read_to_string(&self.path) {
            Ok(raw) => raw,
            Err(err) => {
                log::error!("读取配置文件失败，使用默认配置：{err}");
                return (AppConfig::default(), ConfigLoadStatus::Recovered);
            }
        };

        match serde_json::from_str::<AppConfig>(&raw) {
            Ok(mut config) => {
                let removed = config.normalize();
                if removed > 0 {
                    log::warn!("配置文件中有 {removed} 个重复端点，已自动去重");
                }
                if config.version > CONFIG_VERSION {
                    log::warn!(
                        "配置文件版本 {} 高于程序支持的 {}，将按当前版本读取",
                        config.version,
                        CONFIG_VERSION
                    );
                    config.version = CONFIG_VERSION;
                }
                (config, ConfigLoadStatus::Loaded)
            }
            Err(err) => {
                log::error!("配置文件解析失败，将备份并恢复默认配置：{err}");
                self.backup_corrupt();
                let config = AppConfig::default();
                if let Err(save_err) = self.save(&config) {
                    log::error!("写入默认配置失败：{save_err}");
                }
                (config, ConfigLoadStatus::Recovered)
            }
        }
    }

    /// 原子写入：先写临时文件再替换，避免写入中途崩溃导致文件半截。
    pub fn save(&self, config: &AppConfig) -> Result<(), ConfigError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|e| ConfigError::Directory(e.to_string()))?;
        }

        let json = serde_json::to_string_pretty(config)
            .map_err(|e| ConfigError::Write(format!("序列化配置失败：{e}")))?;

        let tmp = self.path.with_extension("json.tmp");
        {
            let mut file = fs::File::create(&tmp).map_err(|e| ConfigError::Write(e.to_string()))?;
            file.write_all(json.as_bytes())
                .map_err(|e| ConfigError::Write(e.to_string()))?;
            file.sync_all()
                .map_err(|e| ConfigError::Write(e.to_string()))?;
        }

        fs::rename(&tmp, &self.path).map_err(|e| {
            let _ = fs::remove_file(&tmp);
            ConfigError::Write(e.to_string())
        })
    }

    fn backup_corrupt(&self) {
        let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
        let backup = self
            .path
            .with_file_name(format!("config.corrupt-{stamp}.json"));
        match fs::rename(&self.path, &backup) {
            Ok(()) => log::warn!("损坏的配置文件已备份到 {}", backup.display()),
            Err(err) => log::error!("备份损坏配置失败：{err}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("audio-switch-test-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn default_config_keeps_left_click_disabled() {
        let config = AppConfig::default();
        assert!(
            !config.left_click_switch,
            "左键切换默认应关闭，避免误触切换"
        );
        assert_eq!(config.version, CONFIG_VERSION);
        assert!(config.devices.is_empty());
        assert_eq!(config.language, "zh", "默认界面语言应为中文");
        assert_eq!(config.theme, "light", "默认主题应为浅色");
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = temp_dir("roundtrip");
        let store = ConfigStore::new(&dir);

        // 显式使用非默认值，确保序列化的是真实字段而不是回落默认
        let mut config = AppConfig {
            left_click_switch: true,
            language: "en".to_string(),
            theme: "dark".to_string(),
            ..AppConfig::default()
        };
        config.add_device("ep-a", "USB 游戏耳机");
        config.add_device("ep-b", "蓝牙耳机");
        store.save(&config).unwrap();

        let (loaded, status) = store.load();
        assert_eq!(status, ConfigLoadStatus::Loaded);
        assert_eq!(loaded, config);
        assert!(loaded.left_click_switch, "重启后应保留用户的开关选择");
        assert_eq!(loaded.language, "en", "重启后应保留界面语言");
        assert_eq!(loaded.theme, "dark", "重启后应保留界面主题");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn unknown_language_and_theme_fall_back_to_default() {
        let dir = temp_dir("fallback");
        let store = ConfigStore::new(&dir);
        fs::write(
            store.path(),
            br#"{"version":1,"leftClickSwitch":false,"devices":[],"language":"fr","theme":"neon"}"#,
        )
        .unwrap();

        let (config, status) = store.load();
        assert_eq!(status, ConfigLoadStatus::Loaded);
        assert_eq!(config.language, "zh");
        assert_eq!(config.theme, "light");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn locale_variants_are_normalized() {
        let mut config = AppConfig {
            language: "EN-us".to_string(),
            theme: "Dark".to_string(),
            ..AppConfig::default()
        };
        config.normalize();
        assert_eq!(config.language, "en");
        assert_eq!(config.theme, "dark");
        assert_eq!(config.lang(), crate::i18n::Lang::En);
    }

    #[test]
    fn missing_language_and_theme_use_defaults() {
        let dir = temp_dir("legacy");
        let store = ConfigStore::new(&dir);
        // 旧版本配置（没有 language / theme 字段）应能正常读取
        fs::write(
            store.path(),
            br#"{"version":1,"leftClickSwitch":true,"devices":[]}"#,
        )
        .unwrap();

        let (config, status) = store.load();
        assert_eq!(status, ConfigLoadStatus::Loaded);
        assert_eq!(config.language, "zh");
        assert_eq!(config.theme, "light");
        assert!(config.left_click_switch);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_file_creates_default() {
        let dir = temp_dir("missing");
        let store = ConfigStore::new(&dir);
        let (config, status) = store.load();
        assert_eq!(status, ConfigLoadStatus::Created);
        assert!(!config.left_click_switch);
        assert!(store.path().exists(), "缺失时应写出默认配置文件");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn corrupt_file_is_backed_up_and_recovered() {
        let dir = temp_dir("corrupt");
        let store = ConfigStore::new(&dir);
        fs::write(store.path(), b"{ this is not json").unwrap();

        let (config, status) = store.load();
        assert_eq!(status, ConfigLoadStatus::Recovered);
        assert!(!config.left_click_switch);

        let backups: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with("config.corrupt-")
            })
            .collect();
        assert_eq!(backups.len(), 1, "损坏文件应被备份保留");

        // 恢复正常配置
        let (again, status) = store.load();
        assert_eq!(status, ConfigLoadStatus::Loaded);
        assert_eq!(again, AppConfig::default());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn duplicate_endpoints_are_removed() {
        let mut config = AppConfig::default();
        config.devices.push(ConfiguredDevice {
            endpoint_id: "ep-a".into(),
            last_known_name: "A".into(),
            enabled: true,
        });
        config.devices.push(ConfiguredDevice {
            endpoint_id: "ep-a".into(),
            last_known_name: "A2".into(),
            enabled: false,
        });
        config.devices.push(ConfiguredDevice {
            endpoint_id: "  ".into(),
            last_known_name: "空".into(),
            enabled: true,
        });

        let removed = config.normalize();
        assert_eq!(removed, 2);
        assert_eq!(config.devices.len(), 1);
    }

    #[test]
    fn reorder_follows_given_ids() {
        let mut config = AppConfig::default();
        config.add_device("a", "A");
        config.add_device("b", "B");
        config.add_device("c", "C");

        config.reorder(&["c".into(), "a".into()]);
        let order: Vec<_> = config
            .devices
            .iter()
            .map(|d| d.endpoint_id.as_str())
            .collect();
        assert_eq!(order, vec!["c", "a", "b"]);
    }

    #[test]
    fn move_device_respects_bounds() {
        let mut config = AppConfig::default();
        config.add_device("a", "A");
        config.add_device("b", "B");

        assert!(!config.move_device("a", -1), "第一个设备无法上移");
        assert!(config.move_device("b", -1));
        assert_eq!(config.devices[0].endpoint_id, "b");
        assert!(!config.move_device("a", 1), "最后一个设备无法下移");
    }

    #[test]
    fn toggle_enabled_and_remove() {
        let mut config = AppConfig::default();
        config.add_device("a", "A");
        assert!(config.set_enabled("a", false));
        assert!(!config.devices[0].enabled);
        assert!(config.remove_device("a"));
        assert!(config.devices.is_empty());
        assert!(!config.remove_device("a"));
    }
}

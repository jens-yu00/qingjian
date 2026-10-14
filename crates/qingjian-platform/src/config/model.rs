use serde::{Deserialize, Serialize};

/// 配置文件 `[model]` 分节：本地整句模型的开关。
///
/// 随包的字级小模型在本机给整句候选重新排序，全程离线、不联网，与云联想互不影响（本地先出、云端到了另占它自己的格）。
/// 模型文件不在包里（或用户目录 `model/` 里）时开关无效。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct LocalModelConfig {
    /// 开着就加载模型、给整句重排。
    pub enabled: bool,

    /// macOS 本机 Sol 专用决策模型试用，替换字级模型；需要预先安装运行目录。
    pub sol: bool,
}

impl Default for LocalModelConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            sol: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::LocalModelConfig;
    #[test]
    fn sol_is_opt_in_and_survives_master_switch_changes() {
        let old: LocalModelConfig = toml::from_str("enabled = true").unwrap();
        assert!(!old.sol);
        let trial: LocalModelConfig = toml::from_str("enabled = false\nsol = true").unwrap();
        assert!(trial.sol && !trial.enabled);
        let roundtrip: LocalModelConfig =
            toml::from_str(&toml::to_string(&trial).unwrap()).unwrap();
        assert_eq!(trial, roundtrip);
    }
}

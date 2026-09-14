//! 候选窗口的整体缩放档位，拒绝无法在设置中选择的值。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u16", into = "u16")]
pub struct CandidateScale(
    /// 百分比，仅接受 ALL 中的档位。
    u16,
);

impl CandidateScale {
    pub const ALL: [Self; 7] = [
        Self(100),
        Self(125),
        Self(150),
        Self(175),
        Self(200),
        Self(225),
        Self(250),
    ];

    pub fn percent(self) -> u16 {
        self.0
    }

    pub fn factor(self) -> f64 {
        f64::from(self.0) / 100.0
    }
}

impl Default for CandidateScale {
    fn default() -> Self {
        Self(150)
    }
}

impl TryFrom<u16> for CandidateScale {
    type Error = String;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        Self::ALL
            .into_iter()
            .find(|s| s.0 == value)
            .ok_or_else(|| format!("candidate scale must be 100..=250 in steps of 25, got {value}"))
    }
}

impl From<CandidateScale> for u16 {
    fn from(scale: CandidateScale) -> Self {
        scale.0
    }
}

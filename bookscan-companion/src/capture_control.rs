use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CaptureMode {
    Auto,
    #[default]
    Semi,
    Manual,
}

impl CaptureMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Semi => "semi",
            Self::Manual => "manual",
        }
    }

}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PointState {
    #[default]
    Unknown,
    Searching,
    Stable,
    Armed,
}

impl PointState {
    pub fn from_str_lossy(s: &str) -> Self {
        match s {
            "searching" => Self::Searching,
            "stable" => Self::Stable,
            "armed" => Self::Armed,
            _ => Self::Unknown,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Searching => "searching",
            Self::Stable => "stable",
            Self::Armed => "armed",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoParams {
    pub settle_ms: u64,
    pub cooldown_ms: u64,
    pub motion_threshold: f32,
}

impl Default for AutoParams {
    fn default() -> Self {
        Self {
            settle_ms: 600,
            cooldown_ms: 1200,
            motion_threshold: 12.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsOutbound {
    CaptureNow,
    SetMode { mode: CaptureMode },
    SetAutoParams {
        settle_ms: u64,
        cooldown_ms: u64,
        motion_threshold: f32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsInbound {
    Hello { device: Option<String>, version: Option<String> },
    Ready,
    PointState { state: String },
    Capturing,
    Uploaded { page_id: String },
    Error { message: String },
}

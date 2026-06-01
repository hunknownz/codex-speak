use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize)]
pub struct ProviderMeta {
    pub id: &'static str,
    pub label: &'static str,
    pub languages: &'static str,
    pub footprint: &'static str,
    pub role: &'static str,
    pub recommended: bool,
    pub requires_model: bool,
}

pub const PROVIDERS: &[ProviderMeta] = &[
    ProviderMeta {
        id: "sherpa_melo",
        label: "MeloTTS 中文女声",
        languages: "中文优先，兼顾英文",
        footprint: "中等体积",
        role: "默认清楚女声，适合儿童模式第一版",
        recommended: true,
        requires_model: true,
    },
    ProviderMeta {
        id: "sherpa_kokoro",
        label: "Kokoro",
        languages: "中文和英文",
        footprint: "中等体积",
        role: "自然度备选，适合后续对比试听",
        recommended: false,
        requires_model: true,
    },
    ProviderMeta {
        id: "sherpa_zipvoice",
        label: "ZipVoice",
        languages: "中文和英文",
        footprint: "较大体积",
        role: "实验性参考音频方案，适合探索更像真人的声音",
        recommended: false,
        requires_model: true,
    },
    ProviderMeta {
        id: "piper",
        label: "Piper 轻量语音",
        languages: "中文",
        footprint: "轻量",
        role: "低配置兜底，优先保证能在普通 Windows 机器上跑",
        recommended: false,
        requires_model: true,
    },
    ProviderMeta {
        id: "system",
        label: "系统语音",
        languages: "跟随系统",
        footprint: "无需下载",
        role: "无模型兜底，适合快速检查链路",
        recommended: false,
        requires_model: false,
    },
];

pub fn supported_provider_ids() -> &'static [&'static str] {
    &[
        "sherpa_melo",
        "sherpa_kokoro",
        "sherpa_zipvoice",
        "piper",
        "system",
    ]
}

pub fn provider_meta(id: &str) -> Option<&'static ProviderMeta> {
    PROVIDERS.iter().find(|provider| provider.id == id)
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Timestamp,
    Base64,
    Chinese,
    Md5,
    Sha256,
    Json,
    Url,
    Video,
    Qrcode,
    Audio,
}
impl Tool {
    pub const ALL: [Self; 10] = [
        Self::Timestamp,
        Self::Base64,
        Self::Chinese,
        Self::Md5,
        Self::Sha256,
        Self::Json,
        Self::Url,
        Self::Qrcode,
        Self::Audio,
        Self::Video,
    ];
    pub fn title(self) -> &'static str {
        match self {
            Self::Timestamp => "时间戳转换",
            Self::Base64 => "Base64 编解码",
            Self::Chinese => "简体 / 繁体",
            Self::Md5 => "MD5 摘要",
            Self::Sha256 => "SHA-256 摘要",
            Self::Json => "JSON 格式化",
            Self::Url => "URL 编解码",
            Self::Video => "视频工具",
            Self::Qrcode => "二维码",
            Self::Audio => "音频工具",
        }
    }
    pub fn description(self) -> &'static str {
        match self {
            Self::Timestamp => "时间戳、日期时间与常用格式互转",
            Self::Base64 => "UTF-8 文本与标准 Base64 相互转换",
            Self::Chinese => "基于词组字典的简繁转换 · 繁体使用台湾用语",
            Self::Md5 => "生成文本的 32 位十六进制摘要（不适合密码存储）",
            Self::Sha256 => "生成文本的 SHA-256 摘要",
            Self::Json => "语法高亮、代码折叠、缩进与格式化",
            Self::Url => "整个 URL、参数值与表单的 UTF-8 编解码",
            Self::Video => "批量提取音频、添加或替换视频封面",
            Self::Audio => "MP3 / M4A / FLAC · 编辑标签和封面",
            Self::Qrcode => "生成二维码、保存 PNG、识别图片内容",
        }
    }
}

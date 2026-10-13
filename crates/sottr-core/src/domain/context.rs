//! 上下文适用性：结构识别与语义适用分开。
//!
//! 约束（开发方案 SX-006）：
//! - 文件名、4 MiB 尺寸与合法区段流都**不足以**识别普通战役；本模块不看结构，
//!   只接收调用方显式声明的模式、来源与（可空的）构建/DLC 信息；
//! - 用户声明是上下文证据，不是格式识别证明；声明为 campaign 时仍保留来源与限制；
//! - 模式未知或不适用时关闭收集语义（进度、计数、差集）；
//! - 构建号未知允许有限观测与带限制的社区进度候选，不宣称通用兼容。

use crate::report::{Context, ContextSource, Mode};

/// 调用方声明的上下文（输入）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclaredContext {
    /// 平台标识（如 windows_steam）；未知为 `None`。
    pub platform: Option<String>,
    /// 模式；未知用 [`Mode::Unknown`]，不适用用 [`Mode::Unsupported`]。
    pub mode: Mode,
    /// 构建号；未知为 `None`，不虚构。
    pub build: Option<String>,
    /// 已知 DLC 列表；未知为 `None`。
    pub dlc: Option<Vec<String>>,
    /// 声明来源。
    pub source: ContextSource,
}

impl Default for DeclaredContext {
    /// 最保守默认：模式未知、来源未知、其余未声明。
    fn default() -> Self {
        Self {
            platform: None,
            mode: Mode::Unknown,
            build: None,
            dlc: None,
            source: ContextSource::Unknown,
        }
    }
}

/// 上下文评估发现（映射到诊断码与限制文本）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextFinding {
    /// 用户声明普通战役（不是格式识别证明）。
    UserDeclaredCampaign,
    /// 模式未知：收集语义已关闭。
    UnknownModeClosed,
    /// profile 资料文件：仅容器观察。
    ProfileContainerOnly,
    /// 模式不适用：收集语义已关闭。
    UnsupportedModeClosed,
    /// 构建号未知：社区候选仅作有限观测。
    BuildUnknown,
    /// DLC 状态未知：适用范围受限。
    DlcUnknown,
}

impl ContextFinding {
    /// 稳定诊断码。
    pub fn code(self) -> &'static str {
        match self {
            ContextFinding::UserDeclaredCampaign => "MODE_USER_DECLARED_CAMPAIGN",
            ContextFinding::UnknownModeClosed => "MODE_UNKNOWN_COLLECTION_SEMANTICS_CLOSED",
            ContextFinding::ProfileContainerOnly => "MODE_PROFILE_CONTAINER_ONLY",
            ContextFinding::UnsupportedModeClosed => "MODE_UNSUPPORTED_COLLECTION_SEMANTICS_CLOSED",
            ContextFinding::BuildUnknown => "BUILD_UNKNOWN_LIMITED_OBSERVATIONS",
            ContextFinding::DlcUnknown => "DLC_UNKNOWN_LIMITED_APPLICABILITY",
        }
    }

    /// 对应用户可见限制文本（写入 `context.limitations`）。
    pub fn limitation(self) -> &'static str {
        match self {
            ContextFinding::UserDeclaredCampaign => "模式为用户声明（普通战役），不是格式识别证明",
            ContextFinding::UnknownModeClosed => "模式未知：收集语义（进度、计数、差集）已关闭",
            ContextFinding::ProfileContainerOnly => "profile 资料文件：仅容器观察，不进入战役解释",
            ContextFinding::UnsupportedModeClosed => "模式不适用：收集语义已关闭",
            ContextFinding::BuildUnknown => "构建号未知，社区校准候选仅作有限观测，不宣称通用兼容",
            ContextFinding::DlcUnknown => "DLC 状态未知，适用范围受限",
        }
    }
}

/// 上下文评估结果。
#[derive(Debug, Clone, PartialEq)]
pub struct ContextAssessment {
    /// 契约上下文对象（含来源与限制）。
    pub context: Context,
    /// 是否允许战役语义（进度、计数、参考差集）。
    pub campaign_applicable: bool,
    /// 评估发现。
    pub findings: Vec<ContextFinding>,
}

/// 评估声明上下文；不做任何结构识别。
pub fn assess(declared: &DeclaredContext) -> ContextAssessment {
    let mut findings = Vec::new();
    let campaign_applicable = declared.mode == Mode::Campaign;

    match declared.mode {
        Mode::Campaign => findings.push(ContextFinding::UserDeclaredCampaign),
        Mode::Profile => findings.push(ContextFinding::ProfileContainerOnly),
        Mode::Unknown => findings.push(ContextFinding::UnknownModeClosed),
        Mode::Unsupported => findings.push(ContextFinding::UnsupportedModeClosed),
    }
    if declared.build.is_none() {
        findings.push(ContextFinding::BuildUnknown);
    }
    if campaign_applicable && declared.dlc.is_none() {
        findings.push(ContextFinding::DlcUnknown);
    }

    let limitations = findings
        .iter()
        .map(|finding| finding.limitation().to_string())
        .collect();
    ContextAssessment {
        context: Context {
            platform: declared.platform.clone(),
            mode: declared.mode,
            build: declared.build.clone(),
            dlc: declared.dlc.clone(),
            source: declared.source,
            limitations,
        },
        campaign_applicable,
        findings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::limits::Limits;
    use crate::sections::SectionStream;
    use crate::synthetic::SampleBuilder;

    #[test]
    fn default_context_is_closed_and_keeps_reason() {
        let assessment = assess(&DeclaredContext::default());
        assert!(!assessment.campaign_applicable);
        assert_eq!(assessment.context.mode, Mode::Unknown);
        assert_eq!(assessment.context.source, ContextSource::Unknown);
        assert!(
            assessment
                .findings
                .contains(&ContextFinding::UnknownModeClosed)
        );
        assert!(assessment.findings.contains(&ContextFinding::BuildUnknown));
    }

    #[test]
    fn user_declared_campaign_keeps_source_and_limitations() {
        let declared = DeclaredContext {
            platform: Some("windows_steam".to_string()),
            mode: Mode::Campaign,
            build: None,
            dlc: None,
            source: ContextSource::UserDeclared,
        };
        let assessment = assess(&declared);
        assert!(assessment.campaign_applicable);
        assert_eq!(assessment.context.source, ContextSource::UserDeclared);
        assert_eq!(assessment.context.mode, Mode::Campaign);
        assert_eq!(
            assessment.context.platform.as_deref(),
            Some("windows_steam")
        );
        let joined = assessment.context.limitations.join(" ");
        assert!(joined.contains("不是格式识别证明"), "{joined}");
        assert!(joined.contains("构建号未知"), "{joined}");
        assert!(joined.contains("DLC 状态未知"), "{joined}");
        assert!(
            assessment
                .findings
                .contains(&ContextFinding::UserDeclaredCampaign)
        );
        assert!(assessment.findings.contains(&ContextFinding::DlcUnknown));
    }

    #[test]
    fn profile_mode_is_container_only() {
        let declared = DeclaredContext {
            mode: Mode::Profile,
            ..DeclaredContext::default()
        };
        let assessment = assess(&declared);
        assert!(!assessment.campaign_applicable);
        assert!(
            assessment
                .findings
                .contains(&ContextFinding::ProfileContainerOnly)
        );
        assert!(
            assessment
                .context
                .limitations
                .iter()
                .any(|item| item.contains("不进入战役解释"))
        );
    }

    #[test]
    fn renamed_challenge_tomb_style_input_stays_closed_without_declaration() {
        // 挑战古墓风格输入：结构校验通过、0x28 为 0，但不能仅凭结构识别普通战役。
        let mut builder = SampleBuilder::new();
        builder.push_section(1, 0);
        builder.write_f32_le(0x28, 0.0);
        let raw = builder.finish();
        let stream = SectionStream::walk(&raw, &Limits::default());
        assert!(stream.valid, "结构本身可以通过校验");

        let assessment = assess(&DeclaredContext::default());
        assert!(
            !assessment.campaign_applicable,
            "未提供可靠上下文时必须关闭战役语义"
        );
        assert!(
            assessment
                .findings
                .contains(&ContextFinding::UnknownModeClosed)
        );
        assert_eq!(assessment.context.source, ContextSource::Unknown);
    }

    #[test]
    fn campaign_declaration_on_challenge_like_input_keeps_declaration_limit() {
        let declared = DeclaredContext {
            mode: Mode::Campaign,
            source: ContextSource::UserDeclared,
            ..DeclaredContext::default()
        };
        let assessment = assess(&declared);
        assert!(assessment.campaign_applicable, "声明后开启战役语义");
        assert!(
            assessment
                .context
                .limitations
                .iter()
                .any(|item| item.contains("不是格式识别证明")),
            "声明不能变成结构证明"
        );
    }
}

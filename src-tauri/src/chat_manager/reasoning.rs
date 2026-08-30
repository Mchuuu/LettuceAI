#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReasoningMode {
    ProviderDefault,
    Auto,
    Enabled,
    Disabled,
}
impl ReasoningMode {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "provider-default" => Some(Self::ProviderDefault),
            "auto" => Some(Self::Auto),
            "enabled" => Some(Self::Enabled),
            "disabled" => Some(Self::Disabled),
            _ => None,
        }
    }

    pub fn from_settings(mode: Option<&str>, legacy_enabled: Option<bool>) -> Option<Self> {
        mode.and_then(Self::parse)
            .or_else(|| legacy_enabled.map(Self::from))
    }

    pub fn enables_legacy_adapter(self) -> bool {
        matches!(self, Self::Auto | Self::Enabled)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::ProviderDefault => "provider-default",
            Self::Auto => "auto",
            Self::Enabled => "enabled",
            Self::Disabled => "disabled",
        }
    }
}

impl From<bool> for ReasoningMode {
    fn from(enabled: bool) -> Self {
        if enabled {
            Self::Enabled
        } else {
            Self::Disabled
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_mode_precedes_legacy_boolean() {
        assert_eq!(
            ReasoningMode::from_settings(Some("provider-default"), Some(true)),
            Some(ReasoningMode::ProviderDefault)
        );
        assert_eq!(
            ReasoningMode::from_settings(None, Some(false)),
            Some(ReasoningMode::Disabled)
        );
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandCapability {
    ReadOnly,
    PreviewDraftWrite,
    InternalFixtureOnly,
    FutureExecutorOnly,
    BlockedExternal,
}

impl CommandCapability {
    fn is_externally_callable_now(self) -> bool {
        matches!(self, Self::ReadOnly | Self::PreviewDraftWrite)
    }

    fn label(self) -> &'static str {
        match self {
            Self::ReadOnly => "read-only",
            Self::PreviewDraftWrite => "preview/draft write",
            Self::InternalFixtureOnly => "internal fixture-only",
            Self::FutureExecutorOnly => "future executor-only",
            Self::BlockedExternal => "blocked external",
        }
    }
}

pub fn assert_command_allowed(
    command_name: &str,
    capability: CommandCapability,
) -> Result<(), String> {
    if capability.is_externally_callable_now() {
        return Ok(());
    }

    Err(format!(
        "Backend Command Gating V1 blocked command '{command_name}' ({capability}). This command is not externally callable from current UI flows because Apply/Restore execution is not ready.",
        capability = capability.label()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_only_and_preview_draft_commands_are_callable() {
        for capability in [
            CommandCapability::ReadOnly,
            CommandCapability::PreviewDraftWrite,
        ] {
            assert!(assert_command_allowed("safe_preview_command", capability).is_ok());
        }
    }

    #[test]
    fn executor_and_external_file_mutation_commands_fail_closed() {
        for capability in [
            CommandCapability::InternalFixtureOnly,
            CommandCapability::FutureExecutorOnly,
            CommandCapability::BlockedExternal,
        ] {
            let error = assert_command_allowed("risky_command", capability)
                .expect_err("risky command should fail closed");
            assert!(error.contains("Backend Command Gating V1"));
            assert!(error.contains("risky_command"));
            assert!(error.contains("not externally callable"));
        }
    }
}

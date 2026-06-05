#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandCapability {
    /// Safe request/response commands. Kept as explicit safety-review vocabulary
    /// even while most current read-only commands do not call the gate yet.
    #[allow(dead_code)]
    ReadOnly,
    /// Commands that may persist preview/draft metadata but must not mutate user files.
    /// This remains distinct from executor-only capabilities for command-surface review.
    #[allow(dead_code)]
    PreviewDraftWrite,
    /// Hidden fixture/prototype helpers may use this later, but it must never be
    /// treated as externally callable by the current UI command gate.
    #[allow(dead_code)]
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

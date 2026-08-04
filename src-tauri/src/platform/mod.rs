use std::{
    ffi::OsString,
    fs,
    path::Path,
    process::Command,
};

#[allow(dead_code)]
pub mod path_semantics;

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
compile_error!("SimSuite currently supports Windows, macOS, and Linux desktop targets.");

// A native build constructs only its host variant. Pure tests exercise all three command plans.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformId {
    Windows,
    Macos,
    Linux,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RevealTargetKind {
    Directory,
    File,
    Missing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeCommand {
    program: &'static str,
    args: Vec<OsString>,
}

pub fn reveal_in_file_manager(path: &Path) -> Result<(), String> {
    let resolved = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let target_kind = classify_target(&resolved);
    let command = build_reveal_command(current_platform(), &resolved, target_kind)?;

    Command::new(command.program)
        .args(&command.args)
        .spawn()
        .map_err(|error| {
            format!(
                "{} launch failed for '{}': {}",
                command.program,
                resolved.display(),
                error
            )
        })?;

    Ok(())
}

fn classify_target(path: &Path) -> RevealTargetKind {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() => RevealTargetKind::Directory,
        Ok(_) => RevealTargetKind::File,
        Err(_) => RevealTargetKind::Missing,
    }
}

fn current_platform() -> PlatformId {
    #[cfg(target_os = "windows")]
    {
        PlatformId::Windows
    }

    #[cfg(target_os = "macos")]
    {
        PlatformId::Macos
    }

    #[cfg(target_os = "linux")]
    {
        PlatformId::Linux
    }
}

fn build_reveal_command(
    platform: PlatformId,
    path: &Path,
    target_kind: RevealTargetKind,
) -> Result<NativeCommand, String> {
    match platform {
        PlatformId::Windows => match target_kind {
            RevealTargetKind::Directory => Ok(NativeCommand {
                program: "explorer.exe",
                args: vec![windows_path(path)],
            }),
            RevealTargetKind::File => Ok(NativeCommand {
                program: "explorer.exe",
                args: vec![OsString::from("/select,"), windows_path(path)],
            }),
            RevealTargetKind::Missing => Ok(NativeCommand {
                program: "explorer.exe",
                args: vec![windows_path(parent_or_error(path)?)],
            }),
        },
        PlatformId::Macos => match target_kind {
            RevealTargetKind::Directory => Ok(NativeCommand {
                program: "open",
                args: vec![native_path(path)],
            }),
            RevealTargetKind::File => Ok(NativeCommand {
                program: "open",
                args: vec![OsString::from("-R"), native_path(path)],
            }),
            RevealTargetKind::Missing => Ok(NativeCommand {
                program: "open",
                args: vec![native_path(parent_or_error(path)?)],
            }),
        },
        PlatformId::Linux => match target_kind {
            RevealTargetKind::Directory => Ok(NativeCommand {
                program: "xdg-open",
                args: vec![native_path(path)],
            }),
            RevealTargetKind::File | RevealTargetKind::Missing => Ok(NativeCommand {
                program: "xdg-open",
                args: vec![native_path(parent_or_error(path)?)],
            }),
        },
    }
}

fn parent_or_error(path: &Path) -> Result<&Path, String> {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .ok_or_else(|| {
            format!(
                "Could not reveal '{}', because no parent folder is available.",
                path.display()
            )
        })
}

fn native_path(path: &Path) -> OsString {
    path.as_os_str().to_os_string()
}

#[cfg(target_os = "windows")]
fn windows_path(path: &Path) -> OsString {
    use std::os::windows::ffi::{OsStrExt, OsStringExt};

    let normalized = path
        .as_os_str()
        .encode_wide()
        .map(|unit| if unit == b'/' as u16 { b'\\' as u16 } else { unit })
        .collect::<Vec<_>>();
    OsString::from_wide(&normalized)
}

#[cfg(not(target_os = "windows"))]
fn windows_path(path: &Path) -> OsString {
    OsString::from(path.to_string_lossy().replace('/', "\\"))
}

#[cfg(test)]
mod tests {
    use std::{ffi::OsString, path::Path};

    use super::{
        build_reveal_command, NativeCommand, PlatformId, RevealTargetKind,
    };

    #[test]
    fn macos_reveals_files_and_directories_natively() {
        assert_eq!(
            build_reveal_command(
                PlatformId::Macos,
                Path::new("/Users/player/Mods/example.package"),
                RevealTargetKind::File,
            )
            .expect("macOS file command"),
            NativeCommand {
                program: "open",
                args: vec![
                    OsString::from("-R"),
                    OsString::from("/Users/player/Mods/example.package"),
                ],
            }
        );
        assert_eq!(
            build_reveal_command(
                PlatformId::Macos,
                Path::new("/Users/player/Mods"),
                RevealTargetKind::Directory,
            )
            .expect("macOS directory command"),
            NativeCommand {
                program: "open",
                args: vec![OsString::from("/Users/player/Mods")],
            }
        );
    }

    #[test]
    fn windows_selects_files_and_normalizes_separators() {
        assert_eq!(
            build_reveal_command(
                PlatformId::Windows,
                Path::new("C:/Users/player/Mods/example.package"),
                RevealTargetKind::File,
            )
            .expect("Windows file command"),
            NativeCommand {
                program: "explorer.exe",
                args: vec![
                    OsString::from("/select,"),
                    OsString::from("C:\\Users\\player\\Mods\\example.package"),
                ],
            }
        );
    }

    #[test]
    fn linux_opens_the_parent_for_files() {
        assert_eq!(
            build_reveal_command(
                PlatformId::Linux,
                Path::new("/home/player/Mods/example.package"),
                RevealTargetKind::File,
            )
            .expect("Linux file command"),
            NativeCommand {
                program: "xdg-open",
                args: vec![OsString::from("/home/player/Mods")],
            }
        );
    }

    #[test]
    fn missing_paths_fall_back_to_a_real_parent() {
        assert_eq!(
            build_reveal_command(
                PlatformId::Macos,
                Path::new("/Users/player/Mods/missing.package"),
                RevealTargetKind::Missing,
            )
            .expect("missing path fallback"),
            NativeCommand {
                program: "open",
                args: vec![OsString::from("/Users/player/Mods")],
            }
        );
        assert!(build_reveal_command(
            PlatformId::Linux,
            Path::new("missing.package"),
            RevealTargetKind::Missing,
        )
        .is_err());
    }
}

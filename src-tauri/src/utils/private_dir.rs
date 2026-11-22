//! Folders readable by this user only. Written with Claude Code.

use std::path::Path;

/// Restricts the existing folder `dir` to this user (0700). On Linux the
/// default umask leaves new folders world-readable, and many homes are 0755
/// (older Ubuntu, homes created by pam_mkhomedir for directory accounts), so
/// another account of a shared workstation could read the files inside.
/// Windows and macOS already keep the app folders per user.
#[cfg(unix)]
pub fn restrict_to_owner(dir: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    match std::fs::metadata(dir)?.permissions().mode() & 0o777 {
        0o700 => Ok(()),
        _ => std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700)),
    }
}

#[cfg(not(unix))]
pub fn restrict_to_owner(_dir: &Path) -> std::io::Result<()> {
    Ok(())
}

/// Creates the app data folder (settings, history when kept on disk, lists,
/// telemetry state and, on Linux, the logs) readable by this user only.
pub fn ensure_private_app_data_dir(dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    restrict_to_owner(dir)
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn app_data_dir_is_made_private_even_when_it_exists() {
        let root = std::env::temp_dir().join(format!("plumed-private-{}", uuid::Uuid::new_v4()));
        let dir = root.join("org.olicorne.plumed");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();

        ensure_private_app_data_dir(&dir).unwrap();
        let mode = std::fs::metadata(&dir).unwrap().permissions().mode() & 0o777;
        std::fs::remove_dir_all(&root).unwrap();
        assert_eq!(mode, 0o700);
    }
}

use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;

fn main() -> io::Result<()> {
    let repo_root = env::current_dir()?;
    let desktop_src = repo_root.join("ChronoTrack.desktop");

    // Source icon file in the repo (optional).
    let icon_src = repo_root.join("chronotrack_icon.png");

    let home = env::var("HOME").unwrap_or_else(|_| String::from(""));
    if home.is_empty() {
        eprintln!("$HOME is not set; cannot determine ~/.local paths");
        std::process::exit(1);
    }
    let home = PathBuf::from(home);

    let applications_dir = home.join(".local/share/applications");
    let pixmaps_dir = home.join(".local/share/pixmaps");

    fs::create_dir_all(&applications_dir)?;
    fs::create_dir_all(&pixmaps_dir)?;

    let desktop_dst = applications_dir.join("ChronoTrack.desktop");
    fs::copy(&desktop_src, &desktop_dst)?;

    // If icon is available locally, install it.
    if icon_src.exists() {
        let icon_dst = pixmaps_dir.join("chronotrack_icon.png");
        let _ = fs::copy(&icon_src, &icon_dst);
    }

    // Best-effort: desktop entries should be readable; executable bit usually doesn't matter,
    // but set it anyway.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&desktop_dst)?.permissions();
        perms.set_mode(perms.mode() | 0o111);
        fs::set_permissions(&desktop_dst, perms)?;
    }

    // Refresh desktop database if available.
    // (Ignore failures; some systems don't have it.)
    let _ = std::process::Command::new("update-desktop-database")
        .arg(applications_dir)
        .status();

    println!("Installed desktop entry to: {}", desktop_dst.display());
    Ok(())
}

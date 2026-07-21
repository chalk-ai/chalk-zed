use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};
use std::fs;
use std::io;
use zed_extension_api::{self as zed, LanguageServerId, Result};

const REGISTRY: &str = "https://artifactregistry.googleapis.com";
const CHALK_LSP_VERSION: &str = "v2.154.9";

struct ChalkExtension;

impl ChalkExtension {
    fn server_path(&self, language_server_id: &LanguageServerId) -> Result<String> {
        let windows = matches!(zed::current_platform().0, zed::Os::Windows);
        let (executable, backup_executable) = if windows {
            ("server/chalk-lsp.exe", "server.old/chalk-lsp.exe")
        } else {
            ("server/chalk-lsp", "server.old/chalk-lsp")
        };
        let mut installed_version = fs::read_to_string("server/installed-version")
            .ok()
            .map(|version| version.trim().to_string())
            .filter(|version| !version.is_empty());
        let mut server_is_valid = fs::metadata(executable)
            .map(|metadata| metadata.is_file())
            .unwrap_or(false)
            && installed_version.is_some();
        if !server_is_valid {
            let backup_version = fs::read_to_string("server.old/installed-version")
                .ok()
                .map(|version| version.trim().to_string())
                .filter(|version| !version.is_empty());
            let backup_is_valid = fs::metadata(backup_executable)
                .map(|metadata| metadata.is_file())
                .unwrap_or(false)
                && backup_version.is_some();
            match fs::symlink_metadata("server") {
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => {
                    return Err(format!(
                        "failed to inspect the incomplete chalk-lsp installation: {error}"
                    ))
                }
                Ok(metadata) => {
                    let result = if metadata.file_type().is_dir() {
                        fs::remove_dir_all("server")
                    } else {
                        fs::remove_file("server")
                    };
                    result.map_err(|error| {
                        format!("failed to remove the incomplete chalk-lsp installation: {error}")
                    })?;
                }
            }
            if backup_is_valid {
                fs::rename("server.old", "server").map_err(|error| {
                    format!("failed to restore the previous chalk-lsp installation: {error}")
                })?;
                installed_version = backup_version;
                server_is_valid = true;
            }
        }
        if server_is_valid && installed_version.as_deref() == Some(CHALK_LSP_VERSION) {
            fs::remove_dir_all("server.old").ok();
            fs::remove_dir_all("server.new").ok();
            return Ok(executable.to_string());
        }

        let server_exists = server_is_valid;

        let result = (|| {
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::CheckingForUpdate,
            );
            let (os, architecture) = zed::current_platform();
            let target = match (os, architecture) {
                (zed::Os::Mac, zed::Architecture::X8664) => "x86_64-apple-darwin",
                (zed::Os::Mac, zed::Architecture::Aarch64) => "aarch64-apple-darwin",
                (zed::Os::Linux, zed::Architecture::X8664) => "x86_64-unknown-linux-gnu",
                (zed::Os::Linux, zed::Architecture::Aarch64) => "aarch64-unknown-linux-gnu",
                (zed::Os::Windows, zed::Architecture::X8664) => "x86_64-pc-windows-gnu",
                _ => {
                    return Err("chalk-lsp does not publish a binary for this platform".to_string())
                }
            };
            let extension = if windows { "zip" } else { "tar.gz" };
            let archive_name = format!("chalk-lsp-{CHALK_LSP_VERSION}-{target}.{extension}");
            let download_base = format!(
                "{REGISTRY}/download/v1/projects/chalk-prod/locations/us/repositories/chalk-lsp/files/chalk-lsp:{CHALK_LSP_VERSION}:"
            );

            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );
            zed::download_file(
                &format!("{download_base}{archive_name}:download?alt=media"),
                &archive_name,
                zed::DownloadedFileType::Uncompressed,
            )?;
            zed::download_file(
                &format!("{download_base}{archive_name}.sha256:download?alt=media"),
                "archive.sha256",
                zed::DownloadedFileType::Uncompressed,
            )?;

            let expected = fs::read_to_string("archive.sha256")
                .map_err(|error| error.to_string())?
                .split_whitespace()
                .next()
                .ok_or("downloaded checksum was empty")?
                .to_string();
            let actual = format!(
                "{:x}",
                Sha256::digest(fs::read(&archive_name).map_err(|error| error.to_string())?)
            );
            if expected != actual {
                return Err(format!("checksum mismatch for {archive_name}"));
            }

            fs::remove_dir_all("server.new").ok();
            fs::create_dir_all("server.new").map_err(|error| error.to_string())?;
            if extension == "zip" {
                let archive = fs::File::open(&archive_name).map_err(|error| error.to_string())?;
                let mut zip = zip::ZipArchive::new(archive).map_err(|error| error.to_string())?;
                for index in 0..zip.len() {
                    let mut entry = zip.by_index(index).map_err(|error| error.to_string())?;
                    if entry.is_dir() || !entry.name().ends_with("chalk-lsp.exe") {
                        continue;
                    }
                    let mut destination = fs::File::create("server.new/chalk-lsp.exe")
                        .map_err(|error| error.to_string())?;
                    io::copy(&mut entry, &mut destination).map_err(|error| error.to_string())?;
                }
            } else {
                let archive = fs::File::open(&archive_name).map_err(|error| error.to_string())?;
                let mut tar = tar::Archive::new(GzDecoder::new(archive));
                for entry in tar.entries().map_err(|error| error.to_string())? {
                    let mut entry = entry.map_err(|error| error.to_string())?;
                    if !entry
                        .path()
                        .map_err(|error| error.to_string())?
                        .ends_with("chalk-lsp")
                    {
                        continue;
                    }
                    entry
                        .unpack("server.new/chalk-lsp")
                        .map_err(|error| error.to_string())?;
                }
            }
            if fs::metadata(if windows {
                "server.new/chalk-lsp.exe"
            } else {
                "server.new/chalk-lsp"
            })
            .is_err()
            {
                return Err(format!("{archive_name} did not contain chalk-lsp"));
            }
            fs::write(
                "server.new/installed-version",
                format!("{CHALK_LSP_VERSION}\n"),
            )
            .map_err(|error| error.to_string())?;

            match fs::remove_dir_all("server.old") {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(format!("failed to remove the previous backup: {error}")),
            }
            if server_exists {
                fs::rename("server", "server.old").map_err(|error| error.to_string())?;
            }
            if let Err(error) = fs::rename("server.new", "server") {
                if !server_exists {
                    return Err(error.to_string());
                }
                return match fs::rename("server.old", "server") {
                    Ok(()) => Err(error.to_string()),
                    Err(rollback_error) => Err(format!(
                        "failed to replace chalk-lsp ({error}) and restore the previous installation ({rollback_error}); the previous installation remains in server.old"
                    )),
                };
            }
            fs::remove_dir_all("server.old").ok();
            fs::remove_file(&archive_name).ok();
            fs::remove_file("archive.sha256").ok();
            Ok(executable.to_string())
        })();

        match result {
            Ok(path) => {
                zed::set_language_server_installation_status(
                    language_server_id,
                    &zed::LanguageServerInstallationStatus::None,
                );
                Ok(path)
            }
            Err(error) => {
                zed::set_language_server_installation_status(
                    language_server_id,
                    &zed::LanguageServerInstallationStatus::Failed(error.clone()),
                );
                let previous_server_is_valid = fs::metadata(executable)
                    .map(|metadata| metadata.is_file())
                    .unwrap_or(false)
                    && fs::read_to_string("server/installed-version")
                        .map(|version| !version.trim().is_empty())
                        .unwrap_or(false);
                if previous_server_is_valid {
                    Ok(executable.to_string())
                } else {
                    Err(error)
                }
            }
        }
    }
}

impl zed::Extension for ChalkExtension {
    fn new() -> Self {
        Self
    }

    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        if worktree.read_text_file("chalk.yml").is_err()
            && worktree.read_text_file("chalk.yaml").is_err()
        {
            return Err(
                "chalk-lsp only runs in a worktree containing chalk.yml or chalk.yaml".to_string(),
            );
        }
        Ok(zed::Command {
            command: self.server_path(language_server_id)?,
            args: vec!["lsp".to_string()],
            env: Default::default(),
        })
    }
}

zed::register_extension!(ChalkExtension);

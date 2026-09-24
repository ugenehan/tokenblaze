//! Signed in-app updates for the Windows build.
//!
//! The Swift application publishes a Sparkle appcast. The Rust build reads
//! the same feed, but only accepts HTTPS Windows executables signed by the
//! project's Sparkle Ed25519 key.

use std::fs;
use std::io::Read;
use std::net::{SocketAddr, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use ed25519_dalek::{Signature, VerifyingKey};
use reqwest::blocking::Client;
use reqwest::blocking::Response;
use reqwest::Url;

pub const APPCAST_URL: &str =
    "https://raw.githubusercontent.com/ugenehan/tokenblaze/main/appcast.xml";
const RELEASES_URL: &str = "https://github.com/ugenehan/tokenblaze/releases";
const ED25519_PUBLIC_KEY: &str = "3jvJJLbB67g3lB8SU2fZxB7rQpA3Qc4nm7neAexBhpw=";
const MAX_DOWNLOAD_BYTES: u64 = 256 * 1024 * 1024;
const USER_AGENT: &str = concat!("TokenBlaze/", env!("CARGO_PKG_VERSION"));

#[derive(Debug, Clone, PartialEq)]
pub enum UpdateStatus {
    Idle,
    Checking,
    UpToDate,
    Available {
        version: String,
    },
    Incompatible {
        version: String,
    },
    Downloading {
        version: String,
        downloaded: u64,
        total: Option<u64>,
    },
    Ready {
        version: String,
    },
    Failed {
        message: String,
    },
}

#[derive(Debug, Clone)]
struct UpdatePackage {
    version: String,
    url: String,
    length: Option<u64>,
    signature: String,
}

enum CheckOutcome {
    UpToDate,
    Available(UpdatePackage),
    Incompatible(String),
}

enum WorkerMessage {
    Checked(Result<CheckOutcome, String>),
    Progress {
        version: String,
        downloaded: u64,
        total: Option<u64>,
    },
    Downloaded(Result<(UpdatePackage, PathBuf), String>),
}

pub struct UpdateManager {
    status: UpdateStatus,
    package: Option<UpdatePackage>,
    staged_path: Option<PathBuf>,
    sender: Sender<WorkerMessage>,
    receiver: Receiver<WorkerMessage>,
}

impl UpdateManager {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            status: UpdateStatus::Idle,
            package: None,
            staged_path: None,
            sender,
            receiver,
        }
    }

    pub fn status(&self) -> &UpdateStatus {
        &self.status
    }

    pub fn check(&mut self) {
        if matches!(
            self.status,
            UpdateStatus::Checking | UpdateStatus::Downloading { .. }
        ) {
            return;
        }

        self.status = UpdateStatus::Checking;
        self.package = None;
        self.staged_path = None;
        let sender = self.sender.clone();
        thread::spawn(move || {
            let result = check_for_update(APPCAST_URL, env!("CARGO_PKG_VERSION"))
                .map_err(|error| error.to_string());
            let _ = sender.send(WorkerMessage::Checked(result));
        });
    }

    pub fn download(&mut self) {
        let Some(package) = self.package.clone() else {
            return;
        };
        if !matches!(self.status, UpdateStatus::Available { .. }) {
            return;
        }

        self.status = UpdateStatus::Downloading {
            version: package.version.clone(),
            downloaded: 0,
            total: package.length,
        };
        let sender = self.sender.clone();
        thread::spawn(move || {
            let result = download_package(&package, &sender)
                .map(|path| (package, path))
                .map_err(|error| error.to_string());
            let _ = sender.send(WorkerMessage::Downloaded(result));
        });
    }

    pub fn dismiss(&mut self) {
        if !matches!(
            self.status,
            UpdateStatus::Checking | UpdateStatus::Downloading { .. }
        ) {
            self.status = UpdateStatus::Idle;
        }
    }

    pub fn report_error(&mut self, error: impl Into<String>) {
        self.status = UpdateStatus::Failed {
            message: error.into(),
        };
    }

    /// Drain worker messages. Returns true if visible state changed.
    pub fn poll(&mut self) -> bool {
        let mut changed = false;
        while let Ok(message) = self.receiver.try_recv() {
            changed = true;
            match message {
                WorkerMessage::Checked(Ok(CheckOutcome::UpToDate)) => {
                    self.status = UpdateStatus::UpToDate;
                }
                WorkerMessage::Checked(Ok(CheckOutcome::Incompatible(version))) => {
                    self.status = UpdateStatus::Incompatible { version };
                }
                WorkerMessage::Checked(Ok(CheckOutcome::Available(package))) => {
                    self.status = UpdateStatus::Available {
                        version: package.version.clone(),
                    };
                    self.package = Some(package);
                }
                WorkerMessage::Checked(Err(message)) => {
                    self.status = UpdateStatus::Failed { message };
                }
                WorkerMessage::Progress {
                    version,
                    downloaded,
                    total,
                } => {
                    self.status = UpdateStatus::Downloading {
                        version,
                        downloaded,
                        total,
                    };
                }
                WorkerMessage::Downloaded(Ok((package, path))) => {
                    self.status = UpdateStatus::Ready {
                        version: package.version.clone(),
                    };
                    self.package = Some(package);
                    self.staged_path = Some(path);
                }
                WorkerMessage::Downloaded(Err(message)) => {
                    self.status = UpdateStatus::Failed { message };
                }
            }
        }
        changed
    }

    /// Start the trusted helper that replaces the running executable.
    /// The caller should exit the event loop after this succeeds.
    pub fn install_and_restart(&self) -> Result<()> {
        let staged = self
            .staged_path
            .as_ref()
            .context("no verified update has been downloaded")?;
        let current = std::env::current_exe().context("cannot locate TokenBlaze executable")?;
        let helper = update_root().join("tokenblaze-update-helper.exe");

        fs::create_dir_all(update_root()).context("cannot create update directory")?;
        if helper.exists() {
            fs::remove_file(&helper).context("cannot replace old update helper")?;
        }
        fs::copy(&current, &helper).context("cannot prepare update helper")?;
        Command::new(&helper)
            .arg("--apply-update")
            .arg(staged)
            .arg(&current)
            .spawn()
            .context("cannot start update helper")?;
        Ok(())
    }
}

/// Run the helper mode before initializing logging or windows.
pub fn run_update_helper_if_requested() -> Result<bool> {
    let mut args = std::env::args_os();
    let _program = args.next();
    if args.next().as_deref() != Some(std::ffi::OsStr::new("--apply-update")) {
        return Ok(false);
    }

    let staged = PathBuf::from(args.next().context("missing staged update path")?);
    let target = PathBuf::from(args.next().context("missing update target path")?);
    if args.next().is_some() {
        bail!("unexpected update helper arguments");
    }
    apply_staged_update(&staged, &target)?;
    Ok(true)
}

fn check_for_update(feed_url: &str, current_version: &str) -> Result<CheckOutcome> {
    let feed_url = secure_url(feed_url)?;
    let xml = send_https_get(feed_url)
        .context("could not contact the update server")?
        .error_for_status()
        .context("the update server returned an error")?
        .text()
        .context("could not read the update feed")?;
    parse_appcast(&xml, current_version)
}

fn parse_appcast(xml: &str, current_version: &str) -> Result<CheckOutcome> {
    let document = roxmltree::Document::parse(xml).context("invalid update feed XML")?;
    let mut newest_release: Option<String> = None;
    let mut newest_package: Option<UpdatePackage> = None;

    for item in document
        .descendants()
        .filter(|node| node.is_element() && node.tag_name().name() == "item")
    {
        let item_version = item
            .descendants()
            .find(|node| node.is_element() && node.tag_name().name() == "shortVersionString")
            .and_then(|node| node.text())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned);

        let Some(version) = item_version else {
            continue;
        };
        if !is_newer(&version, current_version) {
            continue;
        }
        if newest_release
            .as_ref()
            .map(|known| is_newer(&version, known))
            .unwrap_or(true)
        {
            newest_release = Some(version.clone());
        }

        for enclosure in item
            .descendants()
            .filter(|node| node.is_element() && node.tag_name().name() == "enclosure")
        {
            let attribute = |name: &str| {
                enclosure
                    .attributes()
                    .find(|attribute| attribute.name() == name)
                    .map(|attribute| attribute.value())
            };
            let Some(url) = attribute("url") else {
                continue;
            };
            let os = attribute("os").unwrap_or_default();
            let architecture = attribute("architecture").unwrap_or_default();
            if !is_windows_executable(url, os, architecture) {
                continue;
            }
            let Some(signature) = attribute("edSignature") else {
                continue;
            };
            if secure_url(url).is_err() {
                continue;
            }
            let package = UpdatePackage {
                version: version.clone(),
                url: url.to_owned(),
                length: attribute("length").and_then(|value| value.parse().ok()),
                signature: signature.to_owned(),
            };
            if newest_package
                .as_ref()
                .map(|known| is_newer(&package.version, &known.version))
                .unwrap_or(true)
            {
                newest_package = Some(package);
            }
        }
    }

    if let Some(package) = newest_package {
        Ok(CheckOutcome::Available(package))
    } else if let Some(version) = newest_release {
        Ok(CheckOutcome::Incompatible(version))
    } else {
        Ok(CheckOutcome::UpToDate)
    }
}

fn download_package(package: &UpdatePackage, sender: &Sender<WorkerMessage>) -> Result<PathBuf> {
    if package
        .length
        .is_some_and(|length| length > MAX_DOWNLOAD_BYTES)
    {
        bail!("update package is larger than the allowed limit");
    }
    let url = secure_url(&package.url)?;
    let mut response = send_https_get(url)
        .context("could not download the update")?
        .error_for_status()
        .context("the update download returned an error")?;
    let total = package.length.or_else(|| response.content_length());
    if total.is_some_and(|length| length > MAX_DOWNLOAD_BYTES) {
        bail!("update package is larger than the allowed limit");
    }

    let mut bytes = Vec::with_capacity(total.unwrap_or(0).min(MAX_DOWNLOAD_BYTES) as usize);
    let mut chunk = [0_u8; 64 * 1024];
    loop {
        let count = response
            .read(&mut chunk)
            .context("the update download was interrupted")?;
        if count == 0 {
            break;
        }
        bytes.extend_from_slice(&chunk[..count]);
        if bytes.len() as u64 > MAX_DOWNLOAD_BYTES {
            bail!("update package is larger than the allowed limit");
        }
        let _ = sender.send(WorkerMessage::Progress {
            version: package.version.clone(),
            downloaded: bytes.len() as u64,
            total,
        });
    }

    if let Some(expected) = package.length {
        if bytes.len() as u64 != expected {
            bail!(
                "update size mismatch: expected {expected} bytes, received {}",
                bytes.len()
            );
        }
    }
    verify_signature(&bytes, &package.signature, ED25519_PUBLIC_KEY)?;

    let version_dir = update_root().join(safe_component(&package.version));
    fs::create_dir_all(&version_dir).context("cannot create update staging directory")?;
    let staged = version_dir.join("tokenblaze-update.exe");
    fs::write(&staged, bytes).context("cannot save the verified update")?;
    Ok(staged)
}

fn verify_signature(payload: &[u8], signature: &str, public_key: &str) -> Result<()> {
    let public_key: [u8; 32] = STANDARD
        .decode(public_key)
        .context("invalid updater public key")?
        .try_into()
        .map_err(|_| anyhow!("invalid updater public key length"))?;
    let signature: [u8; 64] = STANDARD
        .decode(signature)
        .context("invalid update signature encoding")?
        .try_into()
        .map_err(|_| anyhow!("invalid update signature length"))?;
    let public_key = VerifyingKey::from_bytes(&public_key).context("invalid updater public key")?;
    public_key
        .verify_strict(payload, &Signature::from_bytes(&signature))
        .context("update signature verification failed")
}

fn apply_staged_update(staged: &Path, target: &Path) -> Result<()> {
    let root = fs::canonicalize(update_root()).context("update directory does not exist")?;
    let staged = fs::canonicalize(staged).context("staged update does not exist")?;
    let helper = fs::canonicalize(std::env::current_exe()?)?;
    if !staged.starts_with(&root) || !helper.starts_with(&root) {
        bail!("update helper paths are outside the staging directory");
    }
    if !target
        .extension()
        .and_then(|value| value.to_str())
        .map(|extension| extension.eq_ignore_ascii_case("exe"))
        .unwrap_or(false)
    {
        bail!("update target is not an executable");
    }
    if !target.exists() {
        bail!("update target does not exist");
    }

    let backup = target.with_extension("exe.old");
    let _ = fs::remove_file(&backup);
    let mut renamed = false;
    let mut last_error = None;
    for _ in 0..300 {
        match fs::rename(target, &backup) {
            Ok(()) => {
                renamed = true;
                break;
            }
            Err(error) => {
                last_error = Some(error);
                thread::sleep(Duration::from_millis(100));
            }
        }
    }
    if !renamed {
        return Err(last_error
            .map(anyhow::Error::from)
            .unwrap_or_else(|| anyhow!("timed out waiting for TokenBlaze to exit")))
        .context("could not replace the running executable");
    }

    if let Err(error) = fs::copy(&staged, target) {
        let _ = fs::rename(&backup, target);
        return Err(error).context("could not install the update");
    }
    if let Err(error) = Command::new(target).spawn() {
        let _ = fs::remove_file(target);
        let _ = fs::rename(&backup, target);
        return Err(error).context("could not restart TokenBlaze");
    }
    let _ = fs::remove_file(&backup);
    Ok(())
}

fn http_client(dns_override: Option<(&str, SocketAddr)>) -> Result<Client> {
    let mut builder = Client::builder()
        .user_agent(USER_AGENT)
        .https_only(true)
        .connect_timeout(Duration::from_secs(7))
        .timeout(Duration::from_secs(120));
    if let Some((host, address)) = dns_override {
        // The URL keeps the original host, so TLS still validates its certificate.
        builder = builder.resolve(host, address);
    }
    builder
        .build()
        .context("cannot initialize the update client")
}

/// Send an HTTPS GET while trying every address returned by the system resolver.
///
/// Some Windows networks can reach only one address behind a multi-address CDN.
/// reqwest may stop after the first same-family address times out, so updates need
/// an explicit fallback rather than failing while another endpoint is reachable.
fn send_https_get(url: Url) -> Result<Response> {
    let host = url
        .host_str()
        .context("the update URL has no host")?
        .to_owned();
    let port = url.port_or_known_default().unwrap_or(443);
    let mut addresses: Vec<_> = (host.as_str(), port)
        .to_socket_addrs()
        .map(|resolved| resolved.collect())
        .unwrap_or_default();
    addresses.sort_by_key(|address| if address.is_ipv4() { 0 } else { 1 });
    addresses.dedup();

    if addresses.is_empty() {
        return http_client(None)?
            .get(url)
            .send()
            .context("the update host could not be resolved");
    }

    let mut last_error = None;
    for address in addresses {
        match http_client(Some((&host, address)))?.get(url.clone()).send() {
            Ok(response) => return Ok(response),
            Err(error) => last_error = Some(error),
        }
    }
    Err(last_error
        .map(anyhow::Error::from)
        .unwrap_or_else(|| anyhow!("the update host has no reachable addresses")))
}

fn secure_url(value: &str) -> Result<Url> {
    let url = Url::parse(value).context("invalid update URL")?;
    if url.scheme() != "https" {
        bail!("the update URL is not HTTPS");
    }
    Ok(url)
}

fn is_windows_executable(url: &str, os: &str, architecture: &str) -> bool {
    let os = os.trim().to_ascii_lowercase();
    let architecture = architecture.trim().to_ascii_lowercase();
    let architecture_matches = architecture.is_empty()
        || architecture == std::env::consts::ARCH
        || (std::env::consts::ARCH == "x86_64"
            && ["x64", "amd64"].contains(&architecture.as_str()))
        || (std::env::consts::ARCH == "aarch64" && architecture == "arm64");
    (os.is_empty() || os == "windows")
        && architecture_matches
        && Url::parse(url)
            .ok()
            .map(|url| url.path().to_ascii_lowercase().ends_with(".exe"))
            .unwrap_or(false)
}

fn is_newer(candidate: &str, current: &str) -> bool {
    let candidate = version_numbers(candidate);
    let current = version_numbers(current);
    let width = candidate.len().max(current.len());
    (0..width)
        .map(|index| {
            (
                candidate.get(index).copied().unwrap_or(0),
                current.get(index).copied().unwrap_or(0),
            )
        })
        .find(|(left, right)| left != right)
        .map(|(left, right)| left > right)
        .unwrap_or(false)
}

fn version_numbers(value: &str) -> Vec<u64> {
    value
        .trim_start_matches(['v', 'V'])
        .split('.')
        .map(|part| {
            part.chars()
                .take_while(|character| character.is_ascii_digit())
                .collect::<String>()
                .parse()
                .unwrap_or(0)
        })
        .collect()
}

fn update_root() -> PathBuf {
    std::env::temp_dir().join("TokenBlaze").join("updates")
}

fn safe_component(value: &str) -> String {
    let safe: String = value
        .chars()
        .filter(|character| {
            character.is_ascii_alphanumeric() || ['.', '-', '_'].contains(character)
        })
        .collect();
    if safe.is_empty() {
        "update".to_owned()
    } else {
        safe
    }
}

pub fn releases_url() -> &'static str {
    RELEASES_URL
}

#[cfg(test)]
mod tests {
    use super::*;

    const PREFIX: &str = r#"<?xml version="1.0" encoding="utf-8"?>
        <rss xmlns:sparkle="http://www.andymatuschak.org/xml-namespaces/sparkle" version="2.0">
        <channel>"#;
    const SUFFIX: &str = "</channel></rss>";

    #[test]
    fn selects_signed_windows_executable() {
        let xml = format!(
            r#"{PREFIX}<item>
                <sparkle:shortVersionString>1.2.0</sparkle:shortVersionString>
                <enclosure url="https://example.com/TokenBlaze.dmg" length="12" sparkle:edSignature="mac" />
                <enclosure url="https://example.com/TokenBlaze.exe" length="42" sparkle:os="windows" sparkle:edSignature="signed" />
            </item>{SUFFIX}"#
        );
        let CheckOutcome::Available(package) = parse_appcast(&xml, "1.0.0").unwrap() else {
            panic!("expected an available Windows update");
        };
        assert_eq!(package.version, "1.2.0");
        assert_eq!(package.length, Some(42));
        assert_eq!(package.signature, "signed");
    }

    #[test]
    fn reports_newer_mac_only_release_as_incompatible() {
        let xml = format!(
            r#"{PREFIX}<item>
                <sparkle:shortVersionString>1.1.16</sparkle:shortVersionString>
                <enclosure url="https://example.com/TokenBlaze.dmg" length="12" sparkle:edSignature="mac" />
            </item>{SUFFIX}"#
        );
        let CheckOutcome::Incompatible(version) = parse_appcast(&xml, "0.1.0").unwrap() else {
            panic!("expected an incompatible release");
        };
        assert_eq!(version, "1.1.16");
    }

    #[test]
    fn compares_numeric_versions() {
        assert!(is_newer("1.10.0", "1.9.9"));
        assert!(is_newer("v2.0", "1.99.99"));
        assert!(!is_newer("1.2", "1.2.0"));
        assert!(!is_newer("1.1.9", "1.2.0"));
    }

    #[test]
    fn verifies_standard_signature_and_rejects_changed_payload() {
        // RFC 8032, test vector 1: signature of an empty message.
        let public_key = "11qYAYKxCrfVS/7TyWQHOg7hcvPapiMlrwIaaPcHURo=";
        let signature = "5VZDAMNgrHKQhuLMgG6CioSHfx645dl02HPgZSJJAVVfuIIVkKM7rMYeOXAc+bRr0lv18FlbviRlUUFDjnoQCw==";
        assert!(verify_signature(b"", signature, public_key).is_ok());
        assert!(verify_signature(b"changed", signature, public_key).is_err());
    }

    #[test]
    #[ignore = "requires the public update service"]
    fn reads_live_appcast() {
        if let Err(error) = check_for_update(APPCAST_URL, env!("CARGO_PKG_VERSION")) {
            panic!("{error:#}");
        }
    }
}

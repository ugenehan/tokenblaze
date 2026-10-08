//! Validate a detached release signature and print the matching appcast XML.
//! This tool does not sign, upload, or publish anything.

use std::env;
use std::fs;

use anyhow::{bail, Context, Result};
use tokenblaze_core::updater::{verify_signature, ED25519_PUBLIC_KEY};

fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    let version = args.next().context("missing version")?;
    let executable = args.next().context("missing executable path")?;
    let url = args.next().context("missing release asset URL")?;
    let signature_file = args.next().context("missing signature file path")?;
    if args.next().is_some() || !valid_version(&version) {
        bail!("expected version, executable path, release asset URL, and signature file");
    }
    let expected_url = format!(
        "https://github.com/ugenehan/tokenblaze/releases/download/v{version}/tokenblaze.exe"
    );
    if url != expected_url {
        bail!("release asset URL does not match the version and repository");
    }
    let bytes = fs::read(executable).context("could not read the release executable")?;
    if bytes.is_empty() || bytes.len() > 256 * 1024 * 1024 || !bytes.starts_with(b"MZ") {
        bail!("release executable is empty, too large, or not a Windows executable");
    }
    let signature =
        fs::read_to_string(signature_file).context("could not read the detached signature")?;
    let signature = signature.trim();
    verify_signature(&bytes, signature, ED25519_PUBLIC_KEY)
        .context("release signature does not match the updater public key")?;
    println!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>");
    println!("<rss version=\"2.0\" xmlns:sparkle=\"http://www.andymatuschak.org/xml-namespaces/sparkle\"><channel>");
    println!("<title>TokenBlaze updates</title><item>");
    println!("<sparkle:shortVersionString>{version}</sparkle:shortVersionString>");
    println!("<enclosure url=\"{url}\" length=\"{}\" sparkle:os=\"windows\" sparkle:architecture=\"x86_64\" sparkle:edSignature=\"{signature}\" />", bytes.len());
    println!("</item></channel></rss>");
    Ok(())
}

fn valid_version(version: &str) -> bool {
    version.split('.').count() == 3
        && version
            .split('.')
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
}

#[cfg(test)]
mod tests {
    use super::valid_version;

    #[test]
    fn release_version_is_numeric_and_xml_safe() {
        assert!(valid_version("1.2.3"));
        assert!(!valid_version("1.2"));
        assert!(!valid_version("1.2.3<item>"));
    }
}

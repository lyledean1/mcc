use anyhow::Result;
use std::path::Path;

#[cfg(feature = "gcs")]
use {anyhow::Context, std::process::Command};

/// Configuration for cloud storage
pub struct CloudConfig {
    pub bucket: String,
    pub enabled: bool,
}

impl CloudConfig {
    /// Load cloud config from ~/.mcc/config.json
    pub fn load() -> Result<Self> {
        let home = std::env::var("HOME")?;
        let config_path = std::path::PathBuf::from(home).join(".mcc/config.json");

        if !config_path.exists() {
            return Ok(Self {
                bucket: String::new(),
                enabled: false,
            });
        }

        let content = std::fs::read_to_string(&config_path)?;
        let config: serde_json::Value = serde_json::from_str(&content)?;

        Ok(Self {
            bucket: config
                .get("gcs_bucket")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            enabled: !config
                .get("gcs_bucket")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .is_empty(),
        })
    }

    /// Save cloud config to ~/.mcc/config.json
    pub fn save(&self) -> Result<()> {
        let home = std::env::var("HOME")?;
        let config_dir = std::path::PathBuf::from(home).join(".mcc");
        std::fs::create_dir_all(&config_dir)?;

        let config_path = config_dir.join("config.json");
        let config = serde_json::json!({
            "gcs_bucket": self.bucket,
        });

        std::fs::write(&config_path, serde_json::to_string_pretty(&config)?)?;
        Ok(())
    }
}

#[cfg(feature = "gcs")]
/// Upload a session file to GCS using gsutil
pub async fn upload_session(file_path: &Path, bucket: &str) -> Result<String> {
    let filename = file_path
        .file_name()
        .and_then(|f| f.to_str())
        .context("Invalid filename")?;

    // Strip gs:// prefix from bucket if present
    let bucket_name = bucket.strip_prefix("gs://").unwrap_or(bucket);
    let gcs_path = format!("gs://{}/{}", bucket_name, filename);

    // TODO: Make this configurable or search common paths
    let gsutil_path = std::env::var("GSUTIL_PATH")
        .unwrap_or_else(|_| "/Users/lyledean/Downloads/google-cloud-sdk/bin/gsutil".to_string());

    // Use gsutil which respects gcloud auth
    let output = Command::new(&gsutil_path)
        .arg("cp")
        .arg(file_path)
        .arg(&gcs_path)
        .output()
        .context(format!("Failed to run gsutil at: {}", gsutil_path))?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("gsutil upload failed: {}", error);
    }

    Ok(gcs_path)
}

#[cfg(feature = "gcs")]
/// Download a session file from GCS using gsutil
pub async fn download_session(gcs_path: &str, output_path: &Path) -> Result<()> {
    // TODO: Make this configurable or search common paths
    let gsutil_path = std::env::var("GSUTIL_PATH")
        .unwrap_or_else(|_| "/Users/lyledean/Downloads/google-cloud-sdk/bin/gsutil".to_string());

    // Use gsutil which respects gcloud auth
    let output = Command::new(&gsutil_path)
        .arg("cp")
        .arg(gcs_path)
        .arg(output_path)
        .output()
        .context(format!("Failed to run gsutil at: {}", gsutil_path))?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("gsutil download failed: {}", error);
    }

    Ok(())
}

#[cfg(not(feature = "gcs"))]
#[allow(dead_code)]
pub async fn upload_session(_file_path: &Path, _bucket: &str) -> Result<String> {
    anyhow::bail!("GCS support not enabled. Rebuild with --features gcs")
}

#[cfg(not(feature = "gcs"))]
#[allow(dead_code)]
pub async fn download_session(_gcs_path: &str, _output_path: &Path) -> Result<()> {
    anyhow::bail!("GCS support not enabled. Rebuild with --features gcs")
}

/// Configure GCS bucket
pub fn configure_bucket(bucket: &str) -> Result<()> {
    let mut config = CloudConfig::load().unwrap_or(CloudConfig {
        bucket: String::new(),
        enabled: false,
    });

    config.bucket = bucket.to_string();
    config.enabled = !bucket.is_empty();
    config.save()?;

    println!("✓ GCS bucket configured: {}", bucket);
    println!("\nYou can now use:");
    println!("  mcc share <session>   # Upload to GCS");
    println!("  mcc fetch <gs://...>  # Download from GCS");

    Ok(())
}

# GCS Setup for MCC

This guide helps you set up Google Cloud Storage for team-wide session sharing.

## Prerequisites

- Google Cloud account
- `gcloud` CLI installed
- A GCS bucket for your team

## 1. Create a GCS Bucket

```bash
# Create bucket
gsutil mb gs://your-team-claude-sessions

# Or via gcloud
gcloud storage buckets create gs://your-team-claude-sessions \
  --location=us-central1

# Set permissions (team-wide access)
gsutil iam ch allAuthenticatedUsers:objectViewer gs://your-team-claude-sessions
```

## 2. Configure MCC

```bash
# Build with GCS support
cargo build --release --features gcs

# Configure bucket
mcc config set-bucket gs://your-team-claude-sessions
```

## 3. Set Up Authentication

### Option A: Using gcloud CLI (Recommended for development)

```bash
# Authenticate
gcloud auth application-default login

# Verify
gcloud auth application-default print-access-token
```

### Option B: Service Account (Recommended for production/CI)

```bash
# Create service account
gcloud iam service-accounts create mcc-uploader \
  --display-name="MCC Session Uploader"

# Grant bucket access
gcloud storage buckets add-iam-policy-binding gs://your-team-claude-sessions \
  --member=serviceAccount:mcc-uploader@your-project.iam.gserviceaccount.com \
  --role=roles/storage.objectAdmin

# Create and download key
gcloud iam service-accounts keys create ~/mcc-key.json \
  --iam-account=mcc-uploader@your-project.iam.gserviceaccount.com

# Set environment variable
export GOOGLE_APPLICATION_CREDENTIALS=~/mcc-key.json

# Add to shell profile for persistence
echo 'export GOOGLE_APPLICATION_CREDENTIALS=~/mcc-key.json' >> ~/.zshrc
```

## 4. Test the Setup

```bash
# Export a session first
mcc  # Press 'e' to export

# Upload to GCS
mcc share ~/.mcc/exports/your-session.json.gz

# Should output:
# ✓ Session uploaded!
#   GCS path: gs://your-team-claude-sessions/your-session.json.gz
#
# Share with your team:
#   mcc fetch gs://your-team-claude-sessions/your-session.json.gz
```

## 5. Share with Team

Send the GCS path to teammates:

```bash
# They run:
mcc fetch gs://your-team-claude-sessions/your-session.json.gz /their/project/path
```

## Bucket Permissions

### Public Team Bucket (Simple)
```bash
# Anyone with link can read
gsutil iam ch allAuthenticatedUsers:objectViewer gs://your-bucket
```

### Private Team Bucket (Secure)
```bash
# Only specific users/groups
gsutil iam ch user:alice@company.com:objectAdmin gs://your-bucket
gsutil iam ch user:bob@company.com:objectAdmin gs://your-bucket

# Or use Google Groups
gsutil iam ch group:engineers@company.com:objectAdmin gs://your-bucket
```

## Cost Considerations

GCS pricing (as of 2024):
- Storage: ~$0.02/GB/month (Standard class, US)
- Operations: ~$0.05 per 10,000 operations
- Network egress: ~$0.12/GB (outside GCP)

**Typical usage:**
- Session file: ~50KB compressed
- 100 sessions/month: ~5MB storage = ~$0.0001/month
- Negligible cost for small teams

## Troubleshooting

### Permission Denied
```bash
# Check current auth
gcloud auth application-default print-access-token

# Re-authenticate
gcloud auth application-default login
```

### Bucket Not Found
```bash
# List buckets
gsutil ls

# Check bucket exists
gsutil ls gs://your-team-claude-sessions
```

### Upload Fails
```bash
# Test manual upload
echo "test" > /tmp/test.txt
gsutil cp /tmp/test.txt gs://your-team-claude-sessions/

# If this works, issue is with MCC
# If this fails, issue is with GCS permissions
```

## Alternative: AWS S3

Want to use S3 instead? The architecture supports it:

1. Replace `cloud-storage` with `aws-sdk-s3` in `Cargo.toml`
2. Update `src/cloud.rs` to use S3 client
3. Similar workflow with `s3://` paths

See [S3_SETUP.md](S3_SETUP.md) for details (coming soon).

## Security Best Practices

1. **Use private buckets** - Don't make sessions public
2. **Rotate service account keys** - Every 90 days
3. **Audit access logs** - Enable GCS audit logging
4. **Encrypt at rest** - GCS encrypts by default, but consider CMEK
5. **Set lifecycle policies** - Auto-delete old sessions

Example lifecycle policy:
```bash
# Auto-delete sessions older than 30 days
gsutil lifecycle set lifecycle.json gs://your-bucket

# lifecycle.json:
{
  "lifecycle": {
    "rule": [{
      "action": {"type": "Delete"},
      "condition": {"age": 30}
    }]
  }
}
```

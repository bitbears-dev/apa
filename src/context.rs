use std::process::Command;

#[derive(Debug, Clone)]
pub struct AwsContext {
    pub profile: String,
    pub region: String,
}

impl AwsContext {
    pub fn detect() -> Self {
        // Find profile
        let profile = std::env::var("AWS_PROFILE").unwrap_or_else(|_| {
            // fallback to querying aws
            let out = Command::new("aws")
                .args(["configure", "get", "profile"])
                .output()
                .ok();
            out.and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "default".to_string())
        });

        // Find region
        let region = std::env::var("AWS_REGION").unwrap_or_else(|_| {
            let out = Command::new("aws")
                .args(["configure", "get", "region"])
                .output()
                .ok();
            out.and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "us-east-1".to_string())
        });

        AwsContext { profile, region }
    }
}

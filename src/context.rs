use std::process::Command;

pub const AWS_REGIONS: &[&str] = &[
    "us-east-1",
    "us-east-2",
    "us-west-1",
    "us-west-2",
    "af-south-1",
    "ap-east-1",
    "ap-south-1",
    "ap-northeast-1",
    "ap-northeast-2",
    "ap-northeast-3",
    "ap-southeast-1",
    "ap-southeast-2",
    "ap-southeast-3",
    "ca-central-1",
    "eu-central-1",
    "eu-west-1",
    "eu-west-2",
    "eu-west-3",
    "eu-south-1",
    "eu-north-1",
    "me-south-1",
    "sa-east-1",
];

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

    pub fn list_profiles() -> Vec<String> {
        let mut profiles = vec!["default".to_string()];
        if let Ok(out) = Command::new("aws")
            .args(["configure", "list-profiles"])
            .output()
            && out.status.success()
            && let Ok(lines) = String::from_utf8(out.stdout)
        {
            let mut fetched: Vec<String> = lines
                .lines()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            profiles.append(&mut fetched);
        }
        // Deduplicate while retaining order
        let mut seen = std::collections::HashSet::new();
        profiles
            .into_iter()
            .filter(|x| seen.insert(x.clone()))
            .collect()
    }

    pub fn list_regions() -> Vec<String> {
        AWS_REGIONS.iter().map(|s| s.to_string()).collect()
    }
}

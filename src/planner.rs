use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MissingParameter {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Plan {
    pub intent_summary: String,
    pub risk_level: RiskLevel,
    pub requires_confirmation: bool,
    pub aws_cli_args: Vec<String>,
    pub missing_parameters: Option<Vec<MissingParameter>>,
    pub profile: Option<String>,
    pub region: Option<String>,
    pub assumptions: Vec<String>,
    pub explanation: String,
}

pub struct Planner {
    api_key: String,
    client: Client,
}

impl Planner {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: Client::new(),
        }
    }

    pub async fn generate_plan(
        &self,
        prompt: &str,
        aws_profile: &str,
        aws_region: &str,
    ) -> Result<Plan> {
        let system_message = format!(
            r#"You are APA (AI Powered AWS CLI), an expert in AWS CLI operations.
Your job is to read natural language requests and output a precise structured JSON plan for executing an AWS CLI command.
Current Context:
- AWS_PROFILE: {aws_profile}
- AWS_REGION: {aws_region}

Constraints:
1. ONLY use the `aws` CLI command.
2. Output valid JSON satisfying the requested schema exactly.
3. For potentially destructive actions (delete, terminate, stop), set `risk_level` to `high`. 
   For write/update actions, set it to `medium`. 
   For read-only operations, set it to `low`.
4. If the user's intent lacks essential parameters (e.g. required resource IDs/names for the command), DO NOT use placeholders like `<xyz>` in `aws_cli_args`. Instead, securely list them in `missing_parameters` with a clear `name` and `description` so the user can be prompted.
5. NEVER include shell pipes (|), redirects (>), or logical operators (&&, ||) in `aws_cli_args`. 
"#
        );

        let tools = serde_json::json!([{
            "type": "function",
            "function": {
                "name": "generate_aws_cli_plan",
                "description": "Generates a structured execution plan for an AWS CLI command",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "intent_summary": {"type": "string"},
                        "risk_level": {"type": "string", "enum": ["low", "medium", "high"]},
                        "requires_confirmation": {"type": "boolean"},
                        "aws_cli_args": {
                            "type": "array",
                            "items": {"type": "string"}
                        },
                        "missing_parameters": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "name": {"type": "string"},
                                    "description": {"type": "string"}
                                },
                                "required": ["name", "description"]
                            }
                        },
                        "profile": {"type": "string"},
                        "region": {"type": "string"},
                        "assumptions": {
                            "type": "array",
                            "items": {"type": "string"}
                        },
                        "explanation": {"type": "string"}
                    },
                    "required": ["intent_summary", "risk_level", "requires_confirmation", "aws_cli_args", "assumptions", "explanation"]
                }
            }
        }]);

        let payload = serde_json::json!({
            "model": "gpt-4o",
            "messages": [
                {"role": "system", "content": system_message},
                {"role": "user", "content": prompt}
            ],
            "tools": tools,
            "tool_choice": {"type": "function", "function": {"name": "generate_aws_cli_plan"}}
        });

        let resp = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&payload)
            .send()
            .await?;

        let response_text = resp.text().await?;

        let completion: serde_json::Value = serde_json::from_str(&response_text)?;
        if let Some(err) = completion.get("error") {
            return Err(anyhow::anyhow!("OpenAI API error: {}", err));
        }

        let tool_calls = completion["choices"][0]["message"]["tool_calls"].as_array();
        if let Some(calls) = tool_calls
            && let Some(call) = calls.first()
            && let Some(args_str) = call["function"]["arguments"].as_str()
        {
            let plan: Plan = serde_json::from_str(args_str)?;
            return Ok(plan);
        }

        Err(anyhow::anyhow!("Failed to parse Plan from OpenAI response"))
    }
}

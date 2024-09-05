use crate::core::{Indication, IndicationKind, LlmClient, Location};
use anyhow::anyhow;
use anyhow::Result;
use log::{debug, error, info};
use openai::set_base_url;
use serde::{Deserialize, Serialize};
use serde_json;

pub struct OpenAI {
    model: String,
    api_key: String,
}

impl LlmClient for OpenAI {
    fn new(api_key: String, model: String) -> OpenAI {
        OpenAI { model, api_key }
    }

    async fn check(&self, path: String, prompt: String) -> Result<Vec<Indication>> {
        set_base_url("https://api.openai.com/v1/".to_string());

        let indications = serde_json::from_str::<ResponseContent>(
            &self.request_chat(prompt).await.unwrap_or_else(|e| {
                panic!("Failed to get response: {}", e);
            }),
        )
        .unwrap_or_else(|e| {
            panic!("Failed to parse response: {}", e);
        });

        Ok(indications
            .messages
            .iter()
            .map(move |indication| Indication {
                kind: match indication.kind.as_str() {
                    "error" => IndicationKind::Error,
                    "warning" => IndicationKind::Warning,
                    "cancel" => IndicationKind::Cancel,
                    _ => panic!("Unknown indication kind: {}", indication.kind),
                },
                message: indication.message.clone(),
                location: Location {
                    path: path.clone(),
                    start_line: indication.location.start_line,
                    end_line: indication.location.end_line,
                },
            })
            .collect())
    }
}

impl OpenAI {
    async fn request_chat(&self, message: String) -> Result<String> {
        let client = reqwest::Client::new();
        let request_json = ChatRequest {
            model: self.model.clone(),
            messages: vec![Message {
                role: "user".to_string(),
                content: message.clone(),
            }],
            response_format: Some(ResponseFormat {
                r#type: "json_object".to_string(),
            }),
        };

        let messages_json = match serde_json::to_string(&request_json) {
            Ok(json) => json,
            Err(e) => panic!("Failed to serialize request: {}", e),
        };

        let response = client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", &format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .body(messages_json)
            .send()
            .await?;

        let status = response.status();
        let text = response.text().await?;
        if !status.is_success() {
            error!("OpenAI API Error: status: {}, response: {}", status, &text);
            error!("Request: {}", message);
            anyhow!("Failed to get response: {}", status);
        }

        let response = serde_json::from_str::<ApiResponse>(&text)?;

        info!("Total tokens: {}", response.usage.total_tokens);
        info!("Cost: {:.2} JPY", response.usage.cost_as_jpy());

        let indication = response.choices.first().unwrap().message.content.clone();

        debug!("OpenAI Response: {}\n====\n{}", message, indication);

        Ok(indication)
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct ChatRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub response_format: Option<ResponseFormat>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ResponseFormat {
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Deserialize, Debug)]
struct ApiResponse {
    choices: Vec<Choice>,
    usage: Usage,
}

#[derive(Deserialize, Debug)]
struct Choice {
    message: Message,
}

#[derive(Serialize, Deserialize, Debug)]
struct Message {
    role: String,
    content: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct ResponseContent {
    messages: Vec<GptIndication>,
}

#[derive(Deserialize, Debug)]
struct Usage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

// Ref: https://openai.com/api/pricing/
const INPUT_TOKEN_UNIT_PRICE: f64 = 5.0 / 1000.0 / 150.0; // $5 per 1000 tokens, $1 per 150 JPY.
const OUTPUT_TOKEN_UNIT_PRICE: f64 = 15.0 / 1000.0 / 150.0; // $15 per 1000 tokens, $1 per 150 JPY.

impl Usage {
    fn cost_as_jpy(&self) -> f64 {
        (self.prompt_tokens as f64 * INPUT_TOKEN_UNIT_PRICE)
            + (self.completion_tokens as f64 * OUTPUT_TOKEN_UNIT_PRICE)
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct GptIndication {
    pub kind: String,
    pub message: String,
    pub location: GptIndicationLocation,
}

#[derive(Serialize, Deserialize, Debug)]
struct GptIndicationLocation {
    pub start_line: u64,
    pub end_line: u64,
}

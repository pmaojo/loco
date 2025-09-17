use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
        ChatCompletionRequestUserMessageArgs, ChatCompletionRequestUserMessageContent,
        CreateChatCompletionRequestArgs,
    },
    Client,
};
use async_trait::async_trait;

use crate::{
    ai::{
        KnowledgeAssistant, KnowledgeAssistantError, KnowledgeAssistantInitError, KnowledgeRequest,
        KnowledgeResponse,
    },
    config::OpenAiSettings,
};

/// Knowledge assistant backed by OpenAI chat completions.
pub struct OpenAiKnowledgeAssistant {
    client: Client<OpenAIConfig>,
    model: String,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    system_prompt: Option<String>,
}

impl OpenAiKnowledgeAssistant {
    /// Instantiates a new assistant using the provided configuration.
    pub fn try_new(settings: &OpenAiSettings) -> Result<Self, KnowledgeAssistantInitError> {
        if settings.api_key.trim().is_empty() {
            return Err(KnowledgeAssistantInitError::InvalidConfiguration(
                "`ai.assistant.api_key` must be provided".to_string(),
            ));
        }
        if settings.model.trim().is_empty() {
            return Err(KnowledgeAssistantInitError::InvalidConfiguration(
                "`ai.assistant.model` must be provided".to_string(),
            ));
        }

        let mut config = OpenAIConfig::new().with_api_key(settings.api_key.clone());
        if let Some(base) = &settings.api_base {
            config = config.with_api_base(base.clone());
        }
        let client = Client::with_config(config);

        Ok(Self {
            client,
            model: settings.model.clone(),
            temperature: settings.temperature,
            max_tokens: settings.max_tokens,
            system_prompt: settings.system_prompt.clone(),
        })
    }

    fn build_messages(
        &self,
        request: &KnowledgeRequest,
    ) -> Result<Vec<ChatCompletionRequestMessage>, KnowledgeAssistantError> {
        let mut messages = Vec::new();
        if let Some(system_prompt) = &self.system_prompt {
            let system = ChatCompletionRequestSystemMessageArgs::default()
                .content(system_prompt.clone())
                .build()
                .map_err(|err| KnowledgeAssistantError::Request(err.to_string()))?;
            messages.push(ChatCompletionRequestMessage::System(system));
        }

        let user_content = format!(
            "Ontology context: {}\n\n{}\n\nPrompt:\n{}",
            request.ontology.as_str(),
            request.context_as_text(),
            request.prompt
        );
        let user = ChatCompletionRequestUserMessageArgs::default()
            .content(ChatCompletionRequestUserMessageContent::Text(user_content))
            .build()
            .map_err(|err| KnowledgeAssistantError::Request(err.to_string()))?;
        messages.push(ChatCompletionRequestMessage::User(user));
        Ok(messages)
    }
}

#[async_trait]
impl KnowledgeAssistant for OpenAiKnowledgeAssistant {
    async fn respond(
        &self,
        request: KnowledgeRequest,
    ) -> Result<KnowledgeResponse, KnowledgeAssistantError> {
        let messages = self.build_messages(&request)?;
        let mut builder = CreateChatCompletionRequestArgs::default();
        builder.model(self.model.clone());
        builder.messages(messages);
        if let Some(max_tokens) = self.max_tokens {
            builder.max_tokens(max_tokens);
        }
        if let Some(temperature) = self.temperature {
            builder.temperature(temperature);
        }
        let payload = builder
            .build()
            .map_err(|err| KnowledgeAssistantError::Request(err.to_string()))?;

        let response = self
            .client
            .chat()
            .create(payload)
            .await
            .map_err(|err| KnowledgeAssistantError::Provider(err.to_string()))?;

        let message = response
            .choices
            .into_iter()
            .find_map(|choice| choice.message.content)
            .ok_or(KnowledgeAssistantError::EmptyResponse)?;

        Ok(KnowledgeResponse { message })
    }
}

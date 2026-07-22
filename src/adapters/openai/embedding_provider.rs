use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use serde_json::json;
use std::sync::Arc;

use crate::config::EmbeddingProviderSettings;
use crate::errors::{CustomError, EmbeddingError, EmbeddingTransportErrorKind};
use crate::EmbeddingProvider;

const OPENAI_EMBEDDING_ENDPOINT: &str = "https://api.openai.com/v1/embeddings";
const MAX_OPENAI_EMBEDDING_INPUTS: usize = 2048;

/// OpenAI-based implementation of the EmbeddingProvider trait.
///
/// # Description
///
/// This implementation uses the specified OpenAI embedding model to generate embeddings.
/// It handles the communication with OpenAI's API to convert text into vector embeddings.
///
/// # See also
///
/// - [`EmbeddingProvider`]
/// - [OpenAI Embeddings API](https://platform.openai.com/docs/api-reference/embeddings)
pub(crate) struct OpenAIEmbeddingProvider {
    api_key: String,
    model: String,
    vector_size: usize,
    transport: Arc<dyn OpenAIEmbeddingTransport>,
}

impl OpenAIEmbeddingProvider {
    /// Creates a new OpenAIEmbeddingProvider instance.
    ///
    /// # Parameters
    ///
    /// - `settings`: Configuration settings containing the OpenAI API key
    ///
    /// # Returns
    ///
    /// - `Ok`: A new `OpenAIEmbeddingProvider` instance
    /// - `Err`: A `CustomError` if initialization fails (e.g., missing API key)
    pub fn new(settings: EmbeddingProviderSettings) -> Result<Self, CustomError> {
        if settings.api_key.trim().is_empty() {
            return Err(EmbeddingError::MissingApiKey.into());
        }
        println!(
            "OpenAI Embedding Provider: Initialized with {} model.",
            settings.model.as_str()
        );
        Ok(OpenAIEmbeddingProvider {
            api_key: settings.api_key,
            model: settings.model.as_str().to_string(),
            vector_size: settings.model.vector_size() as usize,
            transport: Arc::new(ReqwestOpenAIEmbeddingTransport {
                client: Client::new(),
            }),
        })
    }
}

#[async_trait]
trait OpenAIEmbeddingTransport: Send + Sync {
    async fn send_embeddings_request(
        &self,
        api_key: &str,
        payload: serde_json::Value,
    ) -> Result<OpenAIEmbeddingHttpResponse, EmbeddingError>;
}

struct OpenAIEmbeddingHttpResponse {
    status: StatusCode,
    body: String,
}

struct ReqwestOpenAIEmbeddingTransport {
    client: Client,
}

#[async_trait]
impl OpenAIEmbeddingTransport for ReqwestOpenAIEmbeddingTransport {
    async fn send_embeddings_request(
        &self,
        api_key: &str,
        payload: serde_json::Value,
    ) -> Result<OpenAIEmbeddingHttpResponse, EmbeddingError> {
        let response = self
            .client
            .post(OPENAI_EMBEDDING_ENDPOINT)
            .bearer_auth(api_key)
            .json(&payload)
            .send()
            .await
            .map_err(embedding_transport_error)?;
        let status = response.status();
        let body = response.text().await.map_err(embedding_transport_error)?;
        Ok(OpenAIEmbeddingHttpResponse { status, body })
    }
}

#[async_trait]
impl EmbeddingProvider for OpenAIEmbeddingProvider {
    fn vector_size(&self) -> usize {
        self.vector_size
    }

    async fn generate_embedding<'a>(&self, text: &'a str) -> Result<Vec<f32>, EmbeddingError> {
        if text.trim().is_empty() {
            return Err(EmbeddingError::BlankInput { index: None });
        }
        let mut embeddings = self.request_embedding_batch(&[text]).await?;
        if embeddings.len() != 1 {
            return Err(EmbeddingError::CountMismatch {
                expected: 1,
                actual: embeddings.len(),
            });
        }
        Ok(embeddings.remove(0))
    }

    async fn bulk_generate_embeddings<'a>(
        &self,
        texts: &'a [&'a str],
    ) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        validate_embedding_texts(texts)?;
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let mut embeddings = Vec::with_capacity(texts.len());
        for chunk in texts.chunks(MAX_OPENAI_EMBEDDING_INPUTS) {
            embeddings.extend(self.request_embedding_batch(chunk).await?);
        }
        Ok(embeddings)
    }
}

impl OpenAIEmbeddingProvider {
    async fn request_embedding_batch(
        &self,
        texts: &[&str],
    ) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        validate_embedding_texts(texts)?;
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let payload = embedding_payload(&self.model, texts);
        let response = self
            .transport
            .send_embeddings_request(&self.api_key, payload)
            .await?;
        if !response.status.is_success() {
            return Err(EmbeddingError::HttpStatus {
                status: response.status.as_u16(),
                body: response.body,
            });
        }
        let resp_json: serde_json::Value =
            serde_json::from_str(&response.body).map_err(|error| EmbeddingError::InvalidJson {
                detail: error.to_string(),
            })?;
        parse_embedding_response(resp_json, texts.len(), self.vector_size)
    }
}

fn validate_embedding_texts(texts: &[&str]) -> Result<(), EmbeddingError> {
    if let Some(index) = texts.iter().position(|text| text.trim().is_empty()) {
        return Err(EmbeddingError::BlankInput { index: Some(index) });
    }
    Ok(())
}

fn embedding_payload(model: &str, texts: &[&str]) -> serde_json::Value {
    json!({
        "model": model,
        "input": texts,
    })
}

fn parse_embedding_response(
    response: serde_json::Value,
    expected_count: usize,
    vector_size: usize,
) -> Result<Vec<Vec<f32>>, EmbeddingError> {
    let data = response
        .get("data")
        .and_then(|data| data.as_array())
        .ok_or(EmbeddingError::MissingData)?;
    if data.len() != expected_count {
        return Err(EmbeddingError::CountMismatch {
            expected: expected_count,
            actual: data.len(),
        });
    }

    let mut embeddings = vec![None; expected_count];
    for (item_position, item) in data.iter().enumerate() {
        let index = item.get("index").and_then(|index| index.as_u64()).ok_or(
            EmbeddingError::MissingIndex {
                item: item_position,
            },
        )? as usize;
        if index >= expected_count {
            return Err(EmbeddingError::IndexOutOfRange {
                index,
                expected_count,
            });
        }
        if embeddings[index].is_some() {
            return Err(EmbeddingError::DuplicateIndex { index });
        }
        let embedding = item
            .get("embedding")
            .and_then(|embedding| embedding.as_array())
            .ok_or(EmbeddingError::MissingEmbedding {
                item: item_position,
            })?;
        if embedding.len() != vector_size {
            return Err(EmbeddingError::DimensionMismatch {
                index,
                expected: vector_size,
                actual: embedding.len(),
            });
        }
        let vec_embedding = embedding
            .iter()
            .enumerate()
            .map(|(component, value)| {
                value
                    .as_f64()
                    .map(|value| value as f32)
                    .ok_or(EmbeddingError::NonNumericValue { index, component })
            })
            .collect::<Result<Vec<_>, _>>()?;
        embeddings[index] = Some(vec_embedding);
    }

    embeddings
        .into_iter()
        .enumerate()
        .map(|(index, embedding)| embedding.ok_or(EmbeddingError::MissingResponseIndex { index }))
        .collect()
}

fn embedding_transport_error(error: reqwest::Error) -> EmbeddingError {
    let kind = if error.is_timeout() {
        EmbeddingTransportErrorKind::Timeout
    } else if error.is_connect() {
        EmbeddingTransportErrorKind::Connect
    } else if error.is_request() {
        EmbeddingTransportErrorKind::Request
    } else if error.is_body() {
        EmbeddingTransportErrorKind::Body
    } else {
        EmbeddingTransportErrorKind::Other
    };
    EmbeddingError::Transport {
        transport_kind: kind,
        detail: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::vector::EmbeddingModel;
    use std::sync::Mutex;

    fn create_test_settings(api_key: &str) -> EmbeddingProviderSettings {
        EmbeddingProviderSettings::new(api_key.to_string(), EmbeddingModel::TextEmbedding3Large)
    }

    fn create_test_provider(
        transport: Arc<dyn OpenAIEmbeddingTransport>,
    ) -> OpenAIEmbeddingProvider {
        OpenAIEmbeddingProvider {
            api_key: "valid_api_key".to_owned(),
            model: EmbeddingModel::TextEmbedding3Large.as_str().to_owned(),
            vector_size: EmbeddingModel::TextEmbedding3Large.vector_size() as usize,
            transport,
        }
    }

    #[test]
    fn test_new_with_valid_api() {
        let settings = create_test_settings("dummy_key");
        let provider = OpenAIEmbeddingProvider::new(settings);
        assert!(
            provider.is_ok(),
            "OpenAIEmbeddingProvider initialization should succeed with valid API key."
        );
    }

    #[test]
    fn test_new_with_empty_api() {
        let settings = create_test_settings("");
        let error = match OpenAIEmbeddingProvider::new(settings) {
            Ok(_) => panic!("empty API key should be rejected"),
            Err(error) => error,
        };

        assert!(matches!(
            error,
            CustomError::Embedding(EmbeddingError::MissingApiKey)
        ));
    }

    #[tokio::test]
    async fn test_generate_embedding_with_empty_text() {
        let settings = create_test_settings("valid_key");
        let provider = OpenAIEmbeddingProvider::new(settings).unwrap();
        let error = provider.generate_embedding("  ").await.unwrap_err();

        assert_eq!(error, EmbeddingError::BlankInput { index: None });
    }

    #[test]
    fn batch_payload_uses_array_input() {
        let payload = embedding_payload("text-embedding-3-large", &["first", "second"]);

        assert_eq!(payload["model"], "text-embedding-3-large");
        assert_eq!(payload["input"][0], "first");
        assert_eq!(payload["input"][1], "second");
    }

    #[test]
    fn validate_embedding_texts_allows_empty_batch() {
        assert!(validate_embedding_texts(&[]).is_ok());
    }

    #[test]
    fn validate_embedding_texts_rejects_blank_entries() {
        let error = validate_embedding_texts(&["first", "  "]).unwrap_err();

        assert_eq!(error, EmbeddingError::BlankInput { index: Some(1) });
    }

    #[test]
    fn parse_embedding_response_restores_response_order_by_index() {
        let response = json!({
            "data": [
                { "index": 1, "embedding": [0.3, 0.4] },
                { "index": 0, "embedding": [0.1, 0.2] }
            ]
        });

        let embeddings = parse_embedding_response(response, 2, 2).unwrap();

        assert_eq!(embeddings, vec![vec![0.1, 0.2], vec![0.3, 0.4]]);
    }

    #[test]
    fn parse_embedding_response_rejects_count_mismatch() {
        let response = json!({
            "data": [
                { "index": 0, "embedding": [0.1, 0.2] }
            ]
        });

        let error = parse_embedding_response(response, 2, 2).unwrap_err();

        assert_eq!(
            error,
            EmbeddingError::CountMismatch {
                expected: 2,
                actual: 1,
            }
        );
    }

    #[test]
    fn parse_embedding_response_rejects_dimension_mismatch() {
        let response = json!({
            "data": [
                { "index": 0, "embedding": [0.1] }
            ]
        });

        let error = parse_embedding_response(response, 1, 2).unwrap_err();

        assert_eq!(
            error,
            EmbeddingError::DimensionMismatch {
                index: 0,
                expected: 2,
                actual: 1,
            }
        );
    }

    #[test]
    fn parse_embedding_response_rejects_duplicate_index() {
        let response = json!({
            "data": [
                { "index": 0, "embedding": [0.1, 0.2] },
                { "index": 0, "embedding": [0.3, 0.4] }
            ]
        });

        let error = parse_embedding_response(response, 2, 2).unwrap_err();

        assert_eq!(error, EmbeddingError::DuplicateIndex { index: 0 });
    }

    #[test]
    fn parse_embedding_response_rejects_non_numeric_values() {
        let response = json!({
            "data": [
                { "index": 0, "embedding": [0.1, "bad"] }
            ]
        });

        let error = parse_embedding_response(response, 1, 2).unwrap_err();

        assert_eq!(
            error,
            EmbeddingError::NonNumericValue {
                index: 0,
                component: 1,
            }
        );
    }

    #[tokio::test]
    async fn bulk_generate_embeddings_sends_one_array_request_for_multiple_inputs() {
        let transport = Arc::new(RecordingTransport::default());
        transport.enqueue_success_response(2, 3072);
        let provider = create_test_provider(transport.clone());

        let result = provider
            .bulk_generate_embeddings(&["first", "second"])
            .await
            .unwrap();

        assert_eq!(result.len(), 2);
        let requests = transport.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0]["input"][0], "first");
        assert_eq!(requests[0]["input"][1], "second");
    }

    #[tokio::test]
    async fn bulk_generate_embeddings_splits_by_documented_input_count_limit() {
        let transport = Arc::new(RecordingTransport::default());
        transport.enqueue_success_response(MAX_OPENAI_EMBEDDING_INPUTS, 3072);
        transport.enqueue_success_response(1, 3072);
        let provider = create_test_provider(transport.clone());
        let texts = (0..=MAX_OPENAI_EMBEDDING_INPUTS)
            .map(|index| format!("text {index}"))
            .collect::<Vec<_>>();
        let text_refs = texts.iter().map(String::as_str).collect::<Vec<_>>();

        let result = provider.bulk_generate_embeddings(&text_refs).await.unwrap();

        assert_eq!(result.len(), MAX_OPENAI_EMBEDDING_INPUTS + 1);
        let requests = transport.requests();
        assert_eq!(requests.len(), 2);
        assert_eq!(
            requests[0]["input"].as_array().unwrap().len(),
            MAX_OPENAI_EMBEDDING_INPUTS
        );
        assert_eq!(requests[1]["input"].as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn bulk_generate_embeddings_returns_empty_without_request() {
        let transport = Arc::new(RecordingTransport::default());
        let provider = create_test_provider(transport.clone());

        let result = provider.bulk_generate_embeddings(&[]).await.unwrap();

        assert!(result.is_empty());
        assert!(transport.requests().is_empty());
    }

    #[tokio::test]
    async fn bulk_generate_embeddings_rejects_blank_entry_without_request() {
        let transport = Arc::new(RecordingTransport::default());
        let provider = create_test_provider(transport.clone());

        let error = provider
            .bulk_generate_embeddings(&["first", " "])
            .await
            .unwrap_err();

        assert_eq!(error, EmbeddingError::BlankInput { index: Some(1) });
        assert!(transport.requests().is_empty());
    }

    #[derive(Default)]
    struct RecordingTransport {
        requests: Mutex<Vec<serde_json::Value>>,
        responses: Mutex<Vec<OpenAIEmbeddingHttpResponse>>,
    }

    impl RecordingTransport {
        fn enqueue_success_response(&self, count: usize, vector_size: usize) {
            let data = (0..count)
                .map(|index| {
                    json!({
                        "index": index,
                        "embedding": vec![index as f32; vector_size],
                    })
                })
                .collect::<Vec<_>>();
            self.responses
                .lock()
                .expect("responses mutex poisoned")
                .push(OpenAIEmbeddingHttpResponse {
                    status: StatusCode::OK,
                    body: json!({ "data": data }).to_string(),
                });
        }

        fn requests(&self) -> Vec<serde_json::Value> {
            self.requests
                .lock()
                .expect("requests mutex poisoned")
                .clone()
        }
    }

    #[async_trait]
    impl OpenAIEmbeddingTransport for RecordingTransport {
        async fn send_embeddings_request(
            &self,
            _api_key: &str,
            payload: serde_json::Value,
        ) -> Result<OpenAIEmbeddingHttpResponse, EmbeddingError> {
            self.requests
                .lock()
                .expect("requests mutex poisoned")
                .push(payload);
            let response = self
                .responses
                .lock()
                .expect("responses mutex poisoned")
                .remove(0);
            Ok(response)
        }
    }
}

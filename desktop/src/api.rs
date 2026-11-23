use serde_json::Value;

#[derive(Clone)]
pub struct ApiClient {
    base_url: String,
    token: Option<String>,
    http: reqwest::blocking::Client,
}

impl ApiClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            token: None,
            http: reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .expect("failed to build HTTP client"),
        }
    }

    pub fn set_base_url(&mut self, base_url: impl Into<String>) {
        self.base_url = base_url.into();
    }

    pub fn set_token(&mut self, token: Option<String>) {
        self.token = token;
    }

    pub fn get(&self, path: &str) -> Result<Value, String> {
        self.request(reqwest::Method::GET, path, None)
    }

    pub fn post(&self, path: &str, body: &Value) -> Result<Value, String> {
        self.request(reqwest::Method::POST, path, Some(body))
    }

    pub fn delete(&self, path: &str) -> Result<Value, String> {
        self.request(reqwest::Method::DELETE, path, None)
    }

    fn request(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<&Value>,
    ) -> Result<Value, String> {
        let url = format!("{}{}", self.base_url.trim_end_matches('/'), path);
        let mut request = self
            .http
            .request(method, url)
            .header("Accept", "application/json");

        if let Some(token) = &self.token {
            request = request.bearer_auth(token);
        }
        if let Some(body) = body {
            request = request.json(body);
        }

        let response = request
            .send()
            .map_err(|error| format!("request failed: {error}"))?;
        let status = response.status();
        let text = response
            .text()
            .map_err(|error| format!("could not read response body: {error}"))?;

        if !status.is_success() {
            let detail = if text.trim().is_empty() {
                status.to_string()
            } else {
                text
            };
            return Err(format!("HTTP {status}: {detail}"));
        }

        if text.trim().is_empty() {
            Ok(Value::Null)
        } else {
            serde_json::from_str(&text)
                .map_err(|error| format!("invalid JSON response: {error}; body: {text}"))
        }
    }
}

/// Minimal percent-encoding for query and path values.
pub fn encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

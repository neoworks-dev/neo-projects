//! GitHub's GraphQL API: pulling repositories into the cache and pushing the outbox.

mod pull;
mod push;

pub use pull::{PullReport, pull, pull_comments};
pub use push::{PushReport, push};

use serde_json::{Value, json};

use crate::{Error, Result};

const ENDPOINT: &str = "https://api.github.com/graphql";

/// Sends one GraphQL request and returns GitHub's whole answer. Tests stand in for the network.
pub trait Transport {
    fn post(&self, body: &Value) -> Result<Value>;
}

/// The real network, with your token.
pub struct Http {
    token: String,
    agent: ureq::Agent,
}

impl Http {
    pub fn new(token: impl Into<String>) -> Http {
        let agent = ureq::AgentBuilder::new().timeout(std::time::Duration::from_secs(30)).build();
        Http { token: token.into(), agent }
    }

    /// The token `gh` holds for `login`, or for its active account: what the app uses when `gh`
    /// is installed. With several accounts the login matters: an organization's issue fields are
    /// only visible to its members.
    pub fn from_gh(login: Option<&str>) -> Result<Http> {
        let mut args = vec!["auth", "token"];
        if let Some(login) = login {
            args.extend(["--user", login]);
        }
        let out = std::process::Command::new("gh")
            .args(&args)
            .output()
            .map_err(|e| Error::Other(format!("couldn't run gh: {e}")))?;
        if !out.status.success() {
            let who = login.map_or(String::new(), |l| format!(" as {l}"));
            return Err(Error::Other(format!("gh is not logged in{who} (gh auth login)")));
        }
        Ok(Http::new(String::from_utf8_lossy(&out.stdout).trim()))
    }
}

impl Transport for Http {
    fn post(&self, body: &Value) -> Result<Value> {
        let response = self
            .agent
            .post(ENDPOINT)
            .set("Authorization", &format!("bearer {}", self.token))
            .set("User-Agent", "neoworks-projects")
            .send_json(body.clone());
        match response {
            Ok(r) => r.into_json().map_err(|e| Error::Http(e.to_string())),
            Err(ureq::Error::Status(code, r)) => {
                let text = r.into_string().unwrap_or_default();
                Err(Error::Http(format!("{code}: {}", text.chars().take(300).collect::<String>())))
            }
            Err(e) => Err(Error::Http(e.to_string())),
        }
    }
}

pub struct Client<T: Transport> {
    transport: T,
}

impl<T: Transport> Client<T> {
    pub fn new(transport: T) -> Client<T> {
        Client { transport }
    }

    /// Runs a query or mutation and returns its `data`; GraphQL errors become [`Error::GraphQl`].
    pub fn run(&self, query: &str, variables: Value) -> Result<Value> {
        let answer = self.transport.post(&json!({"query": query, "variables": variables}))?;
        if let Some(errors) = answer.get("errors").and_then(Value::as_array).filter(|e| !e.is_empty()) {
            let messages = errors
                .iter()
                .map(|e| e.get("message").and_then(Value::as_str).unwrap_or("unknown error").to_string())
                .collect();
            return Err(Error::GraphQl(messages));
        }
        Ok(answer.get("data").cloned().unwrap_or(Value::Null))
    }
}

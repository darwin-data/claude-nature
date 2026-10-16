use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::footprint::{Factors, Footprint, Tokens, footprint};

#[derive(Deserialize)]
struct Line {
    #[serde(rename = "type")]
    kind: Option<String>,
    timestamp: Option<String>,
    #[serde(rename = "requestId")]
    request_id: Option<String>,
    #[serde(rename = "sessionId")]
    session_id: Option<String>,
    cwd: Option<String>,
    message: Option<Message>,
}

#[derive(Deserialize)]
struct Message {
    id: Option<String>,
    model: Option<String>,
    usage: Option<Usage>,
}

#[derive(Deserialize, Default)]
struct Usage {
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    cache_creation_input_tokens: u64,
    #[serde(default)]
    cache_read_input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
}

struct Request {
    model: String,
    session_id: String,
    tokens: Tokens,
}

#[derive(Default)]
pub struct Filter {
    /// Inclusive lower bound, an ISO 8601 date or datetime.
    pub since: Option<String>,
    /// Keeps only the sessions whose working directory contains this text.
    pub project: Option<String>,
}

#[derive(Debug, Default)]
pub struct Summary {
    pub sessions: usize,
    pub requests: usize,
    pub tokens: Tokens,
    pub footprint: Footprint,
    pub by_family: BTreeMap<String, (Tokens, Footprint)>,
    pub excluded_requests: usize,
}

/// Every `.jsonl` under `root`, subagent transcripts included.
pub fn transcripts(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "jsonl") {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

/// Sums the usage of `files`. Assistant messages are deduplicated by
/// `(message.id, requestId)`, keeping the last occurrence: streaming writes one
/// message several times with a growing `output_tokens`, and resumed sessions
/// replay earlier messages. This is the rule of claude-carbon and ccusage.
pub fn summarise(factors: &Factors, files: &[PathBuf], filter: &Filter) -> Summary {
    let mut requests: HashMap<(String, String), Request> = HashMap::new();
    for file in files {
        let Ok(handle) = fs::File::open(file) else {
            continue;
        };
        for raw in BufReader::new(handle).lines().map_while(Result::ok) {
            if !raw.contains("\"usage\"") {
                continue;
            }
            let Ok(line) = serde_json::from_str::<Line>(&raw) else {
                continue;
            };
            if line.kind.as_deref() != Some("assistant") {
                continue;
            }
            if let (Some(since), Some(ts)) = (&filter.since, &line.timestamp)
                && ts.as_str() < since.as_str()
            {
                continue;
            }
            if let Some(project) = &filter.project
                && !line
                    .cwd
                    .as_deref()
                    .unwrap_or_default()
                    .contains(project.as_str())
            {
                continue;
            }
            let Some(message) = line.message else {
                continue;
            };
            let Some(usage) = message.usage else { continue };
            let key = (
                message.id.unwrap_or_default(),
                line.request_id.unwrap_or_default(),
            );
            requests.insert(
                key,
                Request {
                    model: message.model.unwrap_or_default(),
                    session_id: line.session_id.unwrap_or_default(),
                    tokens: Tokens {
                        input: usage.input_tokens,
                        cache_write: usage.cache_creation_input_tokens,
                        cache_read: usage.cache_read_input_tokens,
                        output: usage.output_tokens,
                    },
                },
            );
        }
    }

    let mut summary = Summary::default();
    let mut sessions = HashSet::new();
    for request in requests.values() {
        let Some(family) = factors.family_of(&request.model) else {
            summary.excluded_requests += 1;
            continue;
        };
        let fp = footprint(factors, family, request.tokens);
        sessions.insert(request.session_id.as_str());
        summary.requests += 1;
        summary.tokens += request.tokens;
        summary.footprint += fp;
        let entry = summary.by_family.entry(family.to_owned()).or_default();
        entry.0 += request.tokens;
        entry.1 += fp;
    }
    summary.sessions = sessions.len();
    summary
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<PathBuf> {
        transcripts(Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/projects"
        )))
    }

    #[test]
    fn streaming_replays_are_counted_once_at_their_last_value() {
        let s = summarise(&Factors::embedded(), &fixture(), &Filter::default());
        assert_eq!(s.requests, 3);
        assert_eq!(s.sessions, 2);
        assert_eq!(s.excluded_requests, 1);
        assert_eq!(
            s.tokens,
            Tokens {
                input: 12,
                cache_write: 1_000,
                cache_read: 50_000,
                output: 1_700
            }
        );
    }

    #[test]
    fn since_and_project_filters() {
        let f = Factors::embedded();
        let since = Filter {
            since: Some("2026-10-02".into()),
            ..Filter::default()
        };
        assert_eq!(summarise(&f, &fixture(), &since).requests, 1);
        let project = Filter {
            project: Some("other".into()),
            ..Filter::default()
        };
        assert_eq!(summarise(&f, &fixture(), &project).requests, 1);
    }
}

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

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

#[derive(Deserialize)]
struct UserLine {
    timestamp: Option<String>,
    #[serde(rename = "isMeta", default)]
    is_meta: bool,
    origin: Option<Origin>,
    message: Option<UserMessage>,
}

#[derive(Deserialize)]
struct Origin {
    kind: Option<String>,
}

#[derive(Deserialize)]
struct UserMessage {
    content: Option<serde_json::Value>,
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

/// One API request, as the transcript records it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    pub message_id: String,
    pub request_id: String,
    pub model: String,
    pub session_id: String,
    pub timestamp: String,
    pub cwd: String,
    pub tokens: Tokens,
}

/// What one transcript file holds: its requests in file order, and the time of its last
/// prompt typed by a person.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Scan {
    pub records: Vec<Record>,
    pub last_prompt: Option<String>,
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

/// Reads one transcript. A line without `"usage"` and without a user prompt is skipped
/// before it is parsed: tool output fills most of a transcript.
pub fn scan(file: &Path) -> Scan {
    let mut out = Scan::default();
    let Ok(handle) = fs::File::open(file) else {
        return out;
    };
    for raw in BufReader::new(handle).lines().map_while(Result::ok) {
        if raw.contains("\"usage\"") {
            if let Some(record) = record(&raw) {
                out.records.push(record);
            }
        } else if raw.contains("\"type\":\"user\"")
            && !raw.contains("\"tool_result\"")
            && let Some(ts) = prompt_time(&raw)
        {
            out.last_prompt = Some(ts);
        }
    }
    out
}

fn record(raw: &str) -> Option<Record> {
    let line = serde_json::from_str::<Line>(raw).ok()?;
    if line.kind.as_deref() != Some("assistant") {
        return None;
    }
    let message = line.message?;
    let usage = message.usage?;
    Some(Record {
        message_id: message.id.unwrap_or_default(),
        request_id: line.request_id.unwrap_or_default(),
        model: message.model.unwrap_or_default(),
        session_id: line.session_id.unwrap_or_default(),
        timestamp: line.timestamp.unwrap_or_default(),
        cwd: line.cwd.unwrap_or_default(),
        tokens: Tokens {
            input: usage.input_tokens,
            cache_write: usage.cache_creation_input_tokens,
            cache_read: usage.cache_read_input_tokens,
            output: usage.output_tokens,
        },
    })
}

/// A prompt is a user line that a person wrote: `origin.kind` is `human` where Claude Code
/// writes it, and older transcripts carry the prompt as a plain string. Meta lines
/// (command output, reminders) are not prompts.
fn prompt_time(raw: &str) -> Option<String> {
    let line = serde_json::from_str::<UserLine>(raw).ok()?;
    if line.is_meta {
        return None;
    }
    let human = line.origin.and_then(|o| o.kind).as_deref() == Some("human");
    let text = line
        .message
        .and_then(|m| m.content)
        .is_some_and(|c| c.is_string());
    (human || text).then_some(line.timestamp?)
}

/// Sums the usage of `files`. Assistant messages are deduplicated by
/// `(message.id, requestId)`, keeping the last occurrence: streaming writes one
/// message several times with a growing `output_tokens`, and resumed sessions
/// replay earlier messages. This is the rule of claude-carbon and ccusage.
pub fn summarise(factors: &Factors, files: &[PathBuf], filter: &Filter) -> Summary {
    let scans: Vec<Scan> = files.iter().map(|f| scan(f)).collect();
    summarise_records(factors, scans.iter().flat_map(|s| &s.records), filter)
}

pub fn summarise_records<'a>(
    factors: &Factors,
    records: impl IntoIterator<Item = &'a Record>,
    filter: &Filter,
) -> Summary {
    let mut requests: HashMap<(&str, &str), &Record> = HashMap::new();
    for record in records {
        if let Some(since) = &filter.since
            && record.timestamp.as_str() < since.as_str()
        {
            continue;
        }
        if let Some(project) = &filter.project
            && !record.cwd.contains(project.as_str())
        {
            continue;
        }
        requests.insert(
            (record.message_id.as_str(), record.request_id.as_str()),
            record,
        );
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

    #[test]
    fn the_last_prompt_is_the_last_line_a_person_typed() {
        let scan = scan(Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/projects/-tmp-demo/sess-a.jsonl"
        )));
        assert_eq!(
            scan.last_prompt.as_deref(),
            Some("2026-10-01T10:00:00.000Z")
        );
        assert_eq!(
            prompt_time(
                r#"{"type":"user","isMeta":true,"timestamp":"t","message":{"content":"x"}}"#
            ),
            None
        );
        assert_eq!(
            prompt_time(
                r#"{"type":"user","timestamp":"t","origin":{"kind":"human"},"message":{"content":[{"type":"text","text":"x"}]}}"#
            ),
            Some("t".into())
        );
        assert_eq!(
            prompt_time(
                r#"{"type":"user","timestamp":"t","message":{"content":[{"type":"text","text":"[Request interrupted by user]"}]}}"#
            ),
            None
        );
    }
}

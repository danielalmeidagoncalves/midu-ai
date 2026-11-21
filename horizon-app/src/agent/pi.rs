//! The Pi harness: `pi --mode rpc` as a child process, JSONL in and out.
//!
//! Pi's RPC protocol is strict JSONL on stdin/stdout; diagnostics go to stderr.
//! The reader thread translates session events into transcript items as they
//! arrive (deltas are appended, then replaced by the authoritative text at the
//! end of a block). Nothing here blocks the UI thread.

use std::io::{BufRead, BufReader};
use std::process::{Child, ChildStderr, ChildStdout, Command, Stdio};

use serde_json::{json, Value};

use super::{ChatItem, Service, Status};

/// Start one harness process in the vault. The manual is passed as a file path,
/// so the harness owns the reading of it and the vault stays clean.
pub fn spawn(
    config: &super::AgentConfig,
    cwd: &std::path::Path,
    manual: Option<&std::path::Path>,
) -> Result<Child, String> {
    let mut command = Command::new(&config.command);
    command
        .current_dir(cwd)
        .arg("--mode")
        .arg("rpc")
        // One session per vault, so closing and reopening Horizon continues the
        // conversation Pi remembers instead of starting from nothing.
        .arg("--session-id")
        .arg("horizon")
        .arg("--name")
        .arg("Horizon")
        .env("NO_COLOR", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(manual) = manual {
        command.arg("--append-system-prompt").arg(manual);
    }
    if !config.provider.trim().is_empty() {
        command.arg("--provider").arg(config.provider.trim());
    }
    if !config.model.trim().is_empty() {
        command.arg("--model").arg(config.model.trim());
    }
    if !config.thinking.trim().is_empty() {
        command.arg("--thinking").arg(config.thinking.trim());
    }
    if !config.write {
        // A read-only conversation: no write, edit or shell tool is declared, so
        // the model cannot change a file even by accident. This is honest: bash
        // is excluded, because bash is a write tool in disguise.
        command.arg("--tools").arg("read,grep,find,ls");
    }
    command.spawn().map_err(|err| {
        format!(
            "Could not start `{}` in {}: {err}. Install Pi or set another command in the brief.",
            config.command,
            cwd.display()
        )
    })
}

/// Read stdout until the harness exits. Records are split on LF only, per the
/// protocol; a `\r` before the LF is tolerated.
pub fn read_stdout(service: &'static Service, stdout: ChildStdout) {
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {}
            Err(err) => {
                super::fail(
                    service,
                    format!("The harness's output could not be read: {err}"),
                );
                break;
            }
        }
        let record = line.trim_end_matches(['\n', '\r']);
        if record.trim().is_empty() {
            continue;
        }
        let Ok(event) = serde_json::from_str::<Value>(record) else {
            super::fail(
                service,
                format!("The harness sent a record Horizon could not read: {record}"),
            );
            continue;
        };
        match event["type"].as_str().unwrap_or("") {
            "extension_ui_request" => answer_extension(service, &event),
            _ => {
                let outcome = {
                    let mut state = match service.state.lock() {
                        Ok(state) => state,
                        Err(_) => return,
                    };
                    apply_event(&mut state, &event)
                };
                if matches!(outcome, Outcome::Settled) {
                    super::settle(service);
                }
            }
        }
    }
    // Stdout ended: the child is on its way out. Its exit status tells the
    // difference between a clean quit and a crash; stderr has the reason.
    let code = service
        .io
        .lock()
        .ok()
        .and_then(|mut io| {
            io.child
                .as_mut()
                .and_then(|child| child.try_wait().ok().flatten())
        })
        .and_then(|status| status.code());
    let stderr = service
        .stderr
        .lock()
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    if !stderr.is_empty() && !matches!(code, Some(0)) {
        super::fail(service, stderr);
    } else {
        super::saw_exit(service, code);
    }
}

/// Diagnostics go to stderr. Keep the last line: it is what a start failure or a
/// crash actually said, and the panel can show it instead of a generic "error".
pub fn read_stderr(service: &'static Service, stderr: ChildStderr) {
    let mut reader = BufReader::new(stderr);
    let mut line = String::new();
    while reader.read_line(&mut line).unwrap_or(0) > 0 {
        let text = line.trim().to_string();
        line.clear();
        if text.is_empty() {
            continue;
        }
        if let Ok(mut kept) = service.stderr.lock() {
            *kept = text;
        }
    }
}

/// Extension dialogs block the harness until someone answers. There is no modal
/// in Horizon's design, so a dialog is declined and the decline is printed as a
/// notice: the agent then has to work without the permission it asked for.
fn answer_extension(service: &'static Service, event: &Value) {
    let method = event["method"].as_str().unwrap_or("");
    let id = event["id"].clone();
    if matches!(method, "select" | "confirm" | "input" | "editor") {
        let _ = service.write(json!({
            "type": "extension_ui_response",
            "id": id,
            "cancelled": true
        }));
        super::notice(
            service,
            format!(
                "An extension asked for input ({}). Horizon has no modal, so it was declined.",
                method
            ),
            true,
        );
    } else if method == "notify" {
        if let Some(message) = event["message"].as_str() {
            if let Ok(mut state) = service.state.lock() {
                state.push(ChatItem::Notice {
                    text: message.to_string(),
                    error: event["notifyType"].as_str() == Some("error"),
                });
            }
        }
    }
    // setStatus / setWidget / setTitle and the rest are TUI concerns; ignore.
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Nothing,
    Settled,
}

/// Translate one session event. Pure over the view, which is what makes it
/// testable without a child process.
pub fn apply_event(view: &mut super::AgentView, event: &Value) -> Outcome {
    match event["type"].as_str().unwrap_or("") {
        "agent_start" => {
            view.busy = true;
            view.status = Status::Working;
            view.detail = "Pi is working.".into();
        }
        "turn_start" => {
            view.busy = true;
            if matches!(view.status, Status::Ready | Status::Starting) {
                view.status = Status::Working;
            }
        }
        "message_start" => {
            let role = event["message"]["role"].as_str().unwrap_or("");
            if role == "assistant" {
                view.streaming.clear();
                view.thinking.clear();
            }
        }
        "message_update" => {
            let update = &event["assistantMessageEvent"];
            match update["type"].as_str().unwrap_or("") {
                "text_delta" => {
                    if let Some(delta) = update["delta"].as_str() {
                        view.streaming.push_str(delta);
                    }
                }
                "text_end" => {
                    let text = update["content"]
                        .as_str()
                        .map(str::to_string)
                        .unwrap_or_else(|| std::mem::take(&mut view.streaming));
                    view.streaming.clear();
                    if !text.trim().is_empty() {
                        view.push(ChatItem::Assistant { text });
                    }
                }
                "thinking_delta" => {
                    if let Some(delta) = update["delta"].as_str() {
                        view.thinking.push_str(delta);
                    }
                }
                "thinking_end" => {
                    if let Some(text) = update["content"].as_str() {
                        view.thinking = text.to_string();
                    }
                    view.flush_thinking();
                }
                "toolcall_end" => {
                    let call = &update["toolCall"];
                    let name = call["name"].as_str().unwrap_or("tool").to_string();
                    let id = call["id"].as_str().unwrap_or_default().to_string();
                    let detail = tool_detail(&name, &call["arguments"]);
                    upsert_tool(view, &id, name, detail, None);
                }
                _ => {}
            }
        }
        "message_end" => {
            let message = &event["message"];
            if message["role"].as_str() == Some("assistant") {
                let had_stream = !view.streaming.trim().is_empty();
                view.flush_stream();
                view.flush_thinking();
                // Some providers deliver the finished blocks without deltas. The
                // final message is authoritative either way; take it only when
                // the stream did not already produce it, so nothing is doubled.
                if !had_stream {
                    if let Some(blocks) = message["content"].as_array() {
                        for block in blocks {
                            match block["type"].as_str() {
                                Some("text") => {
                                    let text = block["text"].as_str().unwrap_or("").trim();
                                    if !text.is_empty() && !recently_said(view, text) {
                                        view.push(ChatItem::Assistant {
                                            text: text.to_string(),
                                        });
                                    }
                                }
                                Some("thinking") => {
                                    let text = block["thinking"].as_str().unwrap_or("").trim();
                                    if !text.is_empty() && !recently_thought(view, text) {
                                        view.push(ChatItem::Thinking {
                                            text: text.to_string(),
                                        });
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
                match message["stopReason"].as_str() {
                    Some("error") => {
                        let text = message["errorMessage"]
                            .as_str()
                            .unwrap_or("The model returned an error.");
                        view.push(ChatItem::Notice {
                            text: text.to_string(),
                            error: true,
                        });
                    }
                    Some("aborted") => view.push(ChatItem::Notice {
                        text: "Stopped.".into(),
                        error: false,
                    }),
                    _ => {}
                }
            }
        }
        "tool_execution_start" => {
            let name = event["toolName"].as_str().unwrap_or("tool").to_string();
            let id = event["toolCallId"].as_str().unwrap_or_default().to_string();
            let detail = tool_detail(&name, &event["args"]);
            upsert_tool(view, &id, name, detail, None);
        }
        "tool_execution_end" => {
            let id = event["toolCallId"].as_str().unwrap_or_default().to_string();
            let failed = event["isError"].as_bool().unwrap_or(false);
            let out = result_summary(&event["result"]);
            upsert_tool_done(view, &id, out, failed);
        }
        "agent_end" => {
            if event["willRetry"].as_bool().unwrap_or(false) {
                view.push(ChatItem::Notice {
                    text: "Recovering and trying again…".into(),
                    error: false,
                });
            }
        }
        "agent_settled" => {
            view.flush_stream();
            view.flush_thinking();
            view.busy = false;
            if !matches!(view.status, Status::Failed | Status::Stopped) {
                view.status = Status::Ready;
                view.detail = "Ready.".into();
            }
            return Outcome::Settled;
        }
        "auto_retry_start" => {
            let attempt = event["attempt"].as_u64().unwrap_or(1);
            let max = event["maxAttempts"].as_u64().unwrap_or(1);
            let reason = event["errorMessage"]
                .as_str()
                .unwrap_or("the provider stumbled");
            view.push(ChatItem::Notice {
                text: format!("Retrying after {reason} ({attempt}/{max})…"),
                error: false,
            });
        }
        "auto_retry_end" => {
            if !event["success"].as_bool().unwrap_or(true) {
                let text = event["finalError"]
                    .as_str()
                    .unwrap_or("The model could not answer.");
                view.push(ChatItem::Notice {
                    text: text.to_string(),
                    error: true,
                });
            }
        }
        "compaction_start" => {
            view.detail = "Compacting the conversation…".into();
        }
        "compaction_end" => {
            view.push(ChatItem::Notice {
                text: "The conversation was compacted; the earlier turns are now a summary.".into(),
                error: false,
            });
        }
        "extension_error" => {
            let text = event["error"].as_str().unwrap_or("An extension failed.");
            view.push(ChatItem::Notice {
                text: text.to_string(),
                error: true,
            });
        }
        "response" => {
            if event["command"].as_str() == Some("get_state")
                && event["success"].as_bool().unwrap_or(false)
            {
                view.model = model_label(&event["data"]["model"]);
            }
            if !event["success"].as_bool().unwrap_or(true) {
                if let Some(error) = event["error"].as_str() {
                    view.push(ChatItem::Notice {
                        text: error.to_string(),
                        error: true,
                    });
                }
            }
        }
        _ => {}
    }
    Outcome::Nothing
}

fn recently_said(view: &super::AgentView, text: &str) -> bool {
    view.items
        .iter()
        .rev()
        .take(4)
        .any(|item| matches!(item, ChatItem::Assistant { text: said } if said.trim() == text))
}

fn recently_thought(view: &super::AgentView, text: &str) -> bool {
    view.items
        .iter()
        .rev()
        .take(4)
        .any(|item| matches!(item, ChatItem::Thinking { text: said } if said.trim() == text))
}

fn model_label(model: &Value) -> String {
    // The display name is what a person recognises; provider/id is the fallback
    // for a harness that reports only identifiers.
    if let Some(name) = model["name"].as_str().filter(|name| !name.is_empty()) {
        return name.to_string();
    }
    let id = model["id"].as_str().unwrap_or("");
    let provider = model["provider"].as_str().unwrap_or("");
    match (provider.is_empty(), id.is_empty()) {
        (false, false) => format!("{provider}/{id}"),
        (true, false) => id.to_string(),
        (false, true) => provider.to_string(),
        (true, true) => String::new(),
    }
}

fn upsert_tool(
    view: &mut super::AgentView,
    id: &str,
    name: String,
    detail: String,
    out: Option<String>,
) {
    if !id.is_empty() {
        if let Some(ChatItem::Tool {
            name: existing_name,
            detail: existing_detail,
            out: existing_out,
            ..
        }) = view
            .items
            .iter_mut()
            .rev()
            .find(|item| matches!(item, ChatItem::Tool { id: item_id, .. } if item_id == id))
        {
            if !name.is_empty() {
                *existing_name = name;
            }
            if !detail.is_empty() {
                *existing_detail = detail;
            }
            if let Some(out) = out {
                *existing_out = out;
            }
            return;
        }
    }
    view.push(ChatItem::Tool {
        id: id.to_string(),
        name,
        detail,
        out: out.unwrap_or_default(),
        done: false,
        failed: false,
    });
}

fn upsert_tool_done(view: &mut super::AgentView, id: &str, out: String, failed: bool) {
    let found = view.items.iter_mut().rev().find_map(|item| match item {
        ChatItem::Tool {
            id: item_id,
            out: item_out,
            done,
            failed: item_failed,
            ..
        } if item_id == id => Some((item_out, done, item_failed)),
        _ => None,
    });
    match found {
        Some((item_out, done, item_failed)) => {
            *item_out = out;
            *done = true;
            *item_failed = failed;
        }
        None => {
            view.push(ChatItem::Tool {
                id: id.to_string(),
                name: "tool".into(),
                detail: String::new(),
                out,
                done: true,
                failed,
            });
        }
    }
}

/// One line per tool: the path it touched or the command it ran. The transcript
/// is read at a glance, so the detail is not a JSON dump.
pub fn tool_detail(name: &str, args: &Value) -> String {
    let text = |key: &str| args[key].as_str().map(str::to_string);
    let detail = match name {
        "bash" => text("command"),
        "read" | "write" | "edit" | "grep" | "find" | "ls" => text("path")
            .or_else(|| text("pattern"))
            .or_else(|| text("glob")),
        _ => None,
    }
    .or_else(|| {
        args.as_object().and_then(|object| {
            object
                .iter()
                .find_map(|(_, value)| value.as_str().map(str::to_string))
        })
    })
    .unwrap_or_default();
    truncate(&detail.replace('\n', " "), 140)
}

fn result_summary(result: &Value) -> String {
    let text = result["content"]
        .as_array()
        .and_then(|blocks| {
            blocks.iter().find_map(|block| {
                if block["type"].as_str() == Some("text") {
                    block["text"].as_str()
                } else {
                    None
                }
            })
        })
        .or_else(|| result.as_str())
        .unwrap_or("");
    truncate(&text.replace('\n', " · "), 220)
}

fn truncate(text: &str, limit: usize) -> String {
    let text = text.trim();
    if text.chars().count() <= limit {
        return text.to_string();
    }
    let mut out: String = text.chars().take(limit.saturating_sub(1)).collect();
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::{AgentView, ChatItem};

    fn view() -> AgentView {
        AgentView::default()
    }

    #[test]
    fn streaming_text_becomes_one_authoritative_turn() {
        let mut view = view();
        let start = jsonrpc("message_start", json!({"role": "assistant", "content": []}));
        apply_event(&mut view, &start);
        for delta in ["Two ", "things ", "tomorrow."] {
            apply_event(
                &mut view,
                &json!({
                    "type": "message_update",
                    "assistantMessageEvent": {"type": "text_delta", "contentIndex": 0, "delta": delta}
                }),
            );
        }
        assert_eq!(view.streaming, "Two things tomorrow.");
        apply_event(
            &mut view,
            &json!({
                "type": "message_update",
                "assistantMessageEvent": {
                    "type": "text_end",
                    "contentIndex": 0,
                    "content": "Two things tomorrow."
                }
            }),
        );
        assert!(view.streaming.is_empty());
        assert_eq!(
            view.items,
            vec![ChatItem::Assistant {
                text: "Two things tomorrow.".into()
            }]
        );
    }

    #[test]
    fn a_tool_call_is_one_item_that_finishes_in_place() {
        let mut view = view();
        apply_event(
            &mut view,
            &json!({
                "type": "tool_execution_start",
                "toolCallId": "call_1",
                "toolName": "write",
                "args": {"path": "notes/ask-about-the-bond-abcd1234.md"}
            }),
        );
        assert_eq!(view.items.len(), 1);
        apply_event(
            &mut view,
            &json!({
                "type": "tool_execution_end",
                "toolCallId": "call_1",
                "toolName": "write",
                "result": {"content": [{"type": "text", "text": "Wrote 1 file\nsecond line"}]},
                "isError": false
            }),
        );
        assert_eq!(view.items.len(), 1, "the call is updated, not duplicated");
        match &view.items[0] {
            ChatItem::Tool {
                name,
                detail,
                done,
                failed,
                out,
                ..
            } => {
                assert_eq!(name, "write");
                assert_eq!(detail, "notes/ask-about-the-bond-abcd1234.md");
                assert!(*done);
                assert!(!*failed);
                assert_eq!(out, "Wrote 1 file · second line");
            }
            other => panic!("expected a tool item, got {other:?}"),
        }
    }

    #[test]
    fn settling_ends_the_run_and_asks_for_a_reconcile() {
        let mut view = view();
        view.busy = true;
        view.status = Status::Working;
        view.streaming = "one last line".into();
        let outcome = apply_event(
            &mut view,
            &json!({"type": "agent_settled", "aborted": false}),
        );
        assert_eq!(outcome, Outcome::Settled);
        assert!(!view.busy);
        assert_eq!(view.status, Status::Ready);
        assert_eq!(view.streaming, "");
        assert_eq!(
            view.items.last(),
            Some(&ChatItem::Assistant {
                text: "one last line".into()
            })
        );
    }

    #[test]
    fn get_state_names_the_model() {
        let mut named = view();
        apply_event(
            &mut named,
            &json!({
                "type": "response",
                "command": "get_state",
                "success": true,
                "data": {"model": {"provider": "anthropic", "id": "claude", "name": "the model"}}
            }),
        );
        assert_eq!(named.model, "the model");

        let mut qualified = view();
        apply_event(
            &mut qualified,
            &json!({
                "type": "response",
                "command": "get_state",
                "success": true,
                "data": {"model": {"provider": "anthropic", "id": "claude"}}
            }),
        );
        assert_eq!(qualified.model, "anthropic/claude");
    }

    #[test]
    fn a_finished_message_without_deltas_still_appears_once() {
        let mut state = view();
        let message = json!({ "role": "assistant", "content": [
            {"type": "thinking", "thinking": "a thought"},
            {"type": "text", "text": "Done."}
        ]});
        apply_event(
            &mut state,
            &json!({ "type": "message_end", "message": message }),
        );
        assert_eq!(state.items.len(), 2);
        // The same authoritative message arriving again must not double it.
        apply_event(
            &mut state,
            &json!({ "type": "message_end", "message": message }),
        );
        assert_eq!(state.items.len(), 2);
    }

    #[test]
    fn a_failed_model_message_is_a_visible_notice() {
        let mut view = view();
        apply_event(
            &mut view,
            &json!({
                "type": "message_end",
                "message": {"role": "assistant", "stopReason": "error", "errorMessage": "529 overloaded"}
            }),
        );
        assert_eq!(
            view.items,
            vec![ChatItem::Notice {
                text: "529 overloaded".into(),
                error: true
            }]
        );
    }

    #[test]
    fn tool_details_prefer_the_path_or_the_command() {
        assert_eq!(
            tool_detail("read", &json!({"path": "entries/a.md"})),
            "entries/a.md"
        );
        assert_eq!(
            tool_detail("bash", &json!({"command": "rg -n mum entries/"})),
            "rg -n mum entries/"
        );
        assert_eq!(
            tool_detail("grep", &json!({"pattern": "title: \"Mum\""})),
            "title: \"Mum\""
        );
        // Long commands are flattened and shortened: the line stays a line.
        let long = tool_detail("bash", &json!({"command": "x".repeat(400)}));
        assert!(long.chars().count() <= 140);
    }

    fn jsonrpc(kind: &str, extra: Value) -> Value {
        let mut value = json!({"type": kind});
        if let (Some(target), Some(extra)) = (value.as_object_mut(), extra.as_object()) {
            for (key, value) in extra {
                target.insert(key.clone(), value.clone());
            }
        }
        value
    }
}

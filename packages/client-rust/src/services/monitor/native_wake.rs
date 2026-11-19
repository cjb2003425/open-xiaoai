use std::collections::VecDeque;
use std::future::Future;

use serde::{Deserialize, Serialize};

use crate::base::AppError;
use crate::utils::shell::CommandResult;

use super::file::FileMonitorEvent;
use super::instruction::{LogMessage, Payload};

const MAX_DIALOG_IDS: usize = 64;
const SUPPRESS_NATIVE_COMMAND: &str = r#"killall tts_play.sh 2>/dev/null || true
killall miplayer 2>/dev/null || true
mphelper pause >/dev/null 2>&1 || true
ubus -t1 -S call pnshelper event_notify '{"src":3,"event":7}' >/dev/null 2>&1 || true
sleep 0.1
ubus -t1 -S call pnshelper event_notify '{"src":3,"event":8}' >/dev/null 2>&1"#;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeWakePayload {
    pub dialog_id: String,
    pub local_suppressed: bool,
    pub ack_played: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeWakePlan {
    pub dialog_id: String,
    pub acknowledge: bool,
}

#[derive(Debug, Default)]
pub struct DialogDeduper {
    dialog_ids: VecDeque<String>,
}

impl DialogDeduper {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn admit(&mut self, dialog_id: &str) -> bool {
        if dialog_id.is_empty() || self.dialog_ids.iter().any(|item| item == dialog_id) {
            return false;
        }
        if self.dialog_ids.len() == MAX_DIALOG_IDS {
            self.dialog_ids.pop_front();
        }
        self.dialog_ids.push_back(dialog_id.to_string());
        true
    }
}

pub fn native_wake_plan(
    event: &FileMonitorEvent,
    _connected: bool,
    deduper: &mut DialogDeduper,
) -> Option<NativeWakePlan> {
    let FileMonitorEvent::NewLine(line) = event else {
        return None;
    };
    let message = serde_json::from_str::<LogMessage>(line).ok()?;
    if message.header.namespace != "SpeechRecognizer"
        || message.header.name != "RecognizeResult"
        || message.header.dialog_id.is_empty()
    {
        return None;
    }
    let Payload::RecognizeResultPayload {
        is_final,
        is_vad_begin,
        results,
    } = message.payload
    else {
        return None;
    };
    if is_final
        || is_vad_begin
        || results.is_empty()
        || results.iter().any(|result| !result.text.trim().is_empty())
        || !deduper.admit(&message.header.dialog_id)
    {
        return None;
    }
    Some(NativeWakePlan {
        dialog_id: message.header.dialog_id,
        // L06A already plays its local native acknowledgement before this
        // marker is written. Adding another TTS acknowledgement produces a
        // duplicate "在" / "我在" response.
        acknowledge: false,
    })
}

pub async fn execute_native_wake<R, Fut>(
    plan: NativeWakePlan,
    mut runner: R,
) -> NativeWakePayload
where
    R: FnMut(String) -> Fut,
    Fut: Future<Output = Result<CommandResult, AppError>>,
{
    let suppressed = runner(SUPPRESS_NATIVE_COMMAND.to_string())
        .await
        .map(|result| result.exit_code == 0)
        .unwrap_or(false);
    NativeWakePayload {
        dialog_id: plan.dialog_id,
        local_suppressed: suppressed,
        ack_played: false,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;

    fn marker(dialog_id: &str) -> FileMonitorEvent {
        FileMonitorEvent::NewLine(format!(
            r#"{{"header":{{"dialog_id":"{dialog_id}","id":"id-1","name":"RecognizeResult","namespace":"SpeechRecognizer"}},"payload":{{"is_final":false,"is_vad_begin":false,"results":[{{"confidence":0.0,"text":""}}]}}}}"#
        ))
    }

    #[test]
    fn accepts_exact_native_wake_marker() {
        let mut deduper = DialogDeduper::new();
        let plan = native_wake_plan(&marker("dialog-1"), true, &mut deduper).unwrap();

        assert_eq!(plan.dialog_id, "dialog-1");
        assert!(!plan.acknowledge);
    }

    #[test]
    fn rejects_non_markers_and_duplicate_dialogs() {
        let mut deduper = DialogDeduper::new();
        assert!(native_wake_plan(&FileMonitorEvent::NewFile, true, &mut deduper).is_none());
        assert!(native_wake_plan(
            &FileMonitorEvent::NewLine("not-json".into()),
            true,
            &mut deduper
        )
        .is_none());

        let wrong = FileMonitorEvent::NewLine(
            r#"{"header":{"dialog_id":"d","id":"id","name":"RecognizeResult","namespace":"SpeechRecognizer"},"payload":{"is_final":true,"is_vad_begin":false,"results":[{"text":""}]}}"#.into(),
        );
        assert!(native_wake_plan(&wrong, true, &mut deduper).is_none());

        assert!(native_wake_plan(&marker("dialog-2"), true, &mut deduper).is_some());
        assert!(native_wake_plan(&marker("dialog-2"), true, &mut deduper).is_none());
    }

    #[test]
    fn deduper_is_bounded() {
        let mut deduper = DialogDeduper::new();
        for index in 0..=MAX_DIALOG_IDS {
            assert!(deduper.admit(&format!("dialog-{index}")));
        }
        assert_eq!(deduper.dialog_ids.len(), MAX_DIALOG_IDS);
        assert!(!deduper.admit("dialog-64"));
        assert!(deduper.admit("dialog-0"));
    }

    #[test]
    fn disconnected_marker_is_suppressed_without_acknowledgement() {
        let mut deduper = DialogDeduper::new();
        let plan = native_wake_plan(&marker("dialog-3"), false, &mut deduper).unwrap();
        assert!(!plan.acknowledge);
    }

    #[tokio::test]
    async fn suppresses_native_dialog_without_adding_a_second_acknowledgement() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let calls_for_runner = calls.clone();
        let payload = execute_native_wake(
            NativeWakePlan {
                dialog_id: "dialog-4".into(),
                acknowledge: false,
            },
            move |command| {
                calls_for_runner.lock().unwrap().push(command);
                std::future::ready(Ok::<CommandResult, AppError>(CommandResult {
                    stdout: String::new(),
                    stderr: String::new(),
                    exit_code: 0,
                }))
            },
        )
        .await;

        let calls = calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert!(calls[0].contains("killall"));
        assert_eq!(
            payload,
            NativeWakePayload {
                dialog_id: "dialog-4".into(),
                local_suppressed: true,
                ack_played: false,
            }
        );
    }
}

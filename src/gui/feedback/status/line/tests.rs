use super::{StatusLineEntry, StatusLineEntryParts, StatusLineLog};

#[test]
fn status_line_log_keeps_latest_bounded_message() {
    let mut log = StatusLineLog::new(3);

    log.publish("button", "pressed");
    log.publish("worker", "started");
    log.publish("animation", "stopped");

    assert_eq!(log.len(), 3);
    assert_eq!(log.latest_line(), "animation: stopped");
    assert_eq!(log.latest(), "animation: stopped");
    assert_eq!(
        log.recent_lines(),
        vec![
            "animation: stopped".to_string(),
            "worker: started".to_string(),
            "button: pressed".to_string()
        ]
    );
}

#[test]
fn status_line_log_exposes_borrowed_latest_line() {
    let mut log = StatusLineLog::new(1);

    assert_eq!(log.latest_line(), "system: Ready");

    log.publish("worker", "finished");

    let latest = log.latest_line();
    assert_eq!(latest, "worker: finished");
    assert_eq!(
        log.entries().next_back().map(StatusLineEntry::line),
        Some(latest)
    );
}

#[test]
fn status_line_log_exposes_borrowed_entries_in_both_orders() {
    let mut log = StatusLineLog::new(2);

    log.publish("worker", "started");
    log.publish("animation", "stopped");

    assert_eq!(
        log.entries()
            .map(|entry| entry.source())
            .collect::<Vec<_>>(),
        vec!["worker", "animation"]
    );
    assert_eq!(
        log.recent_entries()
            .map(|entry| entry.message())
            .collect::<Vec<_>>(),
        vec!["stopped", "started"]
    );
}

#[test]
fn status_line_log_writes_recent_lines_into_reused_storage() {
    let mut log = StatusLineLog::new(3);
    log.publish("button", "pressed");
    log.publish("worker", "started");

    let mut lines = Vec::with_capacity(4);
    lines.push(String::from("stale"));
    let capacity = lines.capacity();

    log.recent_lines_into(&mut lines);

    assert_eq!(lines.capacity(), capacity);
    assert_eq!(
        lines,
        vec![
            String::from("worker: started"),
            String::from("button: pressed"),
            String::from("system: Ready"),
        ]
    );
}

#[test]
fn status_line_entry_exposes_source_message_and_line() {
    let entry = StatusLineEntry::new("worker", "finished");

    assert_eq!(entry.source(), "worker");
    assert_eq!(entry.message(), "finished");
    assert_eq!(entry.line(), "worker: finished");
}

#[test]
fn status_line_entry_supports_named_parts_construction() {
    let entry = StatusLineEntry::from_parts(StatusLineEntryParts {
        source: " worker\npool ".to_owned(),
        message: "\rstarted\njob ".to_owned(),
    });

    assert_eq!(entry.source(), "worker pool");
    assert_eq!(entry.message(), "started job");
    assert_eq!(entry.line(), "worker pool: started job");
}

#[test]
fn status_line_entry_normalizes_multiline_text_at_boundary() {
    let entry = StatusLineEntry::new(" worker\npool ", "\rstarted\njob ");

    assert_eq!(entry.source(), "worker pool");
    assert_eq!(entry.message(), "started job");
    assert_eq!(entry.line(), "worker pool: started job");
}

use std::fs::{self, OpenOptions};
use std::io::{self, Read, Write};
use std::path::Path;
use tracevault_core::hooks::{parse_hook_event, HookResponse};

pub fn handle_hook_from_stdin(project_root: &Path) -> Result<(), io::Error> {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;
    handle_hook_event(&input, project_root)?;

    // Output JSON response to stdout
    let response = HookResponse::allow();
    println!("{}", serde_json::to_string(&response).unwrap());
    Ok(())
}

pub fn handle_hook_event(json_input: &str, project_root: &Path) -> Result<(), io::Error> {
    let event =
        parse_hook_event(json_input).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    // Create session directory
    let session_dir = project_root
        .join(".tracevault")
        .join("sessions")
        .join(&event.session_id);
    fs::create_dir_all(&session_dir)?;

    // Append event to events.jsonl
    let events_path = session_dir.join("events.jsonl");
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&events_path)?;

    let event_json =
        serde_json::to_string(&event).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    writeln!(file, "{event_json}")?;

    // Write session metadata if it doesn't exist
    let meta_path = session_dir.join("metadata.json");
    if !meta_path.exists() {
        let metadata = serde_json::json!({
            "session_id": event.session_id,
            "transcript_path": event.transcript_path,
            "cwd": event.cwd,
            "started_at": chrono::Utc::now().to_rfc3339(),
        });
        fs::write(&meta_path, serde_json::to_string_pretty(&metadata).unwrap())?;
    }

    Ok(())
}

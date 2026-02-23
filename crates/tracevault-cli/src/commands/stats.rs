use std::fs;
use std::path::Path;

pub fn show_stats(project_root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let sessions_dir = project_root.join(".tracevault").join("sessions");
    if !sessions_dir.exists() {
        println!("No sessions found. Run `tracevault init` first.");
        return Ok(());
    }

    let mut total_sessions = 0;
    let mut total_events = 0;

    for entry in fs::read_dir(&sessions_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            total_sessions += 1;
            let events_path = entry.path().join("events.jsonl");
            if events_path.exists() {
                let content = fs::read_to_string(&events_path)?;
                total_events += content.lines().count();
            }
        }
    }

    println!("TraceVault Stats");
    println!("================");
    println!("Sessions:     {total_sessions}");
    println!("Total events: {total_events}");

    Ok(())
}

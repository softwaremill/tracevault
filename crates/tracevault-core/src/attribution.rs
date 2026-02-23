use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attribution {
    pub files: Vec<FileAttribution>,
    pub summary: AttributionSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAttribution {
    pub path: String,
    pub lines_added: u32,
    pub lines_deleted: u32,
    pub ai_lines: Vec<LineRange>,
    pub human_lines: Vec<LineRange>,
    pub mixed_lines: Vec<LineRange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineRange {
    pub start: u32,
    pub end: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributionSummary {
    pub total_lines_added: u32,
    pub total_lines_deleted: u32,
    pub ai_percentage: f32,
    pub human_percentage: f32,
}

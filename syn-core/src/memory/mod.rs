// MODULE_CONTRACT
// MODULE_ID: M-MEMORY
// PURPOSE: Durable project memory — persists reusable lessons learned from bounded agent work
// SCOPE: Lesson record schema, project-local JSON persistence, deterministic list/load/save helpers
// DEPENDS: N/A
// LINKS:
//   → UC-002 (implements) - preserve lessons from bounded autonomous work
//   → NFR-003 (traces_to) - durable memory reduces repeated context reconstruction
//   ← V-M-MEMORY (verified_by) - memory persistence verification

// START_MODULE_MAP
// LessonRecord — Persisted reusable project lesson
// LessonsStore — Project-local lesson persistence manager
// slug — Build filesystem-safe lesson identifiers
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 — Added durable project lessons store]
// END_CHANGE_SUMMARY

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// START_public_api

// START_LessonRecord
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LessonRecord {
    pub lesson_id: String,
    pub title: String,
    pub summary: String,
    pub source: String,
    pub module_id: Option<String>,
    pub tags: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub created_at: String,
}
// END_LessonRecord

// START_LessonsStore
#[derive(Debug, Clone)]
pub struct LessonsStore {
    root: PathBuf,
}
// END_LessonsStore

impl LessonRecord {
    // START_CONTRACT_LessonRecord::new
    // PURPOSE: Create a reusable lesson record with a stable slug-based id
    // INPUTS: { title: String }, { summary: String }, { source: String }
    // OUTPUTS: { LessonRecord }
    // LINKS:
    //   → UC-002 (implements) - lessons capture reusable outcomes from bounded agent work
    //   → NFR-003 (traces_to) - stable lesson identifiers support durable context recovery
    // START_lesson_record_new
    pub fn new(title: String, summary: String, source: String) -> Self {
        let created_at = chrono::Utc::now().to_rfc3339();
        Self {
            lesson_id: format!("lesson-{}-{}", slug(&title), uuid::Uuid::new_v4()),
            title,
            summary,
            source,
            module_id: None,
            tags: Vec::new(),
            evidence_refs: Vec::new(),
            created_at,
        }
    }
    // END_lesson_record_new
}

impl LessonsStore {
    // START_CONTRACT_LessonsStore::new
    // PURPOSE: Create a lessons store rooted at one project directory
    // INPUTS: { root: impl AsRef<Path> }
    // OUTPUTS: { LessonsStore }
    // LINKS:
    //   → NFR-003 (traces_to) - project-local memory avoids global context drift
    // START_lessons_store_new
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
        }
    }
    // END_lessons_store_new

    // START_CONTRACT_LessonsStore::lessons_dir
    // PURPOSE: Return the directory used for project lesson records
    // OUTPUTS: { PathBuf }
    // LINKS:
    //   → NFR-003 (traces_to) - deterministic storage path supports resume and replay
    // START_lessons_store_lessons_dir
    pub fn lessons_dir(&self) -> PathBuf {
        self.root.join("docs/memory/lessons")
    }
    // END_lessons_store_lessons_dir

    // START_CONTRACT_LessonsStore::ensure_lessons_dir
    // PURPOSE: Ensure the lesson directory exists before reads or writes
    // OUTPUTS: { anyhow::Result<PathBuf> }
    // SIDE_EFFECTS: creates docs/memory/lessons when absent
    // LINKS:
    //   → NFR-002 (traces_to) - persistence setup must report filesystem errors
    // START_lessons_store_ensure_lessons_dir
    pub fn ensure_lessons_dir(&self) -> anyhow::Result<PathBuf> {
        let dir = self.lessons_dir();
        std::fs::create_dir_all(&dir)?;
        Ok(dir)
    }
    // END_lessons_store_ensure_lessons_dir

    // START_CONTRACT_LessonsStore::lesson_path
    // PURPOSE: Return the JSON path for one lesson id
    // INPUTS: { lesson_id: &str }
    // OUTPUTS: { PathBuf }
    // LINKS:
    //   → NFR-003 (traces_to) - deterministic lesson paths keep memory replayable
    // START_lessons_store_lesson_path
    pub fn lesson_path(&self, lesson_id: &str) -> PathBuf {
        self.lessons_dir().join(format!("{}.json", slug(lesson_id)))
    }
    // END_lessons_store_lesson_path

    // START_CONTRACT_LessonsStore::save
    // PURPOSE: Persist one lesson record as pretty JSON
    // INPUTS: { lesson: &LessonRecord }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: writes docs/memory/lessons/<lesson_id>.json
    // LINKS:
    //   → UC-002 (implements) - bounded work emits reusable memory artifacts
    //   → NFR-002 (traces_to) - write failures return explicit errors
    // START_lessons_store_save
    pub fn save(&self, lesson: &LessonRecord) -> anyhow::Result<()> {
        self.ensure_lessons_dir()?;
        let content = serde_json::to_vec_pretty(lesson)?;
        std::fs::write(self.lesson_path(&lesson.lesson_id), content)?;
        Ok(())
    }
    // END_lessons_store_save

    // START_CONTRACT_LessonsStore::load
    // PURPOSE: Load one lesson record by id
    // INPUTS: { lesson_id: &str }
    // OUTPUTS: { anyhow::Result<LessonRecord> }
    // LINKS:
    //   → NFR-003 (traces_to) - exact lesson recovery supports long-running agent context
    // START_lessons_store_load
    pub fn load(&self, lesson_id: &str) -> anyhow::Result<LessonRecord> {
        let content = std::fs::read(self.lesson_path(lesson_id))?;
        Ok(serde_json::from_slice(&content)?)
    }
    // END_lessons_store_load

    // START_CONTRACT_LessonsStore::list
    // PURPOSE: List persisted lessons in deterministic lesson_id order
    // OUTPUTS: { anyhow::Result<Vec<LessonRecord>> }
    // LINKS:
    //   → UC-002 (implements) - future agents can retrieve reusable project lessons
    //   → NFR-003 (traces_to) - deterministic listing keeps resume context stable
    // START_lessons_store_list
    pub fn list(&self) -> anyhow::Result<Vec<LessonRecord>> {
        let dir = self.ensure_lessons_dir()?;
        let mut lessons = Vec::new();
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            if entry.path().extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }
            let content = std::fs::read(entry.path())?;
            lessons.push(serde_json::from_slice::<LessonRecord>(&content)?);
        }
        lessons.sort_by(|a, b| a.lesson_id.cmp(&b.lesson_id));
        Ok(lessons)
    }
    // END_lessons_store_list
}

// END_public_api

// START_CONTRACT_slug
// PURPOSE: Convert arbitrary text into a filesystem-safe lowercase slug
// INPUTS: { value: &str }
// OUTPUTS: { String }
// LINKS:
//   → NFR-002 (traces_to) - safe filenames avoid path traversal and invalid names
// START_slug
fn slug(value: &str) -> String {
    let slug = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if slug.is_empty() {
        "lesson".into()
    } else {
        slug
    }
}
// END_slug

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lessons_store_persists_and_lists_lessons() {
        let root = tempfile::tempdir().unwrap();
        let store = LessonsStore::new(root.path());
        let mut lesson = LessonRecord::new(
            "Blocked run approval".into(),
            "Require review before resuming blocked runs".into(),
            "Phase-19".into(),
        );
        lesson.module_id = Some("M-RUNNER".into());
        lesson.tags.push("autonomy".into());
        lesson.evidence_refs.push("docs/phases/Phase-19.xml".into());
        store.save(&lesson).unwrap();

        let loaded = store.load(&lesson.lesson_id).unwrap();
        assert_eq!(loaded.title, "Blocked run approval");
        assert_eq!(loaded.module_id.as_deref(), Some("M-RUNNER"));
        let listed = store.list().unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].lesson_id, lesson.lesson_id);
    }
}

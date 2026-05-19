// MODULE_CONTRACT
// MODULE_ID: M-INDEXER-STORAGE-TYPES
// PURPOSE: Shared storage data types for indexed code blocks
// SCOPE: StoredBlock serialization shape
// DEPENDS: N/A
// LINKS: docs/modules/M-INDEXER-STORAGE.xml

// START_MODULE_MAP
// StoredBlock — Serializable code block for JSON storage
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.1.1 — Extracted StoredBlock from M-INDEXER-STORAGE]
// END_CHANGE_SUMMARY

// START_public_api

// START_StoredBlock
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct StoredBlock {
    pub id: String,
    pub path: String,
    pub language: String,
    pub name: String,
    pub kind: String,
    pub content: String,
    pub start_line: usize,
    pub end_line: usize,
}
// END_StoredBlock

// START_CONTRACT_public_api
// PURPOSE: Export the persisted code block data structure
// OUTPUTS: { StoredBlock }
// END_public_api

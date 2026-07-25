use crate::{
    models::{
        AiJob, AnalysisFrame, AssistantConversation, AssistantMessage, EditPlan,
        RemoteRequestPreview, SemanticSegment,
    },
    storage::{self, NativeResult},
};
use rusqlite::{params, Connection};
use std::path::Path;

pub fn ensure_schema(connection: &Connection) -> NativeResult<()> {
    connection.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS analysis_frames (
          id TEXT PRIMARY KEY,
          asset_id TEXT NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
          timestamp REAL NOT NULL,path TEXT NOT NULL,reason TEXT NOT NULL,perceptual_hash TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS semantic_segments (
          id TEXT PRIMARY KEY,
          asset_id TEXT NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
          start REAL NOT NULL,end REAL NOT NULL,title TEXT NOT NULL,description TEXT NOT NULL,
          transcript_excerpt TEXT NOT NULL,visual_observations_json TEXT NOT NULL,
          importance TEXT NOT NULL,suggested_decision TEXT NOT NULL,confidence REAL NOT NULL,
          provenance TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS remote_request_previews (
          id TEXT PRIMARY KEY,kind TEXT NOT NULL,payload_json TEXT NOT NULL,status TEXT NOT NULL,
          created_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS remote_approvals (
          scope TEXT PRIMARY KEY,approved_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS assistant_conversations (
          id TEXT PRIMARY KEY,title TEXT NOT NULL,created_at TEXT NOT NULL,updated_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS assistant_messages (
          id TEXT PRIMARY KEY,
          conversation_id TEXT NOT NULL REFERENCES assistant_conversations(id) ON DELETE CASCADE,
          role TEXT NOT NULL,content TEXT NOT NULL,created_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS edit_plans (
          id TEXT PRIMARY KEY,conversation_id TEXT NOT NULL,status TEXT NOT NULL,
          plan_json TEXT NOT NULL,created_at TEXT NOT NULL,updated_at TEXT NOT NULL
        );",
    ).map_err(|error| error.to_string())
}

fn insert_segments(connection: &Connection, segments: &[SemanticSegment]) -> NativeResult<()> {
    for segment in segments {
        let observations = serde_json::to_string(&segment.visual_observations)
            .map_err(|error| error.to_string())?;
        connection
            .execute(
                "INSERT INTO semantic_segments
             (id,asset_id,start,end,title,description,transcript_excerpt,visual_observations_json,
              importance,suggested_decision,confidence,provenance)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
                params![
                    segment.id,
                    segment.asset_id,
                    segment.start,
                    segment.end,
                    segment.title,
                    segment.description,
                    segment.transcript_excerpt,
                    observations,
                    segment.importance,
                    segment.suggested_decision,
                    segment.confidence,
                    segment.provenance
                ],
            )
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

pub fn replace_local_analysis(
    project_path: &Path,
    asset_id: &str,
    frames: &[AnalysisFrame],
    segments: &[SemanticSegment],
) -> NativeResult<()> {
    let mut connection = storage::open_database(project_path)?;
    let transaction = connection
        .transaction()
        .map_err(|error| error.to_string())?;
    transaction
        .execute("DELETE FROM analysis_frames WHERE asset_id=?1", [asset_id])
        .map_err(|error| error.to_string())?;
    transaction
        .execute(
            "DELETE FROM semantic_segments WHERE asset_id=?1 AND provenance='local'",
            [asset_id],
        )
        .map_err(|error| error.to_string())?;
    for frame in frames {
        transaction
            .execute(
                "INSERT INTO analysis_frames (id,asset_id,timestamp,path,reason,perceptual_hash)
             VALUES (?1,?2,?3,?4,?5,?6)",
                params![
                    frame.id,
                    frame.asset_id,
                    frame.timestamp,
                    frame.path,
                    frame.reason,
                    frame.perceptual_hash
                ],
            )
            .map_err(|error| error.to_string())?;
    }
    insert_segments(&transaction, segments)?;
    transaction.commit().map_err(|error| error.to_string())
}

pub fn replace_remote_segments(
    project_path: &Path,
    asset_ids: &[String],
    segments: &[SemanticSegment],
) -> NativeResult<()> {
    let mut connection = storage::open_database(project_path)?;
    let transaction = connection
        .transaction()
        .map_err(|error| error.to_string())?;
    for asset_id in asset_ids {
        transaction
            .execute(
                "DELETE FROM semantic_segments WHERE asset_id=?1 AND provenance!='local'",
                [asset_id],
            )
            .map_err(|error| error.to_string())?;
    }
    insert_segments(&transaction, segments)?;
    transaction.commit().map_err(|error| error.to_string())
}

pub fn list_analysis_frames(project_path: &Path) -> NativeResult<Vec<AnalysisFrame>> {
    let connection = storage::open_database(project_path)?;
    let mut statement = connection
        .prepare(
            "SELECT id,asset_id,timestamp,path,reason,perceptual_hash
         FROM analysis_frames ORDER BY asset_id,timestamp",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok(AnalysisFrame {
                id: row.get(0)?,
                asset_id: row.get(1)?,
                timestamp: row.get(2)?,
                path: row.get(3)?,
                reason: row.get(4)?,
                perceptual_hash: row.get(5)?,
            })
        })
        .map_err(|error| error.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

pub fn list_semantic_segments(project_path: &Path) -> NativeResult<Vec<SemanticSegment>> {
    let connection = storage::open_database(project_path)?;
    let mut statement = connection.prepare(
        "SELECT id,asset_id,start,end,title,description,transcript_excerpt,visual_observations_json,
         importance,suggested_decision,confidence,provenance
         FROM semantic_segments ORDER BY asset_id,start",
    ).map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| {
            let observations: String = row.get(7)?;
            Ok(SemanticSegment {
                id: row.get(0)?,
                asset_id: row.get(1)?,
                start: row.get(2)?,
                end: row.get(3)?,
                title: row.get(4)?,
                description: row.get(5)?,
                transcript_excerpt: row.get(6)?,
                visual_observations: serde_json::from_str(&observations).unwrap_or_default(),
                importance: row.get(8)?,
                suggested_decision: row.get(9)?,
                confidence: row.get(10)?,
                provenance: row.get(11)?,
            })
        })
        .map_err(|error| error.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

pub fn save_preview(project_path: &Path, preview: &RemoteRequestPreview) -> NativeResult<()> {
    let connection = storage::open_database(project_path)?;
    let payload = serde_json::to_string(preview).map_err(|error| error.to_string())?;
    connection
        .execute(
            "INSERT OR REPLACE INTO remote_request_previews
         (id,kind,payload_json,status,created_at) VALUES (?1,?2,?3,'prepared',?4)",
            params![preview.id, preview.kind, payload, preview.created_at],
        )
        .map(|_| ())
        .map_err(|error| error.to_string())
}

pub fn load_preview(project_path: &Path, preview_id: &str) -> NativeResult<RemoteRequestPreview> {
    let connection = storage::open_database(project_path)?;
    let payload: String = connection
        .query_row(
            "SELECT payload_json FROM remote_request_previews WHERE id=?1",
            [preview_id],
            |row| row.get(0),
        )
        .map_err(|_| "The prepared remote request no longer exists".to_string())?;
    serde_json::from_str(&payload).map_err(|error| format!("Invalid prepared request: {error}"))
}

pub fn mark_preview_status(
    project_path: &Path,
    preview_id: &str,
    status: &str,
) -> NativeResult<()> {
    storage::open_database(project_path)?
        .execute(
            "UPDATE remote_request_previews SET status=?1 WHERE id=?2",
            params![status, preview_id],
        )
        .map(|_| ())
        .map_err(|error| error.to_string())
}

pub fn remote_analysis_approved(project_path: &Path) -> NativeResult<bool> {
    let count: i64 = storage::open_database(project_path)?
        .query_row(
            "SELECT COUNT(*) FROM remote_approvals WHERE scope='semantic-analysis'",
            [],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    Ok(count > 0)
}

pub fn approve_remote_analysis(project_path: &Path, approved_at: &str) -> NativeResult<()> {
    storage::open_database(project_path)?
        .execute(
            "INSERT OR REPLACE INTO remote_approvals (scope,approved_at)
         VALUES ('semantic-analysis',?1)",
            [approved_at],
        )
        .map(|_| ())
        .map_err(|error| error.to_string())
}

pub fn upsert_conversation(project_path: &Path, value: &AssistantConversation) -> NativeResult<()> {
    storage::open_database(project_path)?.execute(
        "INSERT INTO assistant_conversations (id,title,created_at,updated_at) VALUES (?1,?2,?3,?4)
         ON CONFLICT(id) DO UPDATE SET title=excluded.title,updated_at=excluded.updated_at",
        params![value.id,value.title,value.created_at,value.updated_at],
    ).map(|_| ()).map_err(|error| error.to_string())
}

pub fn insert_message(project_path: &Path, value: &AssistantMessage) -> NativeResult<()> {
    storage::open_database(project_path)?
        .execute(
            "INSERT OR REPLACE INTO assistant_messages (id,conversation_id,role,content,created_at)
         VALUES (?1,?2,?3,?4,?5)",
            params![
                value.id,
                value.conversation_id,
                value.role,
                value.content,
                value.created_at
            ],
        )
        .map(|_| ())
        .map_err(|error| error.to_string())
}

pub fn list_conversations(project_path: &Path) -> NativeResult<Vec<AssistantConversation>> {
    let connection = storage::open_database(project_path)?;
    let mut statement = connection.prepare(
        "SELECT id,title,created_at,updated_at FROM assistant_conversations ORDER BY updated_at",
    ).map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok(AssistantConversation {
                id: row.get(0)?,
                title: row.get(1)?,
                created_at: row.get(2)?,
                updated_at: row.get(3)?,
            })
        })
        .map_err(|error| error.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

pub fn list_messages(project_path: &Path) -> NativeResult<Vec<AssistantMessage>> {
    let connection = storage::open_database(project_path)?;
    let mut statement = connection
        .prepare(
            "SELECT id,conversation_id,role,content,created_at
         FROM assistant_messages ORDER BY created_at",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok(AssistantMessage {
                id: row.get(0)?,
                conversation_id: row.get(1)?,
                role: row.get(2)?,
                content: row.get(3)?,
                created_at: row.get(4)?,
            })
        })
        .map_err(|error| error.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

pub fn save_plan(project_path: &Path, value: &EditPlan) -> NativeResult<()> {
    let payload = serde_json::to_string(value).map_err(|error| error.to_string())?;
    storage::open_database(project_path)?
        .execute(
            "INSERT INTO edit_plans (id,conversation_id,status,plan_json,created_at,updated_at)
         VALUES (?1,?2,?3,?4,?5,?6)
         ON CONFLICT(id) DO UPDATE SET status=excluded.status,plan_json=excluded.plan_json,
         updated_at=excluded.updated_at",
            params![
                value.id,
                value.conversation_id,
                value.status,
                payload,
                value.created_at,
                value.updated_at
            ],
        )
        .map(|_| ())
        .map_err(|error| error.to_string())
}

pub fn load_plan(project_path: &Path, plan_id: &str) -> NativeResult<EditPlan> {
    let payload: String = storage::open_database(project_path)?
        .query_row(
            "SELECT plan_json FROM edit_plans WHERE id=?1",
            [plan_id],
            |row| row.get(0),
        )
        .map_err(|_| "Edit plan not found".to_string())?;
    serde_json::from_str(&payload).map_err(|error| format!("Invalid saved edit plan: {error}"))
}

pub fn list_plans(project_path: &Path) -> NativeResult<Vec<EditPlan>> {
    let connection = storage::open_database(project_path)?;
    let mut statement = connection
        .prepare("SELECT plan_json FROM edit_plans ORDER BY updated_at")
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|error| error.to_string())?;
    rows.map(|row| {
        row.map_err(|error| error.to_string()).and_then(|payload| {
            serde_json::from_str(&payload)
                .map_err(|error| format!("Invalid saved edit plan: {error}"))
        })
    })
    .collect()
}

pub fn empty_ai_job(project_path: &str, preview_id: &str, kind: &str) -> AiJob {
    AiJob {
        id: uuid::Uuid::new_v4().to_string(),
        project_path: project_path.into(),
        preview_id: preview_id.into(),
        kind: kind.into(),
        status: "queued".into(),
        stage: "Waiting to contact OpenRouter".into(),
        progress: 0.0,
        received_chars: 0,
        error: String::new(),
        result_plan_id: String::new(),
        started_at: chrono::Utc::now().to_rfc3339(),
        finished_at: None,
    }
}

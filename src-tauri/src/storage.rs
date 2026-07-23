use crate::models::{
    GlobalData, MediaAsset, ProjectContext, ProjectManifest, ProjectSummary, Scene,
    TranscriptSegment,
};
use chrono::Utc;
use rusqlite::{params, Connection};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub type NativeResult<T> = Result<T, String>;

pub fn app_config_path() -> NativeResult<PathBuf> {
    let directory = dirs::config_dir()
        .ok_or_else(|| "Windows did not provide a configuration directory".to_string())?
        .join("Edentic");
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    Ok(directory.join("settings.json"))
}

pub fn load_global_data() -> GlobalData {
    let Ok(path) = app_config_path() else {
        return GlobalData::default();
    };
    let Ok(content) = fs::read_to_string(path) else {
        return GlobalData::default();
    };
    serde_json::from_str(&content).unwrap_or_default()
}

pub fn save_global_data(data: &GlobalData) -> NativeResult<()> {
    let path = app_config_path()?;
    let temporary = path.with_extension("json.tmp");
    let content = serde_json::to_string_pretty(data).map_err(|error| error.to_string())?;
    fs::write(&temporary, content).map_err(|error| error.to_string())?;
    fs::rename(temporary, path).map_err(|error| error.to_string())
}

pub fn sanitize_project_name(name: &str) -> String {
    let invalid = ['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
    name.trim()
        .chars()
        .filter(|character| !invalid.contains(character) && !character.is_control())
        .collect::<String>()
        .trim_end_matches(['.', ' '])
        .to_string()
}

pub fn initialize_project_folders(project_path: &Path) -> NativeResult<()> {
    let directories = [
        "Media/Originals",
        "Media/Audio",
        "Media/Images",
        "Media/Generated",
        "Edit/index-data",
        "Autosaves",
        "Proxies",
        "Cache/posters",
        "Cache/waveforms",
        "Cache/scenes",
        "Backups",
    ];
    for directory in directories {
        fs::create_dir_all(project_path.join(directory)).map_err(|error| error.to_string())?;
    }
    Ok(())
}

pub fn manifest_path(project_path: &Path) -> PathBuf {
    project_path.join("Edit").join("project.edentic")
}

pub fn database_path(project_path: &Path) -> PathBuf {
    project_path.join("Edit").join("project.sqlite")
}

pub fn save_manifest(project_path: &Path, manifest: &ProjectManifest) -> NativeResult<()> {
    let content = serde_json::to_string_pretty(manifest).map_err(|error| error.to_string())?;
    fs::write(manifest_path(project_path), content).map_err(|error| error.to_string())
}

pub fn load_manifest(project_path: &Path) -> NativeResult<ProjectManifest> {
    let content = fs::read_to_string(manifest_path(project_path))
        .map_err(|_| "This folder is not an Edentic project".to_string())?;
    serde_json::from_str(&content).map_err(|error| format!("Invalid Edentic project: {error}"))
}

pub fn open_database(project_path: &Path) -> NativeResult<Connection> {
    let connection = Connection::open(database_path(project_path)).map_err(|error| error.to_string())?;
    connection
        .execute_batch(
            "
            PRAGMA journal_mode=WAL;
            PRAGMA foreign_keys=ON;
            CREATE TABLE IF NOT EXISTS assets (
              id TEXT PRIMARY KEY,
              project_id TEXT NOT NULL,
              name TEXT NOT NULL,
              original_path TEXT NOT NULL,
              managed_path TEXT NOT NULL,
              duration REAL NOT NULL,
              width INTEGER NOT NULL,
              height INTEGER NOT NULL,
              frame_rate REAL NOT NULL,
              size_bytes INTEGER NOT NULL,
              video_codec TEXT NOT NULL,
              audio_codec TEXT NOT NULL,
              proxy_path TEXT NOT NULL DEFAULT '',
              poster_path TEXT NOT NULL DEFAULT '',
              waveform_path TEXT NOT NULL DEFAULT '',
              index_status TEXT NOT NULL DEFAULT 'waiting',
              imported_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS scenes (
              id TEXT PRIMARY KEY,
              asset_id TEXT NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
              start REAL NOT NULL,
              end REAL NOT NULL,
              label TEXT NOT NULL,
              thumbnail_path TEXT NOT NULL DEFAULT ''
            );
            CREATE TABLE IF NOT EXISTS transcript (
              id TEXT PRIMARY KEY,
              asset_id TEXT NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
              start REAL NOT NULL,
              end REAL NOT NULL,
              text TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS contexts (
              id TEXT PRIMARY KEY,
              name TEXT NOT NULL,
              source TEXT NOT NULL,
              content TEXT NOT NULL,
              created_at TEXT NOT NULL
            );
            ",
        )
        .map_err(|error| error.to_string())?;
    Ok(connection)
}

pub fn insert_asset(project_path: &Path, asset: &MediaAsset) -> NativeResult<()> {
    let connection = open_database(project_path)?;
    connection
        .execute(
            "INSERT INTO assets (
              id, project_id, name, original_path, managed_path, duration, width, height,
              frame_rate, size_bytes, video_codec, audio_codec, proxy_path, poster_path,
              waveform_path, index_status, imported_at
            ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17)",
            params![
                asset.id,
                asset.project_id,
                asset.name,
                asset.original_path,
                asset.managed_path,
                asset.duration,
                asset.width,
                asset.height,
                asset.frame_rate,
                asset.size_bytes,
                asset.video_codec,
                asset.audio_codec,
                asset.proxy_path,
                asset.poster_path,
                asset.waveform_path,
                asset.index_status,
                Utc::now().to_rfc3339()
            ],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

pub fn list_assets(project_path: &Path) -> NativeResult<Vec<MediaAsset>> {
    let connection = open_database(project_path)?;
    let mut statement = connection
        .prepare(
            "SELECT id, project_id, name, original_path, managed_path, duration, width, height,
             frame_rate, size_bytes, video_codec, audio_codec, proxy_path, poster_path, waveform_path, index_status
             FROM assets ORDER BY imported_at ASC",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok(MediaAsset {
                id: row.get(0)?,
                project_id: row.get(1)?,
                name: row.get(2)?,
                original_path: row.get(3)?,
                managed_path: row.get(4)?,
                duration: row.get(5)?,
                width: row.get(6)?,
                height: row.get(7)?,
                frame_rate: row.get(8)?,
                size_bytes: row.get(9)?,
                video_codec: row.get(10)?,
                audio_codec: row.get(11)?,
                proxy_path: row.get(12)?,
                poster_path: row.get(13)?,
                waveform_path: row.get(14)?,
                index_status: row.get(15)?,
            })
        })
        .map_err(|error| error.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|error| error.to_string())
}

pub fn find_asset(project_path: &Path, asset_id: &str) -> NativeResult<MediaAsset> {
    list_assets(project_path)?
        .into_iter()
        .find(|asset| asset.id == asset_id)
        .ok_or_else(|| "The selected source no longer exists".to_string())
}

pub fn replace_index(
    project_path: &Path,
    asset: &MediaAsset,
    scenes: &[Scene],
    transcript: &[TranscriptSegment],
    status: &str,
) -> NativeResult<()> {
    let mut connection = open_database(project_path)?;
    let transaction = connection.transaction().map_err(|error| error.to_string())?;
    transaction
        .execute("DELETE FROM scenes WHERE asset_id = ?1", [asset.id.as_str()])
        .map_err(|error| error.to_string())?;
    transaction
        .execute("DELETE FROM transcript WHERE asset_id = ?1", [asset.id.as_str()])
        .map_err(|error| error.to_string())?;
    for scene in scenes {
        transaction
            .execute(
                "INSERT INTO scenes (id, asset_id, start, end, label, thumbnail_path) VALUES (?1,?2,?3,?4,?5,?6)",
                params![scene.id, scene.asset_id, scene.start, scene.end, scene.label, scene.thumbnail_path],
            )
            .map_err(|error| error.to_string())?;
    }
    for segment in transcript {
        transaction
            .execute(
                "INSERT INTO transcript (id, asset_id, start, end, text) VALUES (?1,?2,?3,?4,?5)",
                params![segment.id, segment.asset_id, segment.start, segment.end, segment.text],
            )
            .map_err(|error| error.to_string())?;
    }
    transaction
        .execute(
            "UPDATE assets SET proxy_path=?1, poster_path=?2, waveform_path=?3, index_status=?4 WHERE id=?5",
            params![asset.proxy_path, asset.poster_path, asset.waveform_path, status, asset.id],
        )
        .map_err(|error| error.to_string())?;
    transaction.commit().map_err(|error| error.to_string())
}

pub fn list_scenes(project_path: &Path) -> NativeResult<Vec<Scene>> {
    let connection = open_database(project_path)?;
    let mut statement = connection
        .prepare("SELECT id, asset_id, start, end, label, thumbnail_path FROM scenes ORDER BY asset_id, start")
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok(Scene {
                id: row.get(0)?,
                asset_id: row.get(1)?,
                start: row.get(2)?,
                end: row.get(3)?,
                label: row.get(4)?,
                thumbnail_path: row.get(5)?,
            })
        })
        .map_err(|error| error.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|error| error.to_string())
}

pub fn list_transcript(project_path: &Path) -> NativeResult<Vec<TranscriptSegment>> {
    let connection = open_database(project_path)?;
    let mut statement = connection
        .prepare("SELECT id, asset_id, start, end, text FROM transcript ORDER BY asset_id, start")
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok(TranscriptSegment {
                id: row.get(0)?,
                asset_id: row.get(1)?,
                start: row.get(2)?,
                end: row.get(3)?,
                text: row.get(4)?,
            })
        })
        .map_err(|error| error.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|error| error.to_string())
}

pub fn insert_context(project_path: &Path, context: &ProjectContext) -> NativeResult<()> {
    let connection = open_database(project_path)?;
    connection
        .execute(
            "INSERT INTO contexts (id, name, source, content, created_at) VALUES (?1,?2,?3,?4,?5)",
            params![context.id, context.name, context.source, context.content, context.created_at],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

pub fn list_contexts(project_path: &Path) -> NativeResult<Vec<ProjectContext>> {
    let connection = open_database(project_path)?;
    let mut statement = connection
        .prepare("SELECT id, name, source, content, created_at FROM contexts ORDER BY created_at")
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok(ProjectContext {
                id: row.get(0)?,
                name: row.get(1)?,
                source: row.get(2)?,
                content: row.get(3)?,
                created_at: row.get(4)?,
            })
        })
        .map_err(|error| error.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|error| error.to_string())
}

pub fn project_summary(project_path: &Path) -> NativeResult<ProjectSummary> {
    let manifest = load_manifest(project_path)?;
    let assets = list_assets(project_path)?;
    let thumbnail = assets
        .iter()
        .find(|asset| !asset.poster_path.is_empty())
        .map(|asset| asset.poster_path.clone())
        .unwrap_or_default();
    Ok(manifest.summary(project_path, assets.len(), thumbnail))
}

pub fn touch_project(project_path: &Path) -> NativeResult<ProjectManifest> {
    let mut manifest = load_manifest(project_path)?;
    manifest.updated_at = Utc::now().to_rfc3339();
    save_manifest(project_path, &manifest)?;
    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizes_windows_project_names() {
        assert_eq!(sanitize_project_name("  Race: Edit?  "), "Race Edit");
        assert_eq!(sanitize_project_name("Project."), "Project");
    }
}

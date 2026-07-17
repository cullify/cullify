use std::{
    path::{Path, PathBuf},
    sync::Mutex,
};

use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, params, params_from_iter};

use crate::{
    error::{AppError, AppResult},
    models::{NewPhoto, Photo, Project},
};

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn open(path: PathBuf) -> AppResult<Self> {
        let conn = Connection::open(&path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let database = Self {
            conn: Mutex::new(conn),
        };
        database.migrate()?;
        Ok(database)
    }

    fn migrate(&self) -> AppResult<()> {
        let conn = self.lock();
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS projects (
                id            TEXT PRIMARY KEY,
                name          TEXT NOT NULL,
                mode          TEXT NOT NULL DEFAULT 'quick',
                status        TEXT NOT NULL DEFAULT 'ready',
                folder_path   TEXT NOT NULL,
                total_photos  INTEGER NOT NULL DEFAULT 0,
                kept_count    INTEGER NOT NULL DEFAULT 0,
                culled_count  INTEGER NOT NULL DEFAULT 0,
                created_at    TEXT NOT NULL,
                updated_at    TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS photos (
                id               TEXT PRIMARY KEY,
                project_id       TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
                original_name    TEXT NOT NULL,
                file_path        TEXT NOT NULL,
                thumbnail_path   TEXT,
                file_size        INTEGER NOT NULL DEFAULT 0,
                width            INTEGER,
                height           INTEGER,
                mime_type        TEXT NOT NULL,
                analysis_status  TEXT NOT NULL DEFAULT 'done',
                quality_score    REAL,
                is_blurred       INTEGER NOT NULL DEFAULT 0,
                blur_score       REAL,
                is_overexposed   INTEGER NOT NULL DEFAULT 0,
                is_underexposed  INTEGER NOT NULL DEFAULT 0,
                exposure_score   REAL,
                user_decision    TEXT,
                is_auto_culled   INTEGER NOT NULL DEFAULT 0,
                perceptual_hash  TEXT,
                group_label      TEXT NOT NULL DEFAULT '未分组',
                captured_at      TEXT,
                camera           TEXT,
                lens             TEXT,
                focal            TEXT,
                aperture         TEXT,
                shutter          TEXT,
                iso              TEXT,
                created_at       TEXT NOT NULL,
                UNIQUE(project_id, file_path)
            );

            CREATE INDEX IF NOT EXISTS idx_photos_project ON photos(project_id);
            CREATE INDEX IF NOT EXISTS idx_photos_status ON photos(analysis_status);
            CREATE INDEX IF NOT EXISTS idx_photos_decision ON photos(user_decision);
            "#,
        )?;
        ensure_column(
            &conn,
            "photos",
            "thumbnail_path",
            "ALTER TABLE photos ADD COLUMN thumbnail_path TEXT",
        )?;
        ensure_column(
            &conn,
            "photos",
            "perceptual_hash",
            "ALTER TABLE photos ADD COLUMN perceptual_hash TEXT",
        )?;
        ensure_column(
            &conn,
            "photos",
            "group_label",
            "ALTER TABLE photos ADD COLUMN group_label TEXT NOT NULL DEFAULT '未分组'",
        )?;
        ensure_column(
            &conn,
            "photos",
            "captured_at",
            "ALTER TABLE photos ADD COLUMN captured_at TEXT",
        )?;
        ensure_column(
            &conn,
            "photos",
            "camera",
            "ALTER TABLE photos ADD COLUMN camera TEXT",
        )?;
        ensure_column(
            &conn,
            "photos",
            "lens",
            "ALTER TABLE photos ADD COLUMN lens TEXT",
        )?;
        ensure_column(
            &conn,
            "photos",
            "focal",
            "ALTER TABLE photos ADD COLUMN focal TEXT",
        )?;
        ensure_column(
            &conn,
            "photos",
            "aperture",
            "ALTER TABLE photos ADD COLUMN aperture TEXT",
        )?;
        ensure_column(
            &conn,
            "photos",
            "shutter",
            "ALTER TABLE photos ADD COLUMN shutter TEXT",
        )?;
        ensure_column(
            &conn,
            "photos",
            "iso",
            "ALTER TABLE photos ADD COLUMN iso TEXT",
        )?;
        Ok(())
    }

    pub fn create_project(&self, name: &str, folder_path: &Path, mode: &str) -> AppResult<Project> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let folder_path = folder_path.to_string_lossy().to_string();
        let conn = self.lock();
        conn.execute(
            "INSERT INTO projects (id, name, mode, status, folder_path, created_at, updated_at)
             VALUES (?1, ?2, ?3, 'ready', ?4, ?5, ?5)",
            params![id, name, mode, folder_path, now],
        )?;
        drop(conn);
        self.get_project(&id)?
            .ok_or_else(|| AppError::ProjectNotFound(id))
    }

    pub fn list_projects(&self) -> AppResult<Vec<Project>> {
        let conn = self.lock();
        let mut stmt = conn.prepare(
            "SELECT id, name, mode, status, folder_path, total_photos, kept_count, culled_count, created_at, updated_at
             FROM projects
             ORDER BY updated_at DESC",
        )?;
        let rows = stmt.query_map([], map_project)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
    }

    pub fn get_project(&self, id: &str) -> AppResult<Option<Project>> {
        let conn = self.lock();
        conn.query_row(
            "SELECT id, name, mode, status, folder_path, total_photos, kept_count, culled_count, created_at, updated_at
             FROM projects WHERE id = ?1",
            params![id],
            map_project,
        )
        .optional()
        .map_err(AppError::from)
    }

    pub fn rename_project(&self, id: &str, name: &str) -> AppResult<Project> {
        let trimmed_name = name.trim();
        if trimmed_name.is_empty() {
            return Err(AppError::EmptyProjectName);
        }

        let now = Utc::now().to_rfc3339();
        let conn = self.lock();
        let affected = conn.execute(
            "UPDATE projects SET name = ?1, updated_at = ?2 WHERE id = ?3",
            params![trimmed_name, now, id],
        )?;
        drop(conn);

        if affected == 0 {
            return Err(AppError::ProjectNotFound(id.to_string()));
        }

        self.get_project(id)?
            .ok_or_else(|| AppError::ProjectNotFound(id.to_string()))
    }

    pub fn delete_project(&self, id: &str) -> AppResult<()> {
        let conn = self.lock();
        let affected = conn.execute("DELETE FROM projects WHERE id = ?1", params![id])?;
        if affected == 0 {
            return Err(AppError::ProjectNotFound(id.to_string()));
        }
        Ok(())
    }

    pub fn insert_photos(&self, project_id: &str, photos: &[NewPhoto]) -> AppResult<usize> {
        if photos.is_empty() {
            return Ok(0);
        }

        let now = Utc::now().to_rfc3339();
        let mut conn = self.lock();
        let tx = conn.transaction()?;
        let mut inserted = 0usize;

        {
            let mut stmt = tx.prepare(
                "INSERT OR IGNORE INTO photos (
                    id, project_id, original_name, file_path, thumbnail_path, file_size, width, height, mime_type,
                    analysis_status, quality_score, is_blurred, blur_score, is_overexposed,
                    is_underexposed, exposure_score, is_auto_culled, perceptual_hash, group_label,
                    captured_at, camera, lens, focal, aperture, shutter, iso, created_at
                 )
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'done', ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26)",
            )?;

            for photo in photos {
                inserted += stmt.execute(params![
                    photo.id,
                    project_id,
                    photo.original_name,
                    photo.file_path,
                    photo.thumbnail_path,
                    photo.file_size,
                    photo.width,
                    photo.height,
                    photo.mime_type,
                    photo.quality_score,
                    photo.is_blurred as i64,
                    photo.blur_score,
                    photo.is_overexposed as i64,
                    photo.is_underexposed as i64,
                    photo.exposure_score,
                    photo.is_auto_culled as i64,
                    photo.perceptual_hash,
                    photo.group_label,
                    photo.captured_at,
                    photo.camera,
                    photo.lens,
                    photo.focal,
                    photo.aperture,
                    photo.shutter,
                    photo.iso,
                    now,
                ])?;
            }
        }

        tx.commit()?;
        drop(conn);
        self.recount_project(project_id)?;
        Ok(inserted)
    }

    pub fn list_photos(&self, project_id: &str) -> AppResult<Vec<Photo>> {
        let conn = self.lock();
        let mut stmt = conn.prepare(
            "SELECT id, project_id, original_name, file_path, thumbnail_path, file_size, width, height, mime_type,
                    analysis_status, quality_score, is_blurred, blur_score, is_overexposed,
                    is_underexposed, exposure_score, user_decision, is_auto_culled, perceptual_hash, group_label,
                    captured_at, camera, lens, focal, aperture, shutter, iso, created_at
             FROM photos
             WHERE project_id = ?1
             ORDER BY group_label ASC, captured_at ASC, quality_score DESC, original_name ASC",
        )?;
        let rows = stmt.query_map(params![project_id], map_photo)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
    }

    pub fn set_photo_decision(&self, photo_id: &str, decision: Option<&str>) -> AppResult<Photo> {
        if let Some(decision) = decision {
            if !matches!(decision, "keep" | "cull") {
                return Err(AppError::InvalidDecision(decision.to_string()));
            }
        }

        let conn = self.lock();
        let project_id: String = conn.query_row(
            "SELECT project_id FROM photos WHERE id = ?1",
            params![photo_id],
            |row| row.get(0),
        )?;
        conn.execute(
            "UPDATE photos SET user_decision = ?1, is_auto_culled = 0 WHERE id = ?2",
            params![decision, photo_id],
        )?;
        drop(conn);
        self.recount_project(&project_id)?;
        self.get_photo(photo_id)
    }

    pub fn set_photo_decisions(
        &self,
        project_id: &str,
        photo_ids: &[String],
        decision: Option<&str>,
    ) -> AppResult<Vec<Photo>> {
        if let Some(decision) = decision {
            if !matches!(decision, "keep" | "cull") {
                return Err(AppError::InvalidDecision(decision.to_string()));
            }
        }

        if photo_ids.is_empty() {
            return Ok(Vec::new());
        }

        let mut conn = self.lock();
        let tx = conn.transaction()?;
        let placeholders = repeat_placeholders(photo_ids.len());
        let affected = {
            let sql = format!(
                "UPDATE photos
                 SET user_decision = ?, is_auto_culled = 0
                 WHERE project_id = ? AND id IN ({placeholders})"
            );
            let params = std::iter::once(decision.map(str::to_string))
                .chain(std::iter::once(Some(project_id.to_string())))
                .chain(photo_ids.iter().cloned().map(Some));
            tx.execute(&sql, params_from_iter(params))?
        };

        if affected != photo_ids.len() {
            return Err(AppError::PhotoBatchMismatch {
                expected: photo_ids.len(),
                updated: affected,
            });
        }

        tx.commit()?;
        drop(conn);
        self.recount_project(project_id)?;
        self.list_photos_by_ids(project_id, photo_ids)
    }

    pub fn get_photo(&self, photo_id: &str) -> AppResult<Photo> {
        let conn = self.lock();
        conn.query_row(
            "SELECT id, project_id, original_name, file_path, thumbnail_path, file_size, width, height, mime_type,
                    analysis_status, quality_score, is_blurred, blur_score, is_overexposed,
                    is_underexposed, exposure_score, user_decision, is_auto_culled, perceptual_hash, group_label,
                    captured_at, camera, lens, focal, aperture, shutter, iso, created_at
             FROM photos WHERE id = ?1",
            params![photo_id],
            map_photo,
        )
        .map_err(AppError::from)
    }

    fn list_photos_by_ids(&self, project_id: &str, photo_ids: &[String]) -> AppResult<Vec<Photo>> {
        if photo_ids.is_empty() {
            return Ok(Vec::new());
        }

        let conn = self.lock();
        let placeholders = repeat_placeholders(photo_ids.len());
        let sql = format!(
            "SELECT id, project_id, original_name, file_path, thumbnail_path, file_size, width, height, mime_type,
                    analysis_status, quality_score, is_blurred, blur_score, is_overexposed,
                    is_underexposed, exposure_score, user_decision, is_auto_culled, perceptual_hash, group_label,
                    captured_at, camera, lens, focal, aperture, shutter, iso, created_at
             FROM photos
             WHERE project_id = ? AND id IN ({placeholders})"
        );
        let params = std::iter::once(project_id.to_string()).chain(photo_ids.iter().cloned());
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params_from_iter(params), map_photo)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
    }

    fn recount_project(&self, project_id: &str) -> AppResult<()> {
        let now = Utc::now().to_rfc3339();
        let conn = self.lock();
        conn.execute(
            r#"
            UPDATE projects
            SET
                total_photos = (SELECT COUNT(*) FROM photos WHERE project_id = ?1),
                kept_count = (SELECT COUNT(*) FROM photos WHERE project_id = ?1 AND user_decision = 'keep'),
                culled_count = (
                    SELECT COUNT(*) FROM photos
                    WHERE project_id = ?1 AND (user_decision = 'cull' OR is_auto_culled = 1)
                ),
                updated_at = ?2
            WHERE id = ?1
            "#,
            params![project_id, now],
        )?;
        Ok(())
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().expect("database mutex poisoned")
    }
}

fn repeat_placeholders(count: usize) -> String {
    std::iter::repeat("?")
        .take(count)
        .collect::<Vec<_>>()
        .join(",")
}

fn map_project(row: &rusqlite::Row<'_>) -> rusqlite::Result<Project> {
    Ok(Project {
        id: row.get(0)?,
        name: row.get(1)?,
        mode: row.get(2)?,
        status: row.get(3)?,
        folder_path: row.get(4)?,
        total_photos: row.get(5)?,
        kept_count: row.get(6)?,
        culled_count: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

fn map_photo(row: &rusqlite::Row<'_>) -> rusqlite::Result<Photo> {
    Ok(Photo {
        id: row.get(0)?,
        project_id: row.get(1)?,
        original_name: row.get(2)?,
        file_path: row.get(3)?,
        thumbnail_path: row.get(4)?,
        file_size: row.get(5)?,
        width: row.get(6)?,
        height: row.get(7)?,
        mime_type: row.get(8)?,
        analysis_status: row.get(9)?,
        quality_score: row.get(10)?,
        is_blurred: row.get::<_, i64>(11)? != 0,
        blur_score: row.get(12)?,
        is_overexposed: row.get::<_, i64>(13)? != 0,
        is_underexposed: row.get::<_, i64>(14)? != 0,
        exposure_score: row.get(15)?,
        user_decision: row.get(16)?,
        is_auto_culled: row.get::<_, i64>(17)? != 0,
        perceptual_hash: row.get(18)?,
        group_label: row.get(19)?,
        captured_at: row.get(20)?,
        camera: row.get(21)?,
        lens: row.get(22)?,
        focal: row.get(23)?,
        aperture: row.get(24)?,
        shutter: row.get(25)?,
        iso: row.get(26)?,
        created_at: row.get(27)?,
    })
}

fn ensure_column(
    conn: &Connection,
    table: &str,
    column: &str,
    alter_statement: &str,
) -> AppResult<()> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let columns = stmt.query_map([], |row| row.get::<_, String>(1))?;
    for existing in columns {
        if existing? == column {
            return Ok(());
        }
    }

    conn.execute(alter_statement, [])?;
    Ok(())
}

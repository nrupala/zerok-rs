//! Version history module - Compressed snapshots with retention policy

use rusqlite::{Connection, params};
use std::collections::HashMap;
use std::fs;

/// Version entry metadata
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VersionInfo {
    pub id: String,
    pub file_id: String,
    pub created: i64,
    pub size: i64,
    pub compressed: bool,
    pub chunk_index: i32,
}

/// Version retention policy settings
#[derive(Debug, Clone)]
pub struct RetentionPolicy {
    pub max_versions: u32,
    pub max_age_days: u32,
    pub max_storage_mb: u32,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            max_versions: 5,
            max_age_days: 30,
            max_storage_mb: 1024,
        }
    }
}

/// Version manager for file version history
pub struct VersionManager {
    db: Connection,
    data_dir: String,
    policy: RetentionPolicy,
}

impl VersionManager {
    /// Create a new version manager
    pub fn new(base_path: &str) -> Result<Self, String> {
        let versions_dir = format!("{}/versions", base_path);
        fs::create_dir_all(&versions_dir).map_err(|e| e.to_string())?;
        
        let db_path = format!("{}/versions.db", base_path);
        let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
        
        // Create tables
        conn.execute(
            "CREATE TABLE IF NOT EXISTS versions (
                id TEXT PRIMARY KEY,
                file_id TEXT NOT NULL,
                created INTEGER NOT NULL,
                size INTEGER,
                compressed INTEGER,
                chunk_index INTEGER DEFAULT 0
            )",
            [],
        ).map_err(|e| e.to_string())?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_versions_file ON versions(file_id)",
            [],
        ).map_err(|e| e.to_string())?;

        Ok(Self {
            db: conn,
            data_dir: versions_dir,
            policy: RetentionPolicy::default(),
        })
    }

    /// Set retention policy
    pub fn set_policy(&mut self, policy: RetentionPolicy) {
        self.policy = policy;
    }

    /// Create a version snapshot of encrypted data
    pub fn create_version(&self, file_id: &str, encrypted_data: &[u8]) -> Result<VersionInfo, String> {
        let version_id = format!("{}_v{}", file_id, chrono::Utc::now().timestamp_millis());
        
        // Compress using gzip (simplified - would use flate2 in production)
        let data = encrypted_data;
        let compressed = false; // TODO: Implement compression
        
        let version_dir = format!("{}/{}", self.data_dir, file_id);
        fs::create_dir_all(&version_dir).map_err(|e| e.to_string())?;
        
        // Save version data
        let version_path = format!("{}/{}.dat", version_dir, version_id);
        fs::write(&version_path, data).map_err(|e| e.to_string())?;

        // Store metadata
        self.db.execute(
            "INSERT INTO versions (id, file_id, created, size, compressed, chunk_index) VALUES (?, ?, ?, ?, ?, ?)",
            params![
                version_id,
                file_id,
                chrono::Utc::now().timestamp(),
                data.len() as i64,
                compressed as i32,
                0
            ],
        ).map_err(|e| e.to_string())?;

        Ok(VersionInfo {
            id: version_id,
            file_id: file_id.to_string(),
            created: chrono::Utc::now().timestamp(),
            size: data.len() as i64,
            compressed,
            chunk_index: 0,
        })
    }

    /// Get all versions for a file
    pub fn get_versions(&self, file_id: &str) -> Result<Vec<VersionInfo>, String> {
        let mut stmt = self.db.prepare(
            "SELECT id, file_id, created, size, compressed, chunk_index FROM versions 
             WHERE file_id = ? ORDER BY created DESC"
        ).map_err(|e| e.to_string())?;

        let versions = stmt.query_map([file_id], |row| {
            Ok(VersionInfo {
                id: row.get(0)?,
                file_id: row.get(1)?,
                created: row.get(2)?,
                size: row.get(3)?,
                compressed: row.get::<_, i32>(4)? != 0,
                chunk_index: row.get(5)?,
            })
        }).map_err(|e| e.to_string())?;

        versions.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    /// Restore a specific version
    pub fn restore_version(&self, version_id: &str) -> Result<Vec<u8>, String> {
        // Get version metadata
        let (file_id,): (String,) = self.db.query_row(
            "SELECT file_id FROM versions WHERE id = ?",
            [version_id],
            |row| row.get(0),
        ).map_err(|e| e.to_string())?;

        // Read version data
        let version_path = format!("{}/{}/{}.dat", self.data_dir, file_id, version_id);
        fs::read(&version_path).map_err(|e| e.to_string())
    }

    /// Apply retention policy
    pub fn apply_retention(&self, vault_id: &str) -> Result<usize, String> {
        let now = chrono::Utc::now().timestamp();
        let max_age_seconds = (self.policy.max_age_days as i64) * 24 * 60 * 60;
        let mut deleted = 0;

        // Get all file IDs
        let mut stmt = self.db.prepare("SELECT DISTINCT file_id FROM versions")
            .map_err(|e| e.to_string())?;
        
        let file_ids: Vec<String> = stmt.query_map([], |row| row.get(0))
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();

        for file_id in file_ids {
            let versions = self.get_versions(&file_id)?;

            // 1. Keep only max_versions
            if versions.len() > self.policy.max_versions as usize {
                let to_delete = &versions[self.policy.max_versions as usize..];
                for v in to_delete {
                    self.delete_version(&v.id)?;
                    deleted += 1;
                }
            }

            // 2. Delete versions older than max_age_days
            let too_old: Vec<String> = versions.iter()
                .filter(|v| now - v.created > max_age_seconds)
                .map(|v| v.id.clone())
                .collect();
            
            for v_id in too_old {
                self.delete_version(&v_id)?;
                deleted += 1;
            }
        }

        // 3. Enforce storage limit
        deleted += self.enforce_storage_limit()?;

        Ok(deleted)
    }

    /// Enforce maximum storage limit
    fn enforce_storage_limit(&self) -> Result<usize, String> {
        let max_bytes = (self.policy.max_storage_mb as u64) * 1024 * 1024;
        
        // Get total size
        let total_size: i64 = self.db.query_row(
            "SELECT COALESCE(SUM(size), 0) FROM versions",
            [],
            |row| row.get(0),
        ).map_err(|e| e.to_string())?;

        if total_size as u64 <= max_bytes {
            return Ok(0);
        }

        // Delete oldest versions until under limit
        let target = (max_bytes as f64 * 0.9) as u64; // Keep 10% buffer
        let mut deleted = 0;

        let mut stmt = self.db.prepare(
            "SELECT id, size FROM versions ORDER BY created ASC"
        ).map_err(|e| e.to_string())?;

        let versions: Vec<(String, i64)> = stmt.query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?))
        }).map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

        let mut current_size = total_size as u64;
        
        for (version_id, size) in versions {
            if current_size <= target {
                break;
            }
            
            self.delete_version(&version_id)?;
            current_size -= size as u64;
            deleted += 1;
        }

        Ok(deleted)
    }

    /// Delete a specific version
    fn delete_version(&self, version_id: &str) -> Result<(), String> {
        // Get file_id for this version
        let (file_id,): (String,) = self.db.query_row(
            "SELECT file_id FROM versions WHERE id = ?",
            [version_id],
            |row| row.get(0),
        ).map_err(|e| e.to_string())?;

        // Delete file
        let version_path = format!("{}/{}/{}.dat", self.data_dir, file_id, version_id);
        fs::remove_file(&version_path).ok();

        // Delete metadata
        self.db.execute("DELETE FROM versions WHERE id = ?", [version_id])
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    /// Get storage statistics
    pub fn get_stats(&self) -> Result<HashMap<String, i64>, String> {
        let mut stats = HashMap::new();
        
        stats.insert("total_versions".to_string(), 
            self.db.query_row("SELECT COUNT(*) FROM versions", [], |row| row.get(0))
                .unwrap_or(0));
        
        stats.insert("total_size_bytes".to_string(),
            self.db.query_row("SELECT COALESCE(SUM(size), 0) FROM versions", [], |row| row.get(0))
                .unwrap_or(0));
        
        stats.insert("unique_files".to_string(),
            self.db.query_row("SELECT COUNT(DISTINCT file_id) FROM versions", [], |row| row.get(0))
                .unwrap_or(0));

        Ok(stats)
    }
}

use chrono;
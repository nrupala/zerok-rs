//! Storage module - File storage and deduplication

use crate::crypto::{derive_key, hash};
use rusqlite::{Connection, params};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;

/// File metadata stored in SQLite
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileInfo {
    pub id: String,
    pub name: String,
    pub size: i64,
    pub hash: String,
    pub mime_type: String,
    pub created: i64,
}

/// File store for encrypted file management
pub struct FileStore {
    db: Connection,
    data_dir: String,
    key: Option<[u8; 32]>,
}

impl FileStore {
    /// Create a new file store
    pub fn new(base_path: &str) -> Result<Self, String> {
        let data_dir = format!("{}/data", base_path);
        fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;
        
        let db_path = format!("{}/store.db", base_path);
        let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
        
        // Create tables
        conn.execute(
            "CREATE TABLE IF NOT EXISTS files (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                size INTEGER,
                hash TEXT,
                mime_type TEXT,
                created INTEGER
            )",
            [],
        ).map_err(|e| e.to_string())?;

        Ok(Self {
            db: conn,
            data_dir,
            key: None,
        })
    }

    /// Set encryption key
    pub fn set_key(&mut self, key: [u8; 32]) {
        self.key = Some(key);
    }

    /// Import and encrypt a file
    pub fn import(&self, source_path: &str) -> Result<FileInfo, String> {
        // Read source file
        let mut file = File::open(source_path).map_err(|e| e.to_string())?;
        let mut data = Vec::new();
        file.read_to_end(&mut data).map_err(|e| e.to_string())?;

        // Calculate hash for deduplication
        let file_hash = hex::encode(hash(&data));
        
        // Check for duplicates
        let existing: Vec<String> = self.db.query_row(
            "SELECT id FROM files WHERE hash = ?",
            [&file_hash],
            |row| row.get(0),
        ).ok().into_iter().collect();

        if !existing.is_empty() {
            // Return existing file info
            return Ok(FileInfo {
                id: existing[0].clone(),
                name: Path::new(source_path)
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
                size: data.len() as i64,
                hash: file_hash,
                mime_type: String::new(),
                created: chrono::Utc::now().timestamp(),
            });
        }

        // Encrypt if key is set
        let encrypted = if let Some(key) = &self.key {
            crate::crypto::encrypt(&data, key)?
        } else {
            data
        };

        // Generate ID from hash
        let id = hex::encode(&hash(&data)[..16]);

        // Save encrypted file
        let dest_path = format!("{}/{}.enc", self.data_dir, id);
        fs::write(&dest_path, &encrypted).map_err(|e| e.to_string())?;

        // Store metadata
        let name = Path::new(source_path)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let mime_type = self.guess_mime(&name);

        self.db.execute(
            "INSERT INTO files (id, name, size, hash, mime_type, created) VALUES (?, ?, ?, ?, ?, ?)",
            params![id, name, data.len() as i64, file_hash, mime_type, chrono::Utc::now().timestamp()],
        ).map_err(|e| e.to_string())?;

        Ok(FileInfo {
            id,
            name,
            size: data.len() as i64,
            hash: file_hash,
            mime_type,
            created: chrono::Utc::now().timestamp(),
        })
    }

    /// Export and decrypt a file
    pub fn export(&self, file_id: &str, dest_path: &str) -> Result<(), String> {
        // Read encrypted file
        let enc_path = format!("{}/{}.enc", self.data_dir, file_id);
        let encrypted = fs::read(&enc_path).map_err(|e| e.to_string())?;

        // Decrypt if key is set
        let data = if let Some(key) = &self.key {
            crate::crypto::decrypt(&encrypted, key)?
        } else {
            encrypted
        };

        // Write to destination
        fs::write(dest_path, data).map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Delete a file
    pub fn delete(&self, file_id: &str) -> Result<(), String> {
        // Delete encrypted file
        let enc_path = format!("{}/{}.enc", self.data_dir, file_id);
        fs::remove_file(&enc_path).ok(); // Ignore if not found

        // Delete metadata
        self.db.execute("DELETE FROM files WHERE id = ?", [file_id])
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    /// List all files
    pub fn list(&self) -> Result<Vec<FileInfo>, String> {
        let mut stmt = self.db.prepare(
            "SELECT id, name, size, hash, mime_type, created FROM files"
        ).map_err(|e| e.to_string())?;

        let files = stmt.query_map([], |row| {
            Ok(FileInfo {
                id: row.get(0)?,
                name: row.get(1)?,
                size: row.get(2)?,
                hash: row.get(3)?,
                mime_type: row.get(4)?,
                created: row.get(5)?,
            })
        }).map_err(|e| e.to_string())?;

        files.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    /// Find duplicates using hash
    pub fn find_duplicates(&self) -> Result<HashMap<String, Vec<String>>, String> {
        let mut stmt = self.db.prepare(
            "SELECT hash, group_concat(id) as ids FROM files GROUP BY hash HAVING COUNT(*) > 1"
        ).map_err(|e| e.to_string())?;

        let mut duplicates = HashMap::new();
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        }).map_err(|e| e.to_string())?;

        for row in rows {
            let (hash, ids) = row.map_err(|e| e.to_string())?;
            let id_list: Vec<String> = ids.split(',').map(|s| s.to_string()).collect();
            duplicates.insert(hash, id_list);
        }

        Ok(duplicates)
    }

    /// Verify file integrity
    pub fn verify(&self, file_id: &str) -> Result<bool, String> {
        // Try to decrypt - if it succeeds, file is valid
        let enc_path = format!("{}/{}.enc", self.data_dir, file_id);
        let encrypted = fs::read(&enc_path).map_err(|e| e.to_string())?;

        if let Some(key) = &self.key {
            Ok(crate::crypto::decrypt(&encrypted, key).is_ok())
        } else {
            Ok(true) // No encryption key, assume valid
        }
    }

    fn guess_mime(&self, filename: &str) -> String {
        let ext = Path::new(filename)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        match ext.as_str() {
            "jpg" | "jpeg" => "image/jpeg",
            "png" => "image/png",
            "gif" => "image/gif",
            "pdf" => "application/pdf",
            "mp4" => "video/mp4",
            "mp3" => "audio/mpeg",
            "txt" => "text/plain",
            "zip" => "application/zip",
            _ => "application/octet-stream",
        }.to_string()
    }
}

// Add chrono for timestamps
use chrono;
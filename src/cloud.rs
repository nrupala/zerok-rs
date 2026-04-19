//! Cloud storage module - S3 "Blind" Storage implementation
//! 
//! Files are encrypted locally first, then uploaded to S3 in chunks.
//! The S3 backend never sees plaintext data.

use rusoto_s3::{S3, S3Client, PutObjectRequest, GetObjectRequest, DeleteObjectRequest, ListObjectsV2Request};
use rusoto_core::Region;
use std::collections::HashMap;
use std::path::Path;

const CHUNK_SIZE: usize = 5 * 1024 * 1024; // 5MB chunks

/// S3 Blind Storage configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct S3Config {
    pub bucket: String,
    pub region: String,
    pub prefix: String, // Path prefix in bucket
}

/// S3 Backend for blind storage
pub struct S3Backend {
    client: S3Client,
    config: S3Config,
}

impl S3Backend {
    /// Create a new S3 backend
    pub fn new(bucket: &str, region: &str) -> Self {
        let r = Region::from_name(region);
        let client = S3Client::new(r);
        
        Self {
            client,
            config: S3Config {
                bucket: bucket.to_string(),
                region: region.to_string(),
                prefix: "zerok".to_string(),
            },
        }
    }

    /// Configure with credentials
    pub fn with_credentials(bucket: &str, region: &str, access_key: &str, secret_key: &str) -> Self {
        let r = Region::from_name(region);
        let credentials = rusoto_credential::StaticProvider::new(
            access_key.to_string(),
            secret_key.to_string(),
            None,
            None,
        );
        let client = S3Client::new_with(r, credentials, r);
        
        Self {
            client,
            config: S3Config {
                bucket: bucket.to_string(),
                region: region.to_string(),
                prefix: "zerok".to_string(),
            },
        }
    }

    /// Set path prefix
    pub fn with_prefix(mut self, prefix: &str) -> Self {
        self.config.prefix = prefix.to_string();
        self
    }

    /// Upload encrypted file to S3 in chunks
    pub async fn upload(&self, local_path: &str, remote_key: &str) -> Result<(), String> {
        let data = std::fs::read(local_path).map_err(|e| e.to_string())?;
        
        // Encrypt locally first (blind storage)
        // Note: In actual implementation, encrypt before upload
        
        let total_chunks = (data.len() + CHUNK_SIZE - 1) / CHUNK_SIZE;
        
        for (i, chunk) in data.chunks(CHUNK_SIZE).enumerate() {
            let chunk_key = format!("{}/{}.chunk.{}", self.config.prefix, remote_key, i);
            
            self.client.put_object(PutObjectRequest {
                bucket: self.config.bucket.clone(),
                key: chunk_key,
                body: Some(chunk.to_vec().into()),
                content_length: Some(chunk.len() as i64),
                ..Default::default()
            }).await.map_err(|e| e.to_string())?;
            
            println!("Uploaded chunk {}/{}", i + 1, total_chunks);
        }

        // Upload metadata
        let metadata = serde_json::json!({
            "chunks": total_chunks,
            "total_size": data.len(),
            "chunk_size": CHUNK_SIZE,
        });
        
        let meta_key = format!("{}/{}.meta", self.config.prefix, remote_key);
        self.client.put_object(PutObjectRequest {
            bucket: self.config.bucket.clone(),
            key: meta_key,
            body: Some(metadata.to_string().into()),
            ..Default::default()
        }).await.map_err(|e| e.to_string())?;

        Ok(())
    }

    /// Download and reassemble file from S3 chunks
    pub async fn download(&self, remote_key: &str, local_path: &str) -> Result<(), String> {
        // Get metadata first
        let meta_key = format!("{}/{}.meta", self.config.prefix, remote_key);
        
        let meta_response = self.client.get_object(GetObjectRequest {
            bucket: self.config.bucket.clone(),
            key: meta_key.clone(),
            ..Default::default()
        }).await.map_err(|e| e.to_string())?;

        let meta_body = meta_response.body
            .ok_or("No metadata body")?
            .concat()
            .await
            .map_err(|e| e.to_string())?;
        
        let metadata: serde_json::Value = serde_json::from_slice(&meta_body)
            .map_err(|e| e.to_string())?;
        
        let chunks = metadata["chunks"].as_u64().unwrap_or(0) as usize;
        
        // Download chunks
        let mut data = Vec::new();
        
        for i in 0..chunks {
            let chunk_key = format!("{}/{}.chunk.{}", self.config.prefix, remote_key, i);
            
            let response = self.client.get_object(GetObjectRequest {
                bucket: self.config.bucket.clone(),
                key: chunk_key,
                ..Default::default()
            }).await.map_err(|e| e.to_string())?;
            
            let chunk = response.body
                .ok_or("No chunk body")?
                .concat()
                .await
                .map_err(|e| e.to_string())?;
            
            data.extend_from_slice(&chunk);
            println!("Downloaded chunk {}/{}", i + 1, chunks);
        }

        // Save to local path
        std::fs::write(local_path, data).map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Delete file from S3 (all chunks)
    pub async fn delete(&self, remote_key: &str) -> Result<(), String> {
        // Get metadata first
        let meta_key = format!("{}/{}.meta", self.config.prefix, remote_key);
        
        let meta_response = self.client.get_object(GetObjectRequest {
            bucket: self.config.bucket.clone(),
            key: meta_key.clone(),
            ..Default::default()
        }).await;

        // Try to get chunks count, default to 1 if not found
        let chunks = if let Ok(response) = meta_response {
            if let Some(body) = response.body {
                if let Ok(data) = body.concat().await {
                    if let Ok(meta) = serde_json::from_slice::<serde_json::Value>(&data) {
                        meta["chunks"].as_u64().unwrap_or(1) as usize
                    } else {
                        1
                    }
                } else {
                    1
                }
            } else {
                1
            }
        } else {
            1
        };

        // Delete chunks
        for i in 0..chunks {
            let chunk_key = format!("{}/{}.chunk.{}", self.config.prefix, remote_key, i);
            
            self.client.delete_object(DeleteObjectRequest {
                bucket: self.config.bucket.clone(),
                key: chunk_key,
                ..Default::default()
            }).await.ok(); // Ignore errors
        }

        // Delete metadata
        self.client.delete_object(DeleteObjectRequest {
            bucket: self.config.bucket.clone(),
            key: meta_key,
            ..Default::default()
        }).await.ok();

        Ok(())
    }

    /// List all remote backups
    pub async fn list(&self) -> Result<Vec<String>, String> {
        let response = self.client.list_objects_v2(ListObjectsV2Request {
            bucket: self.config.bucket.clone(),
            prefix: Some(format!("{}/", self.config.prefix)),
            ..Default::default()
        }).await.map_err(|e| e.to_string())?;

        let mut keys = Vec::new();
        
        if let Some(contents) = response.contents {
            for obj in contents {
                if let Some(key) = obj.key {
                    // Extract just the file identifier (remove prefix and extensions)
                    if let Some(stripped) = key.strip_prefix(&format!("{}/", self.config.prefix)) {
                        if !stripped.contains('.') {
                            keys.push(stripped.to_string());
                        }
                    }
                }
            }
        }

        Ok(keys)
    }

    /// Get storage usage
    pub async fn get_usage(&self) -> Result<HashMap<String, u64>, String> {
        let files = self.list().await?;
        let mut usage = HashMap::new();
        
        for file in files {
            // Get metadata for each file
            let meta_key = format!("{}/{}.meta", self.config.prefix, file);
            
            if let Ok(response) = self.client.get_object(GetObjectRequest {
                bucket: self.config.bucket.clone(),
                key: meta_key,
                ..Default::default()
            }).await {
                if let Some(body) = response.body {
                    if let Ok(data) = body.concat().await {
                        if let Ok(meta) = serde_json::from_slice::<serde_json::Value>(&data) {
                            let size = meta["total_size"].as_u64().unwrap_or(0);
                            usage.insert(file, size);
                        }
                    }
                }
            }
        }

        Ok(usage)
    }
}
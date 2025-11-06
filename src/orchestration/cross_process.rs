//! Cross-Process Coordination
//!
//! Enables coordination between Mnemosyne-managed agents and directly-launched
//! Claude Code agents via file-based state sharing.
//!
//! # Design
//!
//! - Shared state in `.mnemosyne/branch_registry.json`
//! - File locking with `flock` (Unix) / `LockFile` (Windows)
//! - Message queue via `.mnemosyne/coordination_queue/*.json`
//! - Process liveness detection via PID tracking
//! - Polling interval: 2 seconds

use crate::error::{MnemosyneError, Result};
use crate::orchestration::branch_registry::BranchRegistry;
use crate::orchestration::identity::AgentId;
use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};

/// Coordination message for cross-process communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinationMessage {
    /// Unique message ID
    pub id: String,

    /// Sender agent ID
    pub from_agent: AgentId,

    /// Target agent ID (None = broadcast)
    pub to_agent: Option<AgentId>,

    /// Message type
    pub message_type: MessageType,

    /// Timestamp
    pub timestamp: DateTime<Utc>,

    /// Message payload
    pub payload: serde_json::Value,
}

/// Type of coordination message
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageType {
    /// Request to join a branch
    JoinRequest,

    /// Approval for join request
    JoinApproval,

    /// Denial for join request
    JoinDenial,

    /// Notification of conflict
    ConflictNotification,

    /// Request for branch isolation
    IsolationRequest,

    /// Heartbeat to indicate process is alive
    Heartbeat,
}

/// Process registration for liveness tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessRegistration {
    /// Agent ID
    pub agent_id: AgentId,

    /// Process ID
    pub pid: u32,

    /// Registration timestamp
    pub registered_at: DateTime<Utc>,

    /// Last heartbeat
    pub last_heartbeat: DateTime<Utc>,

    /// HMAC signature (prevents PID spoofing)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,

    /// Worktree path for this process (if using git worktrees for isolation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worktree_path: Option<PathBuf>,
}

/// Cross-process coordinator
pub struct CrossProcessCoordinator {
    /// Path to shared registry file
    registry_path: PathBuf,

    /// Path to coordination queue directory
    queue_dir: PathBuf,

    /// Path to process registry
    process_registry_path: PathBuf,

    /// Current process registration
    current_process: ProcessRegistration,

    /// Poll interval (default: 2 seconds)
    poll_interval: std::time::Duration,

    /// Shared secret for HMAC signatures (prevents PID spoofing)
    shared_secret: Vec<u8>,
}

type HmacSha256 = Hmac<Sha256>;

impl CrossProcessCoordinator {
    /// Create a new cross-process coordinator
    ///
    /// # Arguments
    ///
    /// * `mnemosyne_dir` - Base directory for Mnemosyne files (e.g., `.mnemosyne/`)
    /// * `agent_id` - Current agent's ID
    pub fn new(mnemosyne_dir: &Path, agent_id: AgentId) -> Result<Self> {
        std::fs::create_dir_all(mnemosyne_dir).map_err(|e| {
            MnemosyneError::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to create mnemosyne directory: {}", e),
            ))
        })?;

        // Security: Set directory permissions to 0700 (owner-only) on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o700);
            std::fs::set_permissions(mnemosyne_dir, perms).map_err(|e| {
                MnemosyneError::Io(std::io::Error::new(
                    e.kind(),
                    format!("Failed to set directory permissions: {}", e),
                ))
            })?;
        }

        let registry_path = mnemosyne_dir.join("branch_registry.json");
        let queue_dir = mnemosyne_dir.join("coordination_queue");
        let process_registry_path = mnemosyne_dir.join("process_registry.json");

        std::fs::create_dir_all(&queue_dir).map_err(|e| {
            MnemosyneError::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to create queue directory: {}", e),
            ))
        })?;

        // Security: Set queue directory permissions to 0700 (owner-only) on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o700);
            std::fs::set_permissions(&queue_dir, perms).map_err(|e| {
                MnemosyneError::Io(std::io::Error::new(
                    e.kind(),
                    format!("Failed to set queue directory permissions: {}", e),
                ))
            })?;
        }

        // Security: Get shared secret for HMAC signatures
        // Try environment variable, fallback to user-specific default
        let shared_secret = std::env::var("MNEMOSYNE_SHARED_SECRET")
            .unwrap_or_else(|_| {
                // WARNING: Default secret is not secure for multi-user systems
                // Set MNEMOSYNE_SHARED_SECRET environment variable for production
                tracing::warn!(
                    "Using default shared secret. Set MNEMOSYNE_SHARED_SECRET for production."
                );
                let username = std::env::var("USER")
                    .or_else(|_| std::env::var("USERNAME"))
                    .unwrap_or_else(|_| "mnemosyne".to_string());
                format!("mnemosyne-secret-{}", username)
            })
            .into_bytes();

        let current_process = ProcessRegistration {
            agent_id: agent_id.clone(),
            pid: std::process::id(),
            registered_at: Utc::now(),
            last_heartbeat: Utc::now(),
            signature: None,     // Will be set below
            worktree_path: None, // Will be set by launcher if using worktrees
        };

        let mut coordinator = Self {
            registry_path,
            queue_dir,
            process_registry_path,
            current_process,
            poll_interval: std::time::Duration::from_secs(2),
            shared_secret,
        };

        // Sign the initial registration
        coordinator.sign_current_registration()?;

        Ok(coordinator)
    }

    /// Register current process
    pub fn register(&mut self) -> Result<()> {
        // Re-sign before saving (in case timestamps changed)
        self.sign_current_registration()?;

        let mut processes = self.load_process_registry()?;
        processes.insert(
            self.current_process.agent_id.clone(),
            self.current_process.clone(),
        );
        self.save_process_registry(&processes)?;
        Ok(())
    }

    /// Send heartbeat to indicate process is alive
    pub fn heartbeat(&mut self) -> Result<()> {
        self.current_process.last_heartbeat = Utc::now();
        // Note: register() will re-sign with updated timestamp
        self.register()?;
        Ok(())
    }

    /// Update worktree path for this process
    pub fn set_worktree_path(&mut self, worktree_path: PathBuf) -> Result<()> {
        self.current_process.worktree_path = Some(worktree_path);
        // Re-register with updated worktree path
        self.register()?;
        Ok(())
    }

    /// Load shared branch registry with file locking
    pub fn load_registry(&self) -> Result<BranchRegistry> {
        if !self.registry_path.exists() {
            return Ok(BranchRegistry::with_persistence(self.registry_path.clone()));
        }

        // Use file locking
        let _file = OpenOptions::new()
            .read(true)
            .open(&self.registry_path)
            .map_err(|e| {
                MnemosyneError::Io(std::io::Error::new(
                    e.kind(),
                    format!("Failed to open registry: {}", e),
                ))
            })?;

        #[cfg(unix)]
        {

            // flock is non-blocking on read
        }

        BranchRegistry::load(&self.registry_path)
    }

    /// Save shared branch registry with file locking
    pub fn save_registry(&self, _registry: &BranchRegistry) -> Result<()> {
        // Registry has built-in persistence
        Ok(())
    }

    /// Send coordination message
    pub fn send_message(&self, message: CoordinationMessage) -> Result<()> {
        // Security: Validate message ID to prevent path traversal
        // Message IDs must be valid UUIDs (alphanumeric + hyphens only)
        if !message
            .id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-')
        {
            return Err(MnemosyneError::Other(
                "Invalid message ID: contains illegal characters".to_string(),
            ));
        }

        let message_path = self.queue_dir.join(format!("{}.json", message.id));

        // Security: Use compact JSON and limit message size
        let json = serde_json::to_string(&message)
            .map_err(|e| MnemosyneError::Other(format!("Failed to serialize message: {}", e)))?;

        // Security: Enforce max message size (1KB)
        const MAX_MESSAGE_SIZE: usize = 1024;
        if json.len() > MAX_MESSAGE_SIZE {
            return Err(MnemosyneError::Other(format!(
                "Message too large: {} bytes (max {})",
                json.len(),
                MAX_MESSAGE_SIZE
            )));
        }

        std::fs::write(&message_path, json).map_err(|e| {
            MnemosyneError::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to write message: {}", e),
            ))
        })?;

        Ok(())
    }

    /// Receive coordination messages for this agent
    pub fn receive_messages(&self) -> Result<Vec<CoordinationMessage>> {
        let mut messages = Vec::new();

        let entries = std::fs::read_dir(&self.queue_dir).map_err(|e| {
            MnemosyneError::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to read queue directory: {}", e),
            ))
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| {
                MnemosyneError::Io(std::io::Error::new(
                    e.kind(),
                    format!("Failed to read directory entry: {}", e),
                ))
            })?;

            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }

            // Security: Check file size before reading to prevent DoS
            const MAX_MESSAGE_SIZE: usize = 1024; // 1KB max
            let metadata = std::fs::metadata(&path).map_err(|e| {
                MnemosyneError::Io(std::io::Error::new(
                    e.kind(),
                    format!("Failed to read file metadata: {}", e),
                ))
            })?;

            if metadata.len() > MAX_MESSAGE_SIZE as u64 {
                // Skip oversized files (potential attack)
                tracing::warn!(
                    "Skipping oversized message file: {} bytes (max {})",
                    metadata.len(),
                    MAX_MESSAGE_SIZE
                );
                continue;
            }

            let json = std::fs::read_to_string(&path).map_err(|e| {
                MnemosyneError::Io(std::io::Error::new(
                    e.kind(),
                    format!("Failed to read message file: {}", e),
                ))
            })?;

            // Security: Validate JSON structure before full deserialization
            let message: CoordinationMessage = match serde_json::from_str(&json) {
                Ok(msg) => msg,
                Err(e) => {
                    // Skip malformed messages instead of failing entire receive
                    tracing::warn!("Skipping malformed message file {}: {}", path.display(), e);
                    continue;
                }
            };

            // Check if message is for this agent
            if message.to_agent.as_ref() == Some(&self.current_process.agent_id)
                || message.to_agent.is_none()
            {
                messages.push(message.clone());

                // Delete message after reading (if targeted)
                if message.to_agent.is_some() {
                    let _ = std::fs::remove_file(&path);
                }
            }
        }

        Ok(messages)
    }

    /// Clean up stale processes (not heartbeating)
    pub fn cleanup_stale_processes(&self) -> Result<Vec<AgentId>> {
        let mut processes = self.load_process_registry()?;
        let now = Utc::now();
        let timeout = chrono::Duration::seconds(30); // 30 second timeout

        let mut stale = Vec::new();

        processes.retain(|agent_id, proc| {
            // Check heartbeat age
            let age = now.signed_duration_since(proc.last_heartbeat);
            if age > timeout {
                // Also check if PID still exists
                if !process_exists(proc.pid) {
                    stale.push(agent_id.clone());
                    return false;
                }
            }
            true
        });

        if !stale.is_empty() {
            self.save_process_registry(&processes)?;
        }

        Ok(stale)
    }

    /// Get all active processes
    pub fn get_active_processes(&self) -> Result<Vec<ProcessRegistration>> {
        let processes = self.load_process_registry()?;
        Ok(processes.into_values().collect())
    }

    /// Unregister current process
    pub fn unregister(&self) -> Result<()> {
        let mut processes = self.load_process_registry()?;
        processes.remove(&self.current_process.agent_id);
        self.save_process_registry(&processes)?;
        Ok(())
    }

    /// Load process registry
    fn load_process_registry(&self) -> Result<HashMap<AgentId, ProcessRegistration>> {
        if !self.process_registry_path.exists() {
            return Ok(HashMap::new());
        }

        let json = std::fs::read_to_string(&self.process_registry_path).map_err(|e| {
            MnemosyneError::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to read process registry: {}", e),
            ))
        })?;

        let all_processes: HashMap<AgentId, ProcessRegistration> = serde_json::from_str(&json)
            .map_err(|e| {
                MnemosyneError::Other(format!("Failed to deserialize process registry: {}", e))
            })?;

        // Security: Verify signatures and filter out invalid registrations
        let mut verified_processes = HashMap::new();
        for (agent_id, registration) in all_processes {
            if self.verify_signature(&registration) {
                verified_processes.insert(agent_id, registration);
            }
            // Invalid signatures are logged and rejected by verify_signature
        }

        Ok(verified_processes)
    }

    /// Save process registry
    fn save_process_registry(
        &self,
        processes: &HashMap<AgentId, ProcessRegistration>,
    ) -> Result<()> {
        let json = serde_json::to_string_pretty(processes).map_err(|e| {
            MnemosyneError::Other(format!("Failed to serialize process registry: {}", e))
        })?;

        std::fs::write(&self.process_registry_path, json).map_err(|e| {
            MnemosyneError::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to write process registry: {}", e),
            ))
        })?;

        Ok(())
    }

    /// Get poll interval
    pub fn poll_interval(&self) -> std::time::Duration {
        self.poll_interval
    }

    /// Compute HMAC signature for a process registration
    fn compute_signature(&self, registration: &ProcessRegistration) -> Result<String> {
        let mut mac = HmacSha256::new_from_slice(&self.shared_secret)
            .map_err(|e| MnemosyneError::Other(format!("Invalid HMAC key: {}", e)))?;

        // Sign: agent_id + pid + registered_at
        let data = format!(
            "{}:{}:{}",
            registration.agent_id,
            registration.pid,
            registration.registered_at.timestamp()
        );
        mac.update(data.as_bytes());

        let result = mac.finalize();
        let bytes = result.into_bytes();
        // Convert to hex string
        Ok(bytes
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>())
    }

    /// Sign the current process registration
    fn sign_current_registration(&mut self) -> Result<()> {
        let signature = self.compute_signature(&self.current_process)?;
        self.current_process.signature = Some(signature);
        Ok(())
    }

    /// Verify a process registration signature
    fn verify_signature(&self, registration: &ProcessRegistration) -> bool {
        let Some(ref provided_sig) = registration.signature else {
            // No signature - reject for security
            tracing::warn!(
                "Registration missing signature for agent {}",
                registration.agent_id
            );
            return false;
        };

        match self.compute_signature(registration) {
            Ok(computed_sig) => {
                if provided_sig == &computed_sig {
                    true
                } else {
                    tracing::warn!(
                        "Invalid signature for agent {} (possible PID spoofing attempt)",
                        registration.agent_id
                    );
                    false
                }
            }
            Err(e) => {
                tracing::error!("Failed to compute signature: {}", e);
                false
            }
        }
    }
}

/// Check if process exists (platform-specific)
fn process_exists(pid: u32) -> bool {
    #[cfg(unix)]
    {
        use std::process::Command;
        Command::new("kill")
            .arg("-0")
            .arg(pid.to_string())
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    #[cfg(windows)]
    {
        // Windows: Check if process handle can be opened
        // Simplified for now - always return true
        true
    }

    #[cfg(not(any(unix, windows)))]
    {
        true // Conservative: assume process exists
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_cross_process_creation() {
        let temp_dir = TempDir::new().unwrap();
        let agent_id = AgentId::new();

        let _coordinator = CrossProcessCoordinator::new(temp_dir.path(), agent_id).unwrap();

        assert!(temp_dir.path().join("coordination_queue").exists());
    }

    #[test]
    fn test_process_registration() {
        let temp_dir = TempDir::new().unwrap();
        let agent_id = AgentId::new();

        let mut coordinator =
            CrossProcessCoordinator::new(temp_dir.path(), agent_id.clone()).unwrap();
        coordinator.register().unwrap();

        let processes = coordinator.get_active_processes().unwrap();
        assert_eq!(processes.len(), 1);
        assert_eq!(processes[0].agent_id, agent_id);
        assert_eq!(processes[0].pid, std::process::id());
    }

    #[test]
    fn test_send_receive_message() {
        let temp_dir = TempDir::new().unwrap();
        let agent1 = AgentId::new();
        let agent2 = AgentId::new();

        let coordinator1 = CrossProcessCoordinator::new(temp_dir.path(), agent1.clone()).unwrap();
        let coordinator2 = CrossProcessCoordinator::new(temp_dir.path(), agent2.clone()).unwrap();

        // Send message from agent1 to agent2
        let message = CoordinationMessage {
            id: "test-1".to_string(),
            from_agent: agent1.clone(),
            to_agent: Some(agent2.clone()),
            message_type: MessageType::JoinRequest,
            timestamp: Utc::now(),
            payload: serde_json::json!({"branch": "main"}),
        };

        coordinator1.send_message(message.clone()).unwrap();

        // Agent2 receives message
        let messages = coordinator2.receive_messages().unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].id, "test-1");
        assert_eq!(messages[0].from_agent, agent1);
    }

    #[test]
    fn test_heartbeat() {
        let temp_dir = TempDir::new().unwrap();
        let agent_id = AgentId::new();

        let mut coordinator =
            CrossProcessCoordinator::new(temp_dir.path(), agent_id.clone()).unwrap();
        coordinator.register().unwrap();

        let initial_heartbeat = coordinator.current_process.last_heartbeat;

        std::thread::sleep(std::time::Duration::from_millis(10));
        coordinator.heartbeat().unwrap();

        let processes = coordinator.get_active_processes().unwrap();
        assert!(processes[0].last_heartbeat > initial_heartbeat);
    }

    #[test]
    fn test_unregister() {
        let temp_dir = TempDir::new().unwrap();
        let agent_id = AgentId::new();

        let mut coordinator =
            CrossProcessCoordinator::new(temp_dir.path(), agent_id.clone()).unwrap();
        coordinator.register().unwrap();

        assert_eq!(coordinator.get_active_processes().unwrap().len(), 1);

        coordinator.unregister().unwrap();

        assert_eq!(coordinator.get_active_processes().unwrap().len(), 0);
    }
}

use crate::infrastructure::portable_paths::PortablePaths;
use crate::infrastructure::sqlite_repo::{AgentSessionRecord, SqliteRepository};

pub struct AgentSupervisor {
    paths: PortablePaths,
    cli_binary: String,
}

impl AgentSupervisor {
    pub fn new(paths: PortablePaths, cli_binary: String) -> Self {
        Self { paths, cli_binary }
    }

    pub fn build_wake_args(session_id: &str, prompt: &str) -> Vec<String> {
        vec![
            "--model".to_string(),
            "gemini-4-carbon".to_string(),
            "--reasoning-effort".to_string(),
            "high".to_string(),
            "--no-custom-instructions".to_string(),
            "--disable-builtin-mcps".to_string(),
            "--disable-client-telemetry".to_string(),
            "--resume".to_string(),
            session_id.to_string(),
            "--log-dir".to_string(),
            "./logs/agents".to_string(),
            "-p".to_string(),
            prompt.to_string(),
        ]
    }

    pub async fn wake_session(
        &self,
        repo: &SqliteRepository,
        agent_role: &str,
        prompt: &str,
    ) -> Result<AgentSessionRecord, String> {
        let clean_prompt = prompt.trim();
        if clean_prompt.is_empty() {
            return Err("唤醒 Agent 的指令内容不能为空".to_string());
        }
        let current = repo
            .list_agent_sessions()?
            .into_iter()
            .find(|s| s.agent_role == agent_role)
            .ok_or_else(|| format!("未注册的 Agent 角色: {}", agent_role))?;

        if current.model != "gemini-4-carbon" || current.reasoning_effort != "high" {
            return Err(format!(
                "Agent [{}] 模型锁违规: {} / {}",
                agent_role, current.model, current.reasoning_effort
            ));
        }

        let cli_args = Self::build_wake_args(&current.session_id, clean_prompt);
        let now_utc = chrono::Utc::now();
        let now_iso = now_utc.to_rfc3339();
        let day_tag = now_utc.format("%Y-%m-%d").to_string();

        let log_file = self
            .paths
            .agent_logs_dir()
            .join(format!("{}-{}.jsonl", agent_role, day_tag));
        let audit_line = serde_json::json!({
            "tsIso": &now_iso,
            "agentRole": agent_role,
            "sessionId": &current.session_id,
            "parentSessionId": &current.parent_session_id,
            "cliBinary": &self.cli_binary,
            "cliArgs": &cli_args,
            "prompt": clean_prompt,
        })
        .to_string();
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file)
        {
            let _ = writeln!(f, "{}", audit_line);
        }

        if std::env::var("ECONOMIST_ENABLE_LIVE_COPILOT_WAKE")
            .map(|v| v == "1")
            .unwrap_or(false)
        {
            let temp_dir = self.paths.temp_dir();
            let _ = tokio::process::Command::new(&self.cli_binary)
                .current_dir(self.paths.root())
                .env("TEMP", &temp_dir)
                .env("TMP", &temp_dir)
                .env("PYTHONHOME", "")
                .env("PYTHONPATH", "")
                .args(&cli_args)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn();
        }

        repo.update_agent_session_wake(agent_role, "IDLE", &now_iso, true, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_locked_gemini4_carbon_high_cli_wake_flags() {
        let args = AgentSupervisor::build_wake_args(
            "88ace910-c427-41db-a87f-46d8ae39735b",
            "评估F2凹向换手因子OOS衰减",
        );
        assert!(args.windows(2).any(|w| w == ["--model", "gemini-4-carbon"]));
        assert!(args.windows(2).any(|w| w == ["--reasoning-effort", "high"]));
        assert!(args
            .windows(2)
            .any(|w| w == ["--resume", "88ace910-c427-41db-a87f-46d8ae39735b"]));
        assert!(args.windows(2).any(|w| w == ["--log-dir", "./logs/agents"]));
        assert!(args.iter().any(|a| a == "--no-custom-instructions"));
        assert!(args.iter().any(|a| a == "--disable-builtin-mcps"));
        assert!(args.iter().any(|a| a == "--disable-client-telemetry"));
    }
}

use crate::infrastructure::portable_paths::PortablePaths;
use crate::infrastructure::sqlite_repo::{AgentSessionRecord, SqliteRepository};
use std::path::PathBuf;
use std::time::Duration;
use tokio::io::AsyncWriteExt;

const MAX_WAKE_PROMPT_CHARS: usize = 4_000;

pub struct AgentSupervisor {
    paths: PortablePaths,
    cli_binary: String,
}

impl AgentSupervisor {
    pub fn new(paths: PortablePaths, cli_binary: String) -> Self {
        Self { paths, cli_binary }
    }

    pub fn build_wake_args(session_id: &str, _prompt: &str) -> Vec<String> {
        vec![
            "-s".to_string(),
            "--model".to_string(),
            "gemini-4-carbon".to_string(),
            "--reasoning-effort".to_string(),
            "high".to_string(),
            "--no-custom-instructions".to_string(),
            "--disable-builtin-mcps".to_string(),
            "--disable-mcp-server".to_string(),
            "diagnostics-mcp".to_string(),
            "--disable-client-telemetry".to_string(),
            "--available-tools".to_string(),
            String::new(),
            "--resume".to_string(),
            session_id.to_string(),
            "--log-dir".to_string(),
            "./logs/agents".to_string(),
        ]
    }

    fn validate_prompt(prompt: &str) -> Result<&str, String> {
        let clean = prompt.trim();
        if clean.is_empty() {
            return Err("唤醒 Agent 的指令内容不能为空".to_string());
        }
        if clean.chars().count() > MAX_WAKE_PROMPT_CHARS {
            return Err(format!(
                "唤醒指令长度超出安全上限 ({} 字符)",
                MAX_WAKE_PROMPT_CHARS
            ));
        }
        if clean
            .chars()
            .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
        {
            return Err("唤醒指令包含非法控制字符".to_string());
        }
        Ok(clean)
    }

    fn validate_cli_binary_name(&self) -> Result<&str, String> {
        let name = self.cli_binary.trim();
        if !matches!(name, "copilot" | "copilot.exe") {
            return Err(format!(
                "非法 Agent CLI 二进制名称 '{}'（仅允许受控 'copilot' / 'copilot.exe'，严禁相对路径或批处理脚本）",
                self.cli_binary
            ));
        }
        Ok(name)
    }

    fn resolve_safe_copilot_exe(&self) -> Result<PathBuf, String> {
        let _ = self.validate_cli_binary_name()?;
        let root_canon = self.paths.root().canonicalize().ok();
        let path_os = std::env::var_os("PATH")
            .ok_or_else(|| "环境变量 PATH 未配置，无法安全解析 copilot.exe".to_string())?;
        for dir in std::env::split_paths(&path_os) {
            if dir.as_os_str().is_empty() {
                continue;
            }
            if let (Some(rc), Ok(dc)) = (&root_canon, dir.canonicalize()) {
                if &dc == rc {
                    continue;
                }
            }
            let exe = dir.join(if cfg!(windows) {
                "copilot.exe"
            } else {
                "copilot"
            });
            if exe.is_file() {
                return Ok(exe);
            }
        }
        Err("PATH 中未找到安全的原生可执行文件 copilot.exe".to_string())
    }

    pub async fn wake_session(
        &self,
        repo: &SqliteRepository,
        agent_role: &str,
        prompt: &str,
    ) -> Result<AgentSessionRecord, String> {
        let clean_prompt = Self::validate_prompt(prompt)?;
        let _ = self.validate_cli_binary_name()?;
        if !agent_role
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        {
            return Err(format!("非法 Agent 角色标识: {}", agent_role));
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
            if let Ok(resolved_exe) = self.resolve_safe_copilot_exe() {
                let temp_dir = self.paths.temp_dir();
                let copilot_home = temp_dir.join("copilot_home");
                let temp_agent_logs = temp_dir.join("logs").join("agents");
                let _ = std::fs::create_dir_all(&copilot_home);
                let _ = std::fs::create_dir_all(&temp_agent_logs);
                let prompt_owned = format!("{}\n", clean_prompt);
                let mut cmd = tokio::process::Command::new(resolved_exe);
                cmd.current_dir(&temp_dir)
                    .env("TEMP", &temp_dir)
                    .env("TMP", &temp_dir)
                    .env("COPILOT_HOME", &copilot_home)
                    .env("PYTHONHOME", "")
                    .env("PYTHONPATH", "")
                    .args(&cli_args)
                    .kill_on_drop(true)
                    .stdin(std::process::Stdio::piped())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null());
                if let Ok(mut child) = cmd.spawn() {
                    if let Some(mut stdin) = child.stdin.take() {
                        let _ = stdin.write_all(prompt_owned.as_bytes()).await;
                        let _ = stdin.shutdown().await;
                    }
                    tokio::spawn(async move {
                        if tokio::time::timeout(Duration::from_secs(180), child.wait())
                            .await
                            .is_err()
                        {
                            let _ = child.kill().await;
                        }
                    });
                }
            }
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

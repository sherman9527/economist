use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone)]
pub struct PortablePaths {
    root: PathBuf,
}

impl PortablePaths {
    pub fn resolve() -> Result<Self, String> {
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                if exe_dir.join("config").join("app.toml").is_file() {
                    return Self::from_root(exe_dir.to_path_buf());
                }
                let exe_str = exe_dir.to_string_lossy().to_ascii_lowercase();
                let is_cargo_artifact = exe_str.contains("cargo-target")
                    || exe_str.contains("\\target\\")
                    || exe_str.contains("/target/")
                    || exe_str.ends_with("\\deps")
                    || exe_str.ends_with("/deps");
                if !is_cargo_artifact {
                    return Self::from_root(exe_dir.to_path_buf());
                }
                let mut cursor = Some(exe_dir);
                while let Some(dir) = cursor {
                    if dir.join("config").join("app.toml").is_file()
                        || dir.join("src-tauri").join("Cargo.toml").is_file()
                    {
                        return Self::from_root(dir.to_path_buf());
                    }
                    cursor = dir.parent();
                }
            }
        }

        let cwd = std::env::current_dir().map_err(|e| format!("读取当前工作目录失败: {}", e))?;
        let mut cursor: Option<&Path> = Some(cwd.as_path());
        while let Some(dir) = cursor {
            if dir.join("config").join("app.toml").is_file()
                || dir.join("src-tauri").join("Cargo.toml").is_file()
            {
                return Self::from_root(dir.to_path_buf());
            }
            cursor = dir.parent();
        }
        Self::from_root(cwd)
    }

    pub fn from_root(root: PathBuf) -> Result<Self, String> {
        let instance = Self { root };
        for dir in [
            instance.config_dir(),
            instance.data_dir(),
            instance.plugins_dir().join("strategies"),
            instance.plugins_dir().join("factors"),
            instance.reports_dir(),
            instance.agent_logs_dir(),
            instance.telemetry_dir(),
            instance.webview_data_dir(),
            instance.temp_dir(),
        ] {
            std::fs::create_dir_all(&dir)
                .map_err(|e| format!("初始化便携运行子目录失败: {}", e))?;
        }
        Ok(instance)
    }

    pub fn apply_process_env(&self) {
        std::env::set_var("WEBVIEW2_USER_DATA_FOLDER", self.webview_data_dir());
        std::env::set_var("TEMP", self.temp_dir());
        std::env::set_var("TMP", self.temp_dir());
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn config_dir(&self) -> PathBuf {
        self.root.join("config")
    }

    pub fn data_dir(&self) -> PathBuf {
        self.root.join("data")
    }

    pub fn sqlite_db_path(&self) -> PathBuf {
        self.data_dir().join("economist_quant.db")
    }

    pub fn plugins_dir(&self) -> PathBuf {
        self.root.join("plugins")
    }

    pub fn reports_dir(&self) -> PathBuf {
        self.root.join("reports")
    }

    pub fn logs_dir(&self) -> PathBuf {
        self.root.join("logs")
    }

    pub fn agent_logs_dir(&self) -> PathBuf {
        self.logs_dir().join("agents")
    }

    pub fn telemetry_dir(&self) -> PathBuf {
        self.root.join("telemetry")
    }

    pub fn webview_data_dir(&self) -> PathBuf {
        self.root.join("webview_data")
    }

    pub fn temp_dir(&self) -> PathBuf {
        self.root.join(".temp")
    }

    pub fn resolve_guarded_report_path(&self, rel_path: &str) -> Result<PathBuf, String> {
        let normalized = normalize_rel_path(rel_path)?;
        let stripped = normalized
            .strip_prefix("./reports/")
            .or_else(|| normalized.strip_prefix("./"))
            .unwrap_or(&normalized);

        if stripped.is_empty() || !stripped.to_ascii_lowercase().ends_with(".html") {
            return Err("仅允许读取 ./reports/ 目录下的 .html 研报文件".to_string());
        }

        let reports_base = self.reports_dir();
        if let Ok(base_meta) = std::fs::symlink_metadata(&reports_base) {
            if base_meta.file_type().is_symlink() {
                return Err("拒绝通过符号链接访问 ./reports 目录".to_string());
            }
        }

        let mut cur = reports_base;
        for seg in stripped.split('/').filter(|s| !s.is_empty()) {
            cur.push(seg);
            let meta = std::fs::symlink_metadata(&cur)
                .map_err(|_| format!("研报文件不存在: {}", normalized))?;
            if meta.file_type().is_symlink() {
                return Err("检测到符号链接越界风险，已拒绝读取研报文件".to_string());
            }
        }
        if !cur.is_file() {
            return Err(format!("研报文件不存在: {}", normalized));
        }
        Ok(cur)
    }
}

pub fn normalize_rel_path(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("相对路径不能为空".to_string());
    }
    if trimmed.contains(':') {
        return Err(format!("严禁使用带盘符或协议的绝对路径: {}", trimmed));
    }
    let unified = trimmed.replace('\\', "/");
    if unified.starts_with('/') {
        return Err(format!("严禁使用根路径绝对寻址: {}", trimmed));
    }

    let mut segments: Vec<&str> = Vec::new();
    for comp in Path::new(&unified).components() {
        match comp {
            Component::CurDir => {}
            Component::Normal(os) => {
                let s = os
                    .to_str()
                    .ok_or_else(|| "路径包含非法UTF-8字符".to_string())?;
                if !s.is_empty() {
                    segments.push(s);
                }
            }
            Component::ParentDir | Component::Prefix(_) | Component::RootDir => {
                return Err(format!("严禁父级越界或绝对路径组件: {}", trimmed));
            }
        }
    }

    if segments.is_empty() {
        Ok("./".to_string())
    } else {
        Ok(format!("./{}", segments.join("/")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_valid_relative_paths_and_rejects_drive_or_traversal() {
        assert_eq!(
            normalize_rel_path("reports\\2026\\11\\18\\economist_report.html").unwrap(),
            "./reports/2026/11/18/economist_report.html"
        );
        assert_eq!(
            normalize_rel_path("./plugins/factors/core_factors_v1.json").unwrap(),
            "./plugins/factors/core_factors_v1.json"
        );
        // Reject drive colon or parent traversal
        let bad_drive = format!("{}:/secret/file.txt", 'X');
        assert!(normalize_rel_path(&bad_drive).is_err());
        assert!(normalize_rel_path("../outside/secret.html").is_err());
    }

    #[test]
    fn creates_portable_directories_and_blocks_report_path_traversal() {
        let test_root = PathBuf::from(".temp").join("test_portable_paths");
        let paths = PortablePaths::from_root(test_root.clone()).unwrap();
        assert!(paths.config_dir().is_dir());
        assert!(paths.webview_data_dir().is_dir());
        assert!(paths.temp_dir().is_dir());

        let sample_dir = paths.reports_dir().join("2026").join("11");
        std::fs::create_dir_all(&sample_dir).unwrap();
        let sample_html = sample_dir.join("demo.html");
        std::fs::write(&sample_html, "<html>ok</html>").unwrap();

        assert!(paths
            .resolve_guarded_report_path("2026/11/demo.html")
            .is_ok());
        assert!(paths
            .resolve_guarded_report_path("./reports/2026/11/demo.html")
            .is_ok());
        assert!(paths
            .resolve_guarded_report_path("../../config/app.toml")
            .is_err());
    }
}

use crate::shortcuts::{Action, Binding};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "FileConfig")]
pub struct Config {
    pub shell: String,
    pub font: String,
    pub island: crate::island::Config,
    pub shortcuts: BTreeMap<String, String>,
}
#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct FileConfig {
    shell: Option<String>,
    font: Option<String>,
    island: crate::island::Config,
    shortcuts: BTreeMap<String, String>,
}
impl From<FileConfig> for Config {
    fn from(file: FileConfig) -> Self {
        let mut config = Self::default();
        if let Some(shell) = file.shell {
            config.shell = shell;
        }
        if let Some(font) = file.font {
            config.font = font.trim().to_string();
        }
        config.shortcuts.extend(file.shortcuts);
        config.island = file.island;
        config
    }
}
impl Default for Config {
    fn default() -> Self {
        Self {
            shell: default_shell().into(),
            font: default_font(),
            island: crate::island::Config::default(),
            shortcuts: Action::all()
                .into_iter()
                .map(|a| (a.id(), a.default_binding()))
                .collect(),
        }
    }
}
impl Config {
    pub fn load() -> Result<Self, String> {
        Self::load_paths(&paths())
    }
    fn load_paths(paths: &[PathBuf]) -> Result<Self, String> {
        for path in paths {
            match fs::read_to_string(path) {
                Ok(text) => {
                    return Self::parse(&text).map_err(|e| format!("{}: {e}", path.display()))
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => return Err(format!("{}: {e}", path.display())),
            }
        }
        Ok(Self::default())
    }
    pub fn parse(text: &str) -> Result<Self, String> {
        let config: Self = toml::from_str(text).map_err(|e| e.to_string())?;
        config.validate()?;
        Ok(config)
    }
    pub fn bindings(&self) -> Result<Vec<(Action, Binding)>, String> {
        let mut result: Vec<(Action, Binding)> = Vec::new();
        for (id, text) in &self.shortcuts {
            if !Action::all().iter().any(|a| a.id() == *id) {
                return Err(format!("Unknown shortcut action {id:?}"));
            }
            if text.is_empty() {
                continue;
            }
        }
        for action in Action::all() {
            let text = self
                .shortcuts
                .get(&action.id())
                .map(String::as_str)
                .unwrap_or("");
            if text.is_empty() {
                continue;
            }
            let binding = Binding::parse(text).map_err(|e| format!("{}: {e}", action.label()))?;
            if let Some((other, _)) = result.iter().find(|(_, b)| *b == binding) {
                return Err(format!(
                    "{} is assigned to both {} and {}",
                    binding.display(),
                    other.label(),
                    action.label()
                ));
            }
            result.push((action, binding));
        }
        Ok(result)
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.shell.trim().is_empty() {
            return Err(format!(
                "Enter a shell executable, such as {}",
                default_shell()
            ));
        }
        if self.font.trim().is_empty() {
            return Err("Choose a terminal font".into());
        }
        self.bindings().map(|_| ())
    }
    pub fn save_to(&self, path: &Path) -> Result<(), String> {
        self.validate()?;
        let text = toml::to_string_pretty(self).map_err(|e| e.to_string())?;
        let parent = path.parent().ok_or("Invalid configuration directory")?;
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        let temp = parent.join(format!(".dterm-{}.tmp", std::process::id()));
        let result = (|| -> std::io::Result<()> {
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp)?;
            file.write_all(text.as_bytes())?;
            file.sync_all()?;
            fs::rename(&temp, path)
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temp);
        }
        result.map_err(|e| e.to_string())
    }
    pub fn shell_command(&self) -> Result<Vec<String>, String> {
        resolve_shell(&self.shell, std::env::var_os("PATH").as_deref())
            .map(|path| vec![path.to_string_lossy().into_owned()])
    }
}
pub fn path() -> PathBuf {
    delib::xdg::XdgPaths::default().user_config_file("dterm", "dterm.toml")
}
pub fn paths() -> Vec<PathBuf> {
    delib::xdg::XdgPaths::default().config_files("dterm", "dterm.toml")
}
pub fn fingerprint() -> Vec<Option<Vec<u8>>> {
    paths().iter().map(|p| fs::read(p).ok()).collect()
}
pub fn default_shell() -> &'static str {
    crate::backends::default_shell()
}
pub fn default_font() -> String {
    crate::backends::default_font()
}
fn resolve_shell(shell: &str, search: Option<&std::ffi::OsStr>) -> Result<PathBuf, String> {
    let mut names = vec![shell];
    for &fallback in crate::backends::fallback_shells() {
        if !names.contains(&fallback) {
            names.push(fallback);
        }
    }
    for name in names {
        let p = Path::new(name);
        let candidates = if p.components().count() > 1 {
            crate::backends::executable_candidates(p)
        } else {
            search
                .map(std::env::split_paths)
                .into_iter()
                .flatten()
                .flat_map(|dir| crate::backends::executable_candidates(&dir.join(p)))
                .collect()
        };
        if let Some(path) = candidates.into_iter().find(|path| {
            fs::metadata(path).is_ok_and(|meta| crate::backends::is_executable_file(&meta))
        }) {
            return if path.is_absolute() {
                Ok(path)
            } else {
                std::env::current_dir()
                    .map(|cwd| cwd.join(path))
                    .map_err(|e| e.to_string())
            };
        }
    }
    Err(crate::backends::shell_missing_message(shell))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "dterm-config-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&p).unwrap();
        p
    }
    #[test]
    fn partial_config_merges_defaults_and_rejects_conflicts() {
        let c = Config::parse("shell='zsh'\n[shortcuts]\nnew_tab='Ctrl+Alt+T'\ncopy=''\n").unwrap();
        assert_eq!(c.shell, "zsh");
        assert_eq!(c.font, Config::default().font);
        assert_eq!(c.shortcuts["new_window"], "Ctrl+Shift+N");
        assert_eq!(c.shortcuts["copy"], "");
        assert!(Config::parse("[shortcuts]\nnew_tab='Ctrl+Shift+N'").is_err());
        assert!(Config::parse("shell=''").is_err());
        assert!(Config::parse("font=''").is_err());
        assert_eq!(
            Config::parse("font='Consolas'").unwrap().font,
            "Consolas"
        );
        assert!(Config::parse("[shortcuts]\nunknown='Ctrl+K'").is_err());
    }
    #[test]
    fn island_config_defaults_roundtrip_and_validation() {
        use crate::island::{Horizontal, Orientation, Vertical};
        assert_eq!(
            Config::parse("shell='bash'").unwrap().island,
            crate::island::Config::default()
        );
        let config = Config::parse("[island]\nenabled=false\norientation='vertical'\nhorizontal='right'\nvertical='center'").unwrap();
        assert!(!config.island.enabled);
        assert_eq!(config.island.orientation, Orientation::Vertical);
        assert_eq!(config.island.horizontal, Horizontal::Right);
        assert_eq!(config.island.vertical, Vertical::Center);
        assert_eq!(
            Config::parse(&toml::to_string(&config).unwrap()).unwrap(),
            config
        );
        let partial = Config::parse("[island]\nhorizontal='left'").unwrap();
        assert!(partial.island.enabled);
        assert_eq!(partial.island.vertical, Vertical::Bottom);
        assert!(Config::parse("[island]\norientation='diagonal'").is_err());
        assert!(Config::parse("[island]\nvertical='left'").is_err());
        assert!(Config::parse("[island]\nunknown=true").is_err());
    }
    #[test]
    fn config_roundtrip_and_system_fallback() {
        let root = fixture();
        let user = root.join("user/dterm.toml");
        let system = root.join("system/dterm.toml");
        let c = Config {
            shell: "zsh".into(),
            ..Default::default()
        };
        c.save_to(&system).unwrap();
        assert_eq!(Config::load_paths(&[user.clone(), system]).unwrap(), c);
        Config::default().save_to(&user).unwrap();
        assert_eq!(Config::load_paths(&[user]).unwrap(), Config::default());
        fs::remove_dir_all(root).unwrap();
    }
    #[cfg(unix)]
    #[test]
    fn shell_falls_back_in_order_and_ignores_non_executables() {
        use std::os::unix::fs::PermissionsExt;
        let root = fixture();
        let path = std::ffi::OsString::from(&root);
        for name in ["sh", "bash", "custom"] {
            fs::write(root.join(name), "#!/bin/sh\n").unwrap();
            fs::set_permissions(root.join(name), fs::Permissions::from_mode(0o755)).unwrap();
        }
        assert_eq!(
            resolve_shell("custom", Some(&path)).unwrap(),
            root.join("custom")
        );
        assert_eq!(
            resolve_shell("missing", Some(&path)).unwrap(),
            root.join("sh")
        );
        fs::remove_file(root.join("sh")).unwrap();
        assert_eq!(resolve_shell("sh", Some(&path)).unwrap(), root.join("bash"));
        fs::set_permissions(root.join("bash"), fs::Permissions::from_mode(0o644)).unwrap();
        assert!(resolve_shell("missing", Some(&path)).is_err());
        fs::remove_dir_all(root).unwrap();
    }
    #[cfg(windows)]
    #[test]
    fn shell_falls_back_to_windows_executables() {
        let root = fixture();
        let path = std::ffi::OsString::from(&root);
        fs::write(root.join("custom"), "not executable").unwrap();
        for name in ["powershell.exe", "pwsh.exe", "cmd.exe", "custom.exe"] {
            fs::write(root.join(name), "").unwrap();
        }
        assert_eq!(
            resolve_shell("custom", Some(&path))
                .unwrap()
                .to_string_lossy()
                .to_ascii_lowercase(),
            root.join("custom.exe").to_string_lossy().to_ascii_lowercase()
        );
        assert_eq!(
            resolve_shell("missing", Some(&path))
                .unwrap()
                .to_string_lossy()
                .to_ascii_lowercase(),
            root.join("powershell.exe")
                .to_string_lossy()
                .to_ascii_lowercase()
        );
        fs::remove_file(root.join("powershell.exe")).unwrap();
        assert_eq!(
            resolve_shell("missing", Some(&path))
                .unwrap()
                .to_string_lossy()
                .to_ascii_lowercase(),
            root.join("pwsh.exe").to_string_lossy().to_ascii_lowercase()
        );
        fs::remove_file(root.join("pwsh.exe")).unwrap();
        assert_eq!(
            resolve_shell("missing", Some(&path))
                .unwrap()
                .to_string_lossy()
                .to_ascii_lowercase(),
            root.join("cmd.exe").to_string_lossy().to_ascii_lowercase()
        );
        fs::remove_file(root.join("cmd.exe")).unwrap();
        assert!(resolve_shell("missing", Some(&path)).is_err());
        fs::remove_dir_all(root).unwrap();
    }
}

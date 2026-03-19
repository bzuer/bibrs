/// Application configuration loaded from INI file.
///
/// Default location: `~/.config/bibrs/config.ini`.
/// File is optional — all fields have sensible defaults.
use std::path::{Path, PathBuf};

/// Top-level configuration.
#[derive(Debug, Clone, Default)]
pub struct Config {
    pub serialize: SerializeConfig,
    pub normalize: NormalizeConfig,
    pub citekey: CitekeyConfig,
    pub dedup: DedupConfig,
    pub sources: SourcesConfig,
    pub cache: CacheConfig,
}

/// Serialization formatting options.
#[derive(Debug, Clone)]
pub struct SerializeConfig {
    pub indent: String,
    pub align_equals: bool,
    pub trailing_comma: bool,
    pub field_order: Vec<String>,
}

/// Normalization behavior options.
#[derive(Debug, Clone)]
pub struct NormalizeConfig {
    pub name_format: String,
    pub protect_acronyms: bool,
    pub doi_strip_prefix: bool,
}

/// Cite key generation options.
#[derive(Debug, Clone)]
pub struct CitekeyConfig {
    pub pattern: String,
    pub lowercase: bool,
    pub dedup_suffix: String,
}

/// Duplicate detection options.
#[derive(Debug, Clone)]
pub struct DedupConfig {
    pub fuzzy_threshold: f64,
}

/// External API source options.
#[derive(Debug, Clone)]
pub struct SourcesConfig {
    pub mailto: String,
    pub default_sources: Vec<String>,
}

/// Disk cache options.
#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub enabled: bool,
    pub ttl_search_days: u32,
    pub ttl_id_days: u32,
}

impl Default for SerializeConfig {
    fn default() -> Self {
        Self {
            indent: "  ".into(),
            align_equals: false,
            trailing_comma: true,
            field_order: vec![
                "author".into(),
                "title".into(),
                "year".into(),
                "journal".into(),
                "volume".into(),
                "pages".into(),
                "doi".into(),
            ],
        }
    }
}

impl Default for NormalizeConfig {
    fn default() -> Self {
        Self {
            name_format: "last_comma_first".into(),
            protect_acronyms: true,
            doi_strip_prefix: true,
        }
    }
}

impl Default for CitekeyConfig {
    fn default() -> Self {
        Self {
            pattern: "{auth}{year}{shorttitle}".into(),
            lowercase: true,
            dedup_suffix: "alpha".into(),
        }
    }
}

impl Default for DedupConfig {
    fn default() -> Self {
        Self {
            fuzzy_threshold: 0.90,
        }
    }
}

impl Default for SourcesConfig {
    fn default() -> Self {
        Self {
            mailto: String::new(),
            default_sources: vec!["crossref".into(), "openalex".into()],
        }
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            ttl_search_days: 7,
            ttl_id_days: 30,
        }
    }
}

impl Config {
    /// Returns the default config file path (`~/.config/bibrs/config.ini`).
    pub fn default_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("bibrs").join("config.ini"))
    }

    /// Loads configuration from the default path, falling back to defaults.
    pub fn load() -> Self {
        match Self::default_path() {
            Some(path) if path.exists() => Self::load_from(&path).unwrap_or_default(),
            _ => Self::default(),
        }
    }

    /// Loads configuration from a specific INI file path.
    pub fn load_from(path: &Path) -> Result<Self, String> {
        let ini = ini::Ini::load_from_file(path)
            .map_err(|e| format!("failed to read config: {}", e))?;

        let mut config = Self::default();

        if let Some(section) = ini.section(Some("serialize")) {
            if let Some(v) = section.get("indent") {
                config.serialize.indent = v.to_string();
            }
            if let Some(v) = section.get("align_equals") {
                config.serialize.align_equals = v == "true";
            }
            if let Some(v) = section.get("trailing_comma") {
                config.serialize.trailing_comma = v == "true";
            }
            if let Some(v) = section.get("field_order") {
                config.serialize.field_order = v
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
        }

        if let Some(section) = ini.section(Some("normalize")) {
            if let Some(v) = section.get("name_format") {
                config.normalize.name_format = v.to_string();
            }
            if let Some(v) = section.get("protect_acronyms") {
                config.normalize.protect_acronyms = v == "true";
            }
            if let Some(v) = section.get("doi_strip_prefix") {
                config.normalize.doi_strip_prefix = v == "true";
            }
        }

        if let Some(section) = ini.section(Some("citekey")) {
            if let Some(v) = section.get("pattern") {
                config.citekey.pattern = v.to_string();
            }
            if let Some(v) = section.get("lowercase") {
                config.citekey.lowercase = v == "true";
            }
            if let Some(v) = section.get("dedup_suffix") {
                config.citekey.dedup_suffix = v.to_string();
            }
        }

        if let Some(section) = ini.section(Some("dedup")) {
            if let Some(v) = section.get("fuzzy_threshold") {
                if let Ok(f) = v.parse::<f64>() {
                    config.dedup.fuzzy_threshold = f;
                }
            }
        }

        if let Some(section) = ini.section(Some("sources")) {
            if let Some(v) = section.get("mailto") {
                config.sources.mailto = v.to_string();
            }
            if let Some(v) = section.get("default_sources") {
                config.sources.default_sources = v
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
        }

        if let Some(section) = ini.section(Some("cache")) {
            if let Some(v) = section.get("enabled") {
                config.cache.enabled = v == "true";
            }
            if let Some(v) = section.get("ttl_search_days") {
                if let Ok(n) = v.parse::<u32>() {
                    config.cache.ttl_search_days = n;
                }
            }
            if let Some(v) = section.get("ttl_id_days") {
                if let Ok(n) = v.parse::<u32>() {
                    config.cache.ttl_id_days = n;
                }
            }
        }

        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid() {
        let config = Config::default();
        assert_eq!(config.serialize.indent, "  ");
        assert!(config.serialize.trailing_comma);
        assert!(!config.serialize.align_equals);
        assert_eq!(config.normalize.name_format, "last_comma_first");
        assert!(config.normalize.protect_acronyms);
        assert_eq!(config.citekey.pattern, "{auth}{year}{shorttitle}");
        assert_eq!(config.dedup.fuzzy_threshold, 0.90);
        assert!(config.sources.mailto.is_empty());
        assert!(config.cache.enabled);
        assert_eq!(config.cache.ttl_search_days, 7);
        assert_eq!(config.cache.ttl_id_days, 30);
    }

    #[test]
    fn load_from_nonexistent_file() {
        let result = Config::load_from(Path::new("/nonexistent/config.ini"));
        assert!(result.is_err());
    }

    #[test]
    fn load_from_ini_string() {
        let dir = std::env::temp_dir().join("bibrs_test_config");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.ini");
        std::fs::write(
            &path,
            "[sources]\nmailto = test@example.com\n\n[cache]\nenabled = false\nttl_id_days = 60\n",
        )
        .unwrap();

        let config = Config::load_from(&path).unwrap();
        assert_eq!(config.sources.mailto, "test@example.com");
        assert!(!config.cache.enabled);
        assert_eq!(config.cache.ttl_id_days, 60);
        assert_eq!(config.cache.ttl_search_days, 7);

        std::fs::remove_file(&path).ok();
        std::fs::remove_dir(&dir).ok();
    }
}

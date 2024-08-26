use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;
use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;
use poise::serenity_prelude::GuildId;
use parking_lot::RwLock;
use regex::Regex;
use serde_yaml::Value;
use crate::database::manager::DbManager;
use crate::database::models::GuildSettings;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    English,
    Polish,
}

impl Language {
    pub fn as_str(&self) -> &str {
        match self {
            Language::English => "en",
            Language::Polish => "pl",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, ()> {
        match s {
            "en" => Ok(Language::English),
            "pl" => Ok(Language::Polish),
            _ => Err(()),
        }
    }
}

impl FromStr for Language {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "en" => Ok(Language::English),
            "pl" => Ok(Language::Polish),
            _ => Err(()),
        }
    }
}

pub struct LocalizationManager {
    translations: Arc<RwLock<HashMap<Language, HashMap<String, String>>>>,
    default_lang: Language,
}

#[derive(Clone)]
pub struct TranslationRef {
    key: String,
    params: Vec<String>,
}

impl TranslationRef {
    pub fn new<S: Into<String>>(key: S, params: Vec<String>) -> Self {
        TranslationRef { key: key.into(), params }
    }

    pub fn to_translation_params(&self) -> Vec<TranslationParam> {
        self.params.iter().map(|p| TranslationParam::String(p.clone())).collect()
    }
}

impl LocalizationManager {
    pub fn new(default_lang: Language) -> Result<Self, Box<dyn std::error::Error>> {
        let manager = LocalizationManager {
            translations: Arc::new(RwLock::new(HashMap::new())),
            default_lang,
        };
        manager.load_translations()?;
        Ok(manager)
    }

    fn load_translations(&self) -> Result<(), Box<dyn std::error::Error>> {
        let resources_dir = Path::new("bytemate-translations/resource");
        let mut translations = HashMap::new();

        let lang_dirs = self.find_language_dirs(resources_dir)?;

        for (lang, lang_dir) in lang_dirs {
            let lang_translations = self.load_language_translations(&lang_dir)?;
            translations.insert(lang, lang_translations);
        }

        let mut lock = self.translations.write();
        *lock = translations;
        Ok(())
    }

    fn find_language_dirs(&self, base_dir: &Path) -> Result<Vec<(Language, PathBuf)>, Box<dyn std::error::Error>> {
        let mut lang_dirs = Vec::new();

        for entry in fs::read_dir(base_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                let lang_str = path.file_name().unwrap().to_str().unwrap();
                if let Ok(lang) = Language::from_str(lang_str) {
                    lang_dirs.push((lang, path));
                }
            }
        }

        Ok(lang_dirs)
    }

    fn load_language_translations(&self, lang_dir: &Path) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
        let mut translations = HashMap::new();

        self.load_translations_recursive(lang_dir, &mut translations, String::new())?;

        Ok(translations)
    }

    fn load_translations_recursive(
        &self,
        dir: &Path,
        translations: &mut HashMap<String, String>,
        prefix: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                let new_prefix = if prefix.is_empty() {
                    path.file_name().unwrap().to_str().unwrap().to_string()
                } else {
                    format!("{}.{}", prefix, path.file_name().unwrap().to_str().unwrap())
                };
                self.load_translations_recursive(&path, translations, new_prefix)?;
            } else if path.is_file() && path.extension().unwrap_or_default() == "yaml" {
                let file_prefix = if prefix.is_empty() {
                    path.file_stem().unwrap().to_str().unwrap().to_string()
                } else {
                    format!("{}.{}", prefix, path.file_stem().unwrap().to_str().unwrap())
                };
                let content = fs::read_to_string(&path)?;
                let yaml: Value = serde_yaml::from_str(&content)?;
                self.flatten_yaml(&yaml, &file_prefix, translations);
            }
        }
        Ok(())
    }

    fn flatten_yaml(&self, yaml: &Value, prefix: &str, result: &mut HashMap<String, String>) {
        match yaml {
            Value::Mapping(map) => {
                for (key, value) in map {
                    let key_str = key.as_str().unwrap_or("");
                    let new_prefix = if prefix.is_empty() {
                        key_str.to_string()
                    } else {
                        format!("{}.{}", prefix, key_str)
                    };
                    self.flatten_yaml(value, &new_prefix, result);
                }
            }
            Value::String(s) => {
                result.insert(prefix.to_string(), s.clone());
            }
            _ => {}
        }
    }

    pub fn get(&self, key: &str, lang: Language, params: &[TranslationParam]) -> String {
        let translations = self.translations.read();
        let lang_translations = translations.get(&lang).or_else(|| translations.get(&self.default_lang));

        match lang_translations {
            Some(lt) => {
                let template = lt.get(key).cloned().unwrap_or_else(|| key.to_string());
                self.format_translation(template, params, lang)
            }
            None => key.to_string(),
        }
    }

    fn format_translation(&self, template: String, params: &[TranslationParam], lang: Language) -> String {
        let re = Regex::new(r"\{(\d+)}").unwrap();
        let mut result = template;

        for cap in re.captures_iter(&result.clone()) {
            if let (Some(whole), Some(index_str)) = (cap.get(0), cap.get(1)) {
                if let Ok(index) = index_str.as_str().parse::<usize>() {
                    if index < params.len() {
                        let replacement = self.resolve_param(&params[index], lang);
                        result = result.replace(whole.as_str(), &replacement);
                    }
                }
            }
        }
        result
    }

    fn resolve_param(&self, param: &TranslationParam, lang: Language) -> String {
        match param {
            TranslationParam::String(s) => s.clone(),
            TranslationParam::Ref(tr) => {
                let params = tr.to_translation_params();
                self.get(&tr.key, lang, &params)
            }
            TranslationParam::None => String::new(),
        }
    }

    pub async fn get_guild_language(&self, db: Arc<DbManager>, id: GuildId) -> Result<Language, ()> {
        use crate::database::schema::guild_settings::dsl::*;
        let result = db.run(|conn| {
            guild_settings
                .filter(guild_id.eq(id.get() as i64))
                .select(lang)
                .first::<String>(conn)
        }).await.unwrap_or(self.default_lang.as_str().to_string());

        Language::from_str(&result)
    }


    pub async fn set_guild_language(&self, db: Arc<DbManager>, id: GuildId, new_lang: Language) -> Result<(), diesel::result::Error> {
        use crate::database::schema::guild_settings::dsl::*;

        let new_lang_str = new_lang.as_str().to_string();

        db.run(move |conn| {
            diesel::insert_into(guild_settings)
                .values(GuildSettings {
                    guild_id: id.get() as i64,
                    lang: new_lang_str.clone(),
                })
                .on_conflict(guild_id)
                .do_update()
                .set(lang.eq(new_lang_str))
                .execute(conn)
        }).await?;

        Ok(())
    }

    pub fn get_translated_lang_name(&self, lang: Language) -> String {
        let key = match lang {
            Language::English => "languages.en",
            Language::Polish => "languages.pl",
        };
        self.get(key, lang, &[])
    }
}

pub enum TranslationParam {
    String(String),
    Ref(TranslationRef),
    None,
}

impl<T: AsRef<str>> From<T> for TranslationParam {
    fn from(s: T) -> Self {
        TranslationParam::String(s.as_ref().to_string())
    }
}

impl From<TranslationRef> for TranslationParam {
    fn from(tr: TranslationRef) -> Self {
        TranslationParam::Ref(tr)
    }
}

impl TranslationParam {
    pub fn from_option(opt: Option<TranslationRef>) -> Self {
        opt.map(TranslationParam::Ref).unwrap_or(TranslationParam::None)
    }
}
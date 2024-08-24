
use std::collections::HashMap;
use std::{fs};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::str::FromStr;
use std::sync::Arc;
use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;
use poise::serenity_prelude::GuildId;
use tokio::sync::RwLock;
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
    translations: RwLock<HashMap<Language, HashMap<String, String>>>,
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

impl AsRef<TranslationParam> for TranslationParam {
    fn as_ref(&self) -> &TranslationParam {
        self
    }
}

impl LocalizationManager {
    pub async fn new(default_lang: Language) -> Result<Self, Box<dyn std::error::Error>> {
        let mut manager = LocalizationManager {
            translations: RwLock::new(HashMap::new()),
            default_lang,
        };
        manager.load_translations().await?;
        Ok(manager)
    }

    async fn load_translations(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let resources_dir = Path::new("bytemate-translations/resource");
        let mut translations = HashMap::new();

        let lang_dirs = self.find_language_dirs(resources_dir)?;

        for (lang, lang_dir) in lang_dirs {
            let lang_translations = self.load_language_translations(&lang_dir).await?;
            translations.insert(lang, lang_translations);
        }

        let mut lock = self.translations.write().await;
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

    async fn load_language_translations(&self, lang_dir: &Path) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
        let mut translations = HashMap::new();

        self.load_translations_recursive(lang_dir, &mut translations, String::new()).await?;

        Ok(translations)
    }

    fn load_translations_recursive<'a>(
        &'a self,
        dir: &'a Path,
        translations: &'a mut HashMap<String, String>,
        prefix: String,
    ) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error>>> + Send + 'a>> {
        Box::pin(async move {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    let new_prefix = if prefix.is_empty() {
                        path.file_name().unwrap().to_str().unwrap().to_string()
                    } else {
                        format!("{}.{}", prefix, path.file_name().unwrap().to_str().unwrap())
                    };
                    self.load_translations_recursive(&path, translations, new_prefix).await?;
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
        })
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

    pub fn get<'a>(
        &'a self,
        key: &'a str,
        lang: Language,
        params: &'a [TranslationParam],
    ) -> Pin<Box<dyn Future<Output = String> + Send + 'a>> {
        Box::pin(async move {
            let translations = self.translations.read().await;
            let lang_translations = translations.get(&lang).or_else(|| translations.get(&self.default_lang));

            match lang_translations {
                Some(lt) => {
                    let template = lt.get(key).cloned().unwrap_or_else(|| key.to_string());
                    self.format_translation(template, params, lang).await
                }
                None => key.to_string(),
            }
        })
    }

    fn format_translation<'a>(
        &'a self,
        template: String,
        params: &'a [TranslationParam],
        lang: Language,
    ) -> Pin<Box<dyn Future<Output = String> + Send + 'a>> {
        Box::pin(async move {
            let re = Regex::new(r"\{(\d+)}").unwrap();
            let mut result = template;
            let captures: Vec<_> = re.captures_iter(&result).collect();
            
            let mut result_clone = result.clone();
            for cap in captures {
                if let (Some(whole), Some(index_str)) = (cap.get(0), cap.get(1)) {
                    if let Ok(index) = index_str.as_str().parse::<usize>() {
                        if index < params.len() {
                            let replacement = self.resolve_param(&params[index], lang).await;
                            result_clone = result_clone.replace(whole.as_str(), &replacement);
                        } else {
                            println!("Index {} out of range for params", index);
                        }
                    }
                }
            }
            result = result_clone;
            result
        })
    }


    fn resolve_param<'a>(
        &'a self,
        param: &'a TranslationParam,
        lang: Language,
    ) -> Pin<Box<dyn Future<Output = String> + Send + 'a>> {
        Box::pin(async move {
            match param {
                TranslationParam::String(s) => s.clone(),
                TranslationParam::Ref(tr) => {
                    let params = tr.to_translation_params();
                    self.get(&tr.key, lang, &params).await
                }
                TranslationParam::None => String::new(),
            }
        })
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
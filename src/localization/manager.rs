use std::collections::HashMap;
use std::fs;
use std::path::Path;
use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;
use tokio::sync::RwLock;
use crate::database::manager::DbManager;
use crate::database::models::GuildSettings;

pub struct LocalizationManager {
    translations: RwLock<HashMap<String, HashMap<String, String>>>,
    default_lang: String,
}

impl LocalizationManager {
    pub async fn new(default_lang: String) -> Result<Self, Box<dyn std::error::Error>> {
        let mut manager = LocalizationManager {
            translations: RwLock::new(HashMap::new()),
            default_lang,
        };
        manager.load_translations().await?;
        Ok(manager)
    }

    async fn load_translations(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let resources_dir = Path::new("resources");
        let mut translations = HashMap::new();

        for entry in fs::read_dir(resources_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                let lang = path.file_name().unwrap().to_str().unwrap().to_string();
                let lang_translations = self.load_language_translations(&path).await?;
                translations.insert(lang, lang_translations);
            }
        }

        let mut lock = self.translations.write().await;
        *lock = translations;
        Ok(())
    }

    async fn load_language_translations(&self, lang_dir: &Path) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
        let mut translations = HashMap::new();

        for entry in fs::read_dir(lang_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.extension().unwrap_or_default() == "yml" {
                let content = fs::read_to_string(&path)?;
                let yaml: serde_yaml::Value = serde_yaml::from_str(&content)?;
                self.flatten_yaml(&yaml, String::new(), &mut translations);
            }
        }

        Ok(translations)
    }

    fn flatten_yaml(&self, yaml: &serde_yaml::Value, prefix: String, result: &mut HashMap<String, String>) {
        match yaml {
            serde_yaml::Value::Mapping(map) => {
                for (key, value) in map {
                    let new_prefix = if prefix.is_empty() {
                        key.as_str().unwrap().to_string()
                    } else {
                        format!("{}.{}", prefix, key.as_str().unwrap())
                    };
                    self.flatten_yaml(value, new_prefix, result);
                }
            }
            serde_yaml::Value::String(s) => {
                result.insert(prefix, s.clone());
            }
            _ => {}
        }
    }

    pub async fn get_translation(&self, key: &str, lang: &str) -> String {
        let translations = self.translations.read().await;
        let lang_translations = translations.get(lang).or_else(|| translations.get(&self.default_lang));

        match lang_translations {
            Some(lt) => lt.get(key).cloned().unwrap_or_else(|| key.to_string()),
            None => key.to_string(),
        }
    }

    pub async fn get_guild_language(&self, db: DbManager, id: i64) -> Result<String, diesel::result::Error> {
        use crate::database::schema::guild_settings::dsl::*;
        let result = db.run(|conn| {
            guild_settings
                .filter(guild_id.eq(id))
                .select(lang)
                .first::<String>(conn)
            
        }).await?;
        
        Ok(result)
    }

    pub async fn set_guild_language(&self, db: DbManager, id: i64, new_lang: &str) -> Result<(), diesel::result::Error> {
        use crate::database::schema::guild_settings::dsl::*;

        db.run(move |conn| {
            diesel::insert_into(guild_settings)
                .values(GuildSettings {
                    guild_id: id,
                    lang: new_lang.to_string(),
                })
                .on_conflict(guild_id)
                .do_update()
                .set(lang.eq(new_lang))
                .execute(conn)
        }).await?;
        
        Ok(())
    }

}
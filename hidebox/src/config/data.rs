#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct Config {
    #[serde(skip)]
    pub config_path: String,

    #[serde(skip)]
    pub db_path: String,

    #[serde(skip)]
    pub cache_dir: String,

    pub ui: UI,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UI {
    pub font_size: u32,
    pub font_family: String,
    pub win_width: u32,
    pub win_height: u32,
    pub language: String,
}

impl Default for UI {
    fn default() -> Self {
        Self {
            font_size: 18,
            font_family: "SourceHanSerifCN".to_string(),
            win_width: 600,
            win_height: 400,
            language: "cn".to_string(),
        }
    }
}

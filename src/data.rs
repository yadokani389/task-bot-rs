use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    sync::Mutex,
};

use anyhow::Error;
use chrono::{DateTime, Local, NaiveTime};
use poise::serenity_prelude as serenity;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub enum Category {
    // イベント
    Event,
    // テスト
    Exam,
    // 宿題
    Homework,
    // 持ち物
    Belongings,
    // その他
    Other,
}

impl From<Category> for String {
    fn from(category: Category) -> Self {
        match category {
            Category::Event => "イベント",
            Category::Exam => "テスト",
            Category::Homework => "宿題",
            Category::Belongings => "持ち物",
            Category::Other => "その他",
        }
        .to_string()
    }
}

impl From<String> for Category {
    fn from(category: String) -> Self {
        match category.as_str() {
            "イベント" => Category::Event,
            "テスト" => Category::Exam,
            "宿題" => Category::Homework,
            "持ち物" => Category::Belongings,
            "その他" => Category::Other,
            _ => Category::Other,
        }
    }
}

impl Category {
    pub const VALUES: [Category; 5] = [
        Category::Event,
        Category::Exam,
        Category::Homework,
        Category::Belongings,
        Category::Other,
    ];
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Task {
    pub category: Category,
    pub subject: String,
    pub details: String,
    pub datetime: DateTime<Local>,
}

impl Task {
    pub fn to_field(&self) -> (String, String, bool) {
        (
            format!(
                "【{}】{} {}",
                String::from(self.category),
                self.subject,
                self.details
            ),
            format!(
                "<t:{}:F>(<t:{}:R>)",
                self.datetime.timestamp(),
                self.datetime.timestamp()
            ),
            false,
        )
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Data {
    pub tasks: Mutex<Vec<Task>>,
    pub subjects: Mutex<BTreeSet<String>>,
    pub suggest_times: Mutex<BTreeMap<NaiveTime, String>>,
    pub panel_message: Mutex<Option<serenity::Message>>,
    pub ping_channel: Mutex<Option<serenity::ChannelId>>,
    pub ping_role: Mutex<Option<serenity::RoleId>>,
    pub log_channel: Mutex<Option<serenity::ChannelId>>,
    #[serde(skip)]
    pub panel_listener: Mutex<Option<tokio::task::JoinHandle<Result<(), Error>>>>,
}

const DATA_FILE: &str = "data.json";

pub fn save(data: &Data) -> Result<(), Error> {
    let data = serde_json::to_string(data)?;
    fs::write(DATA_FILE, data)?;
    Ok(())
}

pub fn load() -> Result<Data, Error> {
    let data = fs::read_to_string(DATA_FILE)?;
    let data = serde_json::from_str(&data)?;
    Ok(data)
}

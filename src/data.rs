use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    sync::Mutex,
};

use anyhow::{Context, Error};
use chrono::{DateTime, Local, NaiveDate, NaiveTime, TimeZone};
use poise::serenity_prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Subject {
    Value(String),
    Other,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Task {
    pub category: Category,
    pub subject: Subject,
    pub details: String,
    pub datetime: DateTime<Local>,
}

impl Task {
    pub fn to_field(&self) -> (String, String, bool) {
        (
            format!(
                "【{}】{}{}",
                String::from(self.category),
                match &self.subject {
                    Subject::Value(s) => format!("{} ", s),
                    Subject::Other => "".to_string(),
                },
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

    pub fn as_partial(&self) -> PartialTask {
        self.clone().into()
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PartialTask {
    pub category: Option<Category>,
    pub subject: Option<Subject>,
    pub details: Option<String>,
    pub date: Option<NaiveDate>,
    pub time: Option<NaiveTime>,
}

impl PartialTask {
    pub fn to_task(&self) -> Result<Task, Error> {
        let category = self.category.context("Category not selected")?;
        let subject = self.subject.clone().context("Subject not selected")?;
        let details = self.details.clone().context("Details not selected")?;
        let date = self.date.context("Date not selected")?;
        let time = self.time.context("Time not selected")?;
        let datetime = Local
            .from_local_datetime(&date.and_time(time))
            .single()
            .context("Invalid date and time")?;
        Ok(Task {
            category,
            subject,
            details,
            datetime,
        })
    }
}

impl From<Task> for PartialTask {
    fn from(task: Task) -> Self {
        Self {
            category: Some(task.category),
            subject: Some(task.subject),
            details: Some(task.details),
            date: Some(task.datetime.date_naive()),
            time: Some(task.datetime.time()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Data {
    pub tasks: Mutex<BTreeSet<Task>>,
    pub subjects: Mutex<BTreeSet<String>>,
    pub suggest_times: Mutex<BTreeMap<NaiveTime, String>>,
    pub panel_message: Mutex<Option<Message>>,
    pub ping_channel: Mutex<Option<ChannelId>>,
    pub ping_role: Mutex<Option<RoleId>>,
    pub log_channel: Mutex<Option<ChannelId>>,
    #[serde(skip)]
    pub panel_listener: Mutex<Option<tokio::task::JoinHandle<Result<(), Error>>>>,
}

pub const DATA_FILE: &str = "data.json";

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

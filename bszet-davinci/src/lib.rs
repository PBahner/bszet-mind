extern crate core;

use std::ops::Not;
use std::str::FromStr;

use anyhow::anyhow;
use once_cell::sync::Lazy;
use regex::Regex;
use reqwest::{Client, StatusCode, Url};
use select::document::Document;
use select::predicate::Name;
use time::{Date, Month, OffsetDateTime};

use crate::Change::{Addition, Cancel, Other, RoomChange};

const DATE_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new("\\S+ (\\d{2})\\.(\\d{2})\\.(\\d{4})").unwrap());

#[cfg(test)]
mod test;

pub struct Davinci {
  client: Client,
  username: String,
  password: String,
  base: Url,
  data: Option<Data>,
}

pub struct Data {
  pub last_checked: OffsetDateTime,
  pub last_modified: OffsetDateTime,
  pub tables: Vec<Table>,
}

impl Davinci {
  pub fn new(base: Url, username: String, password: String) -> Self {
    Self {
      client: Client::new(),
      username,
      password,
      base,
      data: None,
    }
  }

  pub async fn update(&mut self) -> anyhow::Result<bool> {
    let mut tables: Vec<Table> = Vec::new();

    for index in 1..50 {
      println!("{}", index);
      if let Some(mut table) = self.load_plan(index).await? {
        if let Some(last) = tables.last_mut() {
          if last.date == table.date {
            last.rows.append(&mut table.rows);
          } else {
            tables.push(table);
          }
        } else {
          tables.push(table);
        }
      } else {
        break;
      }
    }

    println!("{:?}", tables);

    let now = OffsetDateTime::now_utc();

    if let Some(data) = self.data.as_mut() {
      if tables.iter().zip(&data.tables).find(|(a, b)| a != b).is_none() {
        data.last_checked = now;
        return Ok(false);
      }
    }

    self.data = Some(Data {
      last_checked: now.clone(),
      last_modified: now,
      tables,
    });

    Ok(true)
  }

  async fn load_plan(&self, index: usize) -> anyhow::Result<Option<Table>> {
    let name = format!("V_DC_{:0>3}.html", index);

    let response = self.client.get(self.base.join(&name)?)
      .basic_auth(&self.username, Some(&self.password))
      .send()
      .await?;

    println!("{}", name);

    if response.status() == StatusCode::NOT_FOUND {
      return Ok(None);
    }

    let text = response.error_for_status()?.text().await?;
    let document = Document::from(text.as_str());

    let mut date = None;
    for node in document.find(Name("h1")) {
      if let Some(captures) = DATE_REGEX.captures(&node.text()) {
        let day = u8::from_str(captures.get(1).unwrap().as_str()).unwrap();
        let month = u8::from_str(captures.get(2).unwrap().as_str()).unwrap();
        let year = i32::from_str(captures.get(3).unwrap().as_str()).unwrap();

        date = Some(Date::from_calendar_date(year, Month::try_from(month)?, day)?);
      }
    };

    let date = if let Some(date) = date { date } else { return Err(anyhow!("Missing date in document")); };

    let table = if let Some(table) = document.find(Name("tbody")).next() {
      table
    } else {
      return Err(anyhow!("Missing time table in document"));
    };

    let mut rows = Vec::new();

    println!("{}", date);

    let mut last: Option<Row> = None;

    for row in table.find(Name("tr")) {
      let columns = row.find(Name("td"))
        .map(|data| {
          let text = data.text();
          let column = text.trim();
          if column.is_empty() { None } else { Some(column.to_string()) }
        })
        .collect::<Vec<Option<String>>>();

      if columns.len() != 7 {
        panic!("Invalid count of columns");
      }

      let class = &columns[0].as_ref();
      let lesson = columns[1].as_ref().map(|value| u8::from_str(&value[..value.len() - 1]).unwrap());
      let subject = &columns[2].as_ref();
      let room = &columns[3].as_ref();
      let teacher = &columns[4].as_ref();
      let change = columns[5].as_ref().map(|value| Change::from(value.as_ref()));
      let notice = &columns[6].as_ref();

      let row = if let Some(last) = last {
        Row {
          class: class.unwrap_or(&last.class).to_string(),
          lesson: lesson.unwrap_or(last.lesson),
          subject: subject.unwrap_or(&last.subject).to_string(),
          room: room.unwrap_or(&last.room).to_string(),
          teacher: teacher.unwrap_or(&last.teacher).to_string(),
          change: change.unwrap_or(last.change),
          notice: notice.map(|value| value.to_string()).or(last.notice),
        }
      } else {
        Row {
          class: class.expect("First row, can not have missing fields.").to_string(),
          lesson: lesson.expect("First row, can not have missing fields."),
          subject: subject.expect("First row, can not have missing fields.").to_string(),
          room: room.expect("First row, can not have missing fields.").to_string(),
          teacher: teacher.expect("First row, can not have missing fields.").to_string(),
          change: change.expect("First row, can not have missing fields."),
          notice: notice.map(|value| value.to_string()),
        }
      };

      rows.push(row.clone());
      last = Some(row);
    }

    Ok(Some(Table {
      date,
      rows,
    }))
  }
}

#[derive(Debug, PartialEq)]
pub struct Table {
  pub date: Date,
  pub rows: Vec<Row>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Row {
  pub class: String,
  pub lesson: u8,
  pub subject: String,
  pub room: String,
  pub teacher: String,
  pub change: Change,
  pub notice: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Change {
  Cancel,
  RoomChange,
  Addition,
  Other(String),
}

impl From<&str> for Change {
  fn from(value: &str) -> Self {
    match value {
      "Fällt aus" => Cancel,
      "Raumänderung" => RoomChange,
      "Zusatzunterricht" => Addition,
      value => Other(value.to_string()),
    }
  }
}
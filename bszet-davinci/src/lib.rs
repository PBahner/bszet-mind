extern crate core;

use std::str::FromStr;

use anyhow::anyhow;
use once_cell::sync::Lazy;
use regex::Regex;
use reqwest::{Client, Url};
use reqwest::header::LAST_MODIFIED;
use select::document::Document;
use select::predicate::Name;
use time::{Date, Month, OffsetDateTime};
use time::format_description::well_known::Rfc2822;

use crate::Change::{Addition, Cancel, Other, PlaceChange};
use crate::timetable::{Lesson, Subject};
use crate::timetable::igd21::IGD21;

const OVERVIEW_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new("V_DC_(\\d{3}).html").unwrap());
const DATE_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new("\\S+ (\\d{2})\\.(\\d{2})\\.(\\d{4})").unwrap());

#[cfg(test)]
mod test;
pub mod timetable;

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


    // fetch all pages until a page returns a 404
    for index in 1..50 {
      let mut table = self.load_plan(index).await?;
      if let Some(last) = tables.last_mut() {
        if last.date == table.date {
          last.rows.append(&mut table.rows);
        } else {
          tables.push(table);
        }
      } else {
        tables.push(table);
      }
    }

    let now = OffsetDateTime::now_utc();

    // check if there is a difference
    if let Some(data) = self.data.as_mut() {
      if !tables.iter().zip(&data.tables).any(|(a, b)| a != b) {
        data.last_checked = now;
        return Ok(false);
      }
    }

    for x in &mut tables {
      x.rows.push(Row {
        class: vec!["IGD21".to_string()],
        lesson: 1,
        subject: Subject::GermanBasic,
        place: "111".to_string(),
        teacher: "wefwe".to_string(),
        change: Change::Cancel,
        notice: Some("Hallo".to_string()),
      });
    }

    self.data = Some(Data {
      last_checked: now,
      last_modified: now,
      tables,
    });

    Ok(true)
  }

  pub fn data(&self) -> Option<&Data> {
    self.data.as_ref()
  }

  pub fn apply_changes(&self, date: Date) -> Vec<Lesson> {
    let mut day = IGD21.get(&date.weekday()).unwrap().clone();


    if let Some(data) = &self.data {
      for table in &data.tables {
        println!("{:?}", table);

        if table.date != date {
          continue;
        }

        for row in &table.rows {
          println!("{:?}", row);
          if row.class.contains(&"IGD21".to_string()) {
            for mut lesson in &mut day {
              println!("{:?}", row);

              if lesson.lesson == row.lesson {
                // match &row.change {
                //   Cancel => {
                //     lesson.subject = Subject::Cancel(Box::new(lesson.subject.clone()));
                //   }
                //   PlaceChange => {
                //     lesson.place = row.place.clone();
                //   }
                //   Addition => {
                //     day.push(Lesson {
                //       lesson: row.lesson,
                //       subject: row.subject.clone(),
                //       iteration: None,
                //       place: row.place.clone(),
                //       notice: row.notice.clone(),
                //     });
                //   }
                //   Other(other) => {
                //     lesson.notice = Some(match &row.notice {
                //       None => other.to_string(),
                //       Some(notice) => format!("{} - {}", other, notice),
                //     });
                //   }
                // }
                break;
              }
            }
          }
        }

        break;
      }
    }

    day
  }

  async fn load_overview(&self) -> anyhow::Result<(OffsetDateTime, Vec<Table>)> {
    let response = self.client.get(&self.base)
      .basic_auth(&self.username, Some(&self.password))
      .send()
      .await?
      .error_for_status()?;

    let text = response.text().await?;

    let mut tables = Vec::new();

    if let Some(capture) = OVERVIEW_REGEX.captures_iter(&text).last() {
      let last = u16::from_str(capture.get(1).unwrap().as_str()).unwrap();

      for i in 1..=last {
        let mut table = self.load_plan(i).await?;

        match tables.last_mut() {
          Some(last) => {
            if last.date == table.date {
              last.rows.append(&mut table.rows);
            } else {
              tables.push(table);
            }
          }
          None => tables.push(table)
        }
      }
    }

    match response.headers().get(LAST_MODIFIED) {
      None => Err(anyhow!("last-modified http header is required")),
      Some(value) => Ok((OffsetDateTime::parse(value.to_str()?, &Rfc2822)?, tables))
    }
  }

  async fn load_plan(&self, index: u16) -> anyhow::Result<Table> {
    let name = format!("V_DC_{:<3}.html", index);

    let response = self.client.get(self.base.join(&name)?)
      .basic_auth(&self.username, Some(&self.password))
      .send()
      .await?
      .error_for_status()?;

    let text = response.text().await?;
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

      let class = columns[0].as_ref().map(|value| value.split(',').map(|value| value.trim().to_string()).collect::<Vec<String>>());
      let lesson = columns[1].as_ref().map(|value| u8::from_str(&value[..value.len() - 1]).unwrap());
      let subject = columns[2].as_ref().map(|value| Subject::from(value.as_ref()));
      let place = &columns[3].as_ref();
      let teacher = &columns[4].as_ref();
      let change = columns[5].as_ref().map(|value| Change::from(value.as_ref()));
      let notice = &columns[6].as_ref();

      let row = if let Some(last) = last {
        Row {
          class: class.unwrap_or(last.class.clone()),
          lesson: lesson.unwrap_or(last.lesson),
          subject: subject.unwrap_or(last.subject),
          place: place.unwrap_or(&last.place).to_string(),
          teacher: teacher.unwrap_or(&last.teacher).to_string(),
          change: change.unwrap_or(last.change),
          notice: notice.map(|value| value.to_string()).or(last.notice),
        }
      } else {
        Row {
          class: class.expect("First row, can not have missing fields."),
          lesson: lesson.expect("First row, can not have missing fields."),
          subject: subject.expect("First row, can not have missing fields."),
          place: place.expect("First row, can not have missing fields.").to_string(),
          teacher: teacher.expect("First row, can not have missing fields.").to_string(),
          change: change.expect("First row, can not have missing fields."),
          notice: notice.map(|value| value.to_string()),
        }
      };

      rows.push(row.clone());
      last = Some(row);
    }

    Ok(Table {
      date,
      rows,
    })
  }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Table {
  pub date: Date,
  pub rows: Vec<Row>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
  pub class: Vec<String>,
  pub lesson: u8,
  pub subject: Subject,
  pub teacher: String,
  pub change: Change,
  pub notice: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Change {
  Cancel,
  PlaceChange,
  Addition,
  Other(String),
}

impl From<&str> for Change {
  fn from(value: &str) -> Self {
    match value {
      "Fällt aus" => Cancel,
      "Raumänderung" => PlaceChange,
      "Zusatzunterricht" => Addition,
      value => Other(value.to_string()),
    }
  }
}

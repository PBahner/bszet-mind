extern crate core;

use std::ops::Sub;
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
use tokio::io::AsyncBufReadExt;

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
  entrypoint: Url,
  data: Option<Data>,
}

pub struct Data {
  pub last_checked: OffsetDateTime,
  pub last_modified: OffsetDateTime,
  pub rows: Vec<Row>,
}

impl Davinci {
  pub fn new(entrypoint: Url, username: String, password: String) -> Self {
    Self {
      client: Client::new(),
      username,
      password,
      entrypoint,
      data: None,
    }
  }
  pub fn data(&self) -> Option<&Data> {
    self.data.as_ref()
  }

  pub fn apply_changes(&self, date: Date) -> Vec<Lesson> {
    let mut day = IGD21.get(&date.weekday()).unwrap().clone();


    if let Some(data) = &self.data {
      for row in &data.rows {
        if row.date != date {
          continue;
        }

        if row.class.contains(&"IGD21".to_string()) {
          for mut lesson in &mut day {
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

        break;
      }
    }

    day
  }

  pub async fn update(&mut self) -> anyhow::Result<bool> {
    let mut url = self.entrypoint.clone();
    let mut rows = Vec::new();

    loop {
      match self.fetch(url, &mut rows).await? {
        None => break,
        Some(next) => url = next,
      };
    }

    let now = OffsetDateTime::now_utc();

    // check if there is a difference
    if let Some(data) = self.data.as_mut() {
      if !rows.iter().zip(&data.rows).any(|(a, b)| a != b) {
        data.last_checked = now;
        return Ok(false);
      }
    }

    self.data = Some(Data {
      last_checked: now,
      last_modified: now,
      rows,
    });

    Ok(true)
  }

  async fn fetch(&self, url: Url, rows: &mut Vec<Row>) -> anyhow::Result<Option<Url>> {
    let response = self.client.get(url.clone())
      .basic_auth(&self.username, Some(&self.password))
      .send()
      .await?
      .error_for_status()?;

    let last_modified = match response.headers().get(LAST_MODIFIED) {
      None => return Err(anyhow!("last-modified http header is required")),
      Some(value) => OffsetDateTime::parse(value.to_str()?, &Rfc2822)?
    };
    println!("{} {}", url, last_modified);

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

    for row in table.find(Name("tr")) {
      let columns = row.find(Name("td"))
        .map(|data| {
          let text = data.text();
          let column = text.trim();
          column.to_string()
        })
        .collect::<Vec<String>>();

      if columns.len() != 7 {
        panic!("Invalid count of columns");
      }

      let class = if columns[0].is_empty() { None } else { Some(columns[0].split(',').map(|value| value.trim().to_string()).collect::<Vec<String>>()) };
      let lesson = if columns[1].is_empty() { None } else { Some(u8::from_str(&columns[1][..columns[1].len() - 1]).unwrap()) };
      let change = Change::new(&columns[5], Subject::from(columns[2].as_str()), columns[3].to_string(), columns[4].to_string());
      let notice = if columns[6].is_empty() { None } else { Some(columns[6].to_string()) };

      let row = if let Some(last) = rows.last() {
        Row {
          date,
          class: class.unwrap_or(last.class.clone()),
          lesson: lesson.unwrap_or(last.lesson),
          change,
          notice,
        }
      } else {
        Row {
          date,
          class: class.expect("First row, can not have missing fields."),
          lesson: lesson.expect("First row, can not have missing fields."),
          change,
          notice,
        }
      };

      rows.push(row);
    }

    if let Some(js) = document.find(Name("input"))
      .filter_map(|input| input.attr("onclick"))
      .last() {
      let next = self.entrypoint.join(&js[22..js.len() - 1])?;
      if next != url {
        return Ok(Some(next));
      }
    }

    Ok(None)
  }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
  pub date: Date,
  pub class: Vec<String>,
  pub lesson: u8,
  pub change: Change,
  pub notice: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Change {
  Cancel {
    subject: Subject,
    teacher: String,
    place: String,
  },
  PlaceChange {
    subject: Subject,
    teacher: String,
    place: String,
  },
  Addition {
    subject: Subject,
    teacher: String,
    place: String,
  },
  Other {
    value: String,
    subject: Subject,
    teacher: String,
    place: String,
  },
}

impl Change {
  fn new(value: &str, subject: Subject, place: String, teacher: String) -> Self {
    match value {
      "Fällt aus" => Self::Cancel { subject, place, teacher },
      "Raumänderung" => Self::PlaceChange { subject, place, teacher },
      "Zusatzunterricht" => Self::Addition { subject, place, teacher },
      value => Self::Other { value: value.to_string(), subject, place, teacher }
    }
  }
}

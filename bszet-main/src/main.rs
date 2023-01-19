mod ascii;

use std::string::ToString;

use time::{Date, OffsetDateTime};
use time::Month::January;

use bszet_davinci::Davinci;
use bszet_notify::telegram::Telegram;
use crate::ascii::table;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  let mut davinci = Davinci::new(
    "https://geschuetzt.bszet.de/s-lk-vw/Vertretungsplaene/V_PlanBGy/V_DC_001.html".parse()?,
    "".to_string(),
    "".to_string(),
  );

  davinci.update().await?;
  let table = table(davinci.apply_changes(Date::from_calendar_date(2023, January, 19)?));

  let telegram = Telegram::new("")?;

  match davinci.data() {
    None => telegram.send(-734603836, "Es konnte kein Vertretungsplan geladen werden.".to_string()).await?,
    Some(data) => {
      let age = OffsetDateTime::now_utc() - data.last_checked;
      let text = format!("Der Vertretungsplan wurde zuletzt vor {} aktualisiert.\n```\n{}```", age, table);
      telegram.send(-734603836, text).await?;
    }
  }

  Ok(())
}

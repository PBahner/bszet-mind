use std::string::ToString;
use std::time::Duration;

use clap::Parser;
use reqwest::Url;
use time::{Date, OffsetDateTime};
use time::Month::January;

use bszet_davinci::Davinci;
use bszet_notify::telegram::Telegram;
use tokio_cron_scheduler::JobScheduler;

use crate::ascii::table;

mod ascii;

#[derive(Parser)]
#[command(author, version, about, long_about)]
struct Args {
  #[arg(long, short, env = "BSZET_MIND_ENTRYPOINT", default_value = "https://geschuetzt.bszet.de/s-lk-vw/Vertretungsplaene/V_PlanBGy/V_DC_001.html")]
  entrypoint: Url,
  #[arg(long, short, env = "BSZET_MIND_USERNAME")]
  username: String,
  #[arg(long, short, env = "BSZET_MIND_PASSWORD")]
  password: String,
  #[arg(long, short, env = "BSZET_MIND_TELEGRAM_TOKEN")]
  telegram_token: String,
  #[arg(long, short, env = "BSZET_MIND_CHAT_IDS")]
  chat_ids: Vec<i32>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  let args = Args::parse();

  let mut davinci = Davinci::new(
    args.entrypoint,
    args.username,
    args.password,
  );

  loop {
    if !davinci.update().await? {
      println!("keine updates");
      tokio::time::sleep(Duration::from_secs(5)).await;
      continue;
    }

    println!("updates, telegram");

    let table = table(davinci.apply_changes(Date::from_calendar_date(2023, January, 20)?));

    let telegram = Telegram::new(&args.telegram_token)?;

    for id in &args.chat_ids {
      match davinci.data() {
        None => telegram.send(*id, "Es konnte kein Vertretungsplan geladen werden.".to_string()).await?,
        Some(data) => {
          let age = OffsetDateTime::now_utc() - data.last_checked;
          let text = format!("Der Vertretungsplan wurde zuletzt vor {} aktualisiert.\n```\n{}```", age, table);
          telegram.send(*id, text).await?;
        }
      }
    }
    tokio::time::sleep(Duration::from_secs(5)).await;
  }

  Ok(())
}

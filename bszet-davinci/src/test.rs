use crate::Davinci;

#[tokio::test]
async fn test_load() -> anyhow::Result<()> {
  let mut davinci = Davinci::new(
    "https://geschuetzt.bszet.de/s-lk-vw/Vertretungsplaene/V_PlanBGy/".parse().unwrap(),
    "".to_string(),
    "".to_string(),
  );

  assert_eq!(true, davinci.update().await?);
  assert_eq!(false, davinci.update().await?);

  for table in &davinci.data.unwrap().tables {
    println!("{}:", table.date);

    for row in &table.rows {
      println!("- {:?}", row);
    }
  }

  Ok(())
}

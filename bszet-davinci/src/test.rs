use crate::Davinci;

#[tokio::test]
async fn test_load() {
  let mut davinci = Davinci::new(
    "https://geschuetzt.bszet.de/s-lk-vw/Vertretungsplaene/V_PlanBGy".parse().unwrap(),
    "".to_string(),
    "".to_string(),
  );

  println!("{}", davinci.update().await.unwrap());

  println!("{}", &davinci.data.as_ref().unwrap().last_checked);

  for table in &davinci.data.unwrap().tables {
    println!("{}:", table.date);

    for row in &table.rows {
      println!("- {:?}", row);
    }
  }
}
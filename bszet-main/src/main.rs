use bszet_davinci::load;

#[tokio::main]
async fn main() {
  let vec = load().await.unwrap();

  for table in vec {
    println!("{}:", table.date);

    for row in table.rows {
      println!("- {:?}", row);
    }
  }
}

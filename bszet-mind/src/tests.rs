use std::time::Duration;

use crate::format_duration;

#[test]
fn test_format_duration() {
  assert_eq!(
    "einer Stunde und 2 Minuten",
    format_duration(Duration::from_secs(60 * 60 + 60 * 2))
  );
  assert_eq!(
    "einer Stunde",
    format_duration(Duration::from_secs(60 * 60))
  );
}

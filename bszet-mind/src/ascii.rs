use std::fmt::Write;

use bszet_davinci::timetable::Lesson;

pub fn table(day: Vec<Lesson>) -> String {
  let mut lesson_w = 0;
  let mut subject_w = 0;
  let mut place_w = 0;

  for lesson in &day {
    let l = format!("{}", lesson.lesson);
    let s = format!("{}", lesson.subject);
    let p = &lesson.place;

    lesson_w = lesson_w.max(l.chars().count());
    subject_w = subject_w.max(s.chars().count());
    place_w = place_w.max(p.as_ref().map(|s| s.chars().count()).unwrap_or(0));
  }

  // only works with ascii characters, with utf like ü, ä, ö, ß, ...
  // there will be an additional allocation
  let mut out = String::with_capacity(day.len() * (lesson_w + subject_w + place_w + 2));

  let mut first = true;

  for lesson in &day {
    let l = format!("{}", lesson.lesson);
    let s = format!("{}", lesson.subject);
    let p = &lesson.place;

    if first {
      first = false;
    } else {
      writeln!(out).unwrap();
    }

    if let Some(notice) = &lesson.notice {
      write!(
        out,
        "{}{} {}{} {}{} {}",
        l,
        " ".repeat(lesson_w - l.chars().count()),
        s,
        " ".repeat(subject_w - s.chars().count()),
        p.as_ref().unwrap_or(&"".to_string()),
        " ".repeat(place_w - p.as_ref().map(|s| s.chars().count()).unwrap_or(0)),
        notice
      )
      .unwrap();
    } else {
      write!(
        out,
        "{}{} {}{} {}",
        l,
        " ".repeat(lesson_w - l.chars().count()),
        s,
        " ".repeat(subject_w - s.chars().count()),
        p.as_ref().unwrap_or(&"".to_string())
      )
      .unwrap();
    }
  }

  out
}

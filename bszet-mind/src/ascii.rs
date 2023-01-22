use std::fmt::Write;

use bszet_davinci::timetable::Lesson;

pub fn table(day: Vec<Lesson>) -> String {
  let mut lessonW = 0;
  let mut subjectW = 0;
  let mut placeW = 0;

  for lesson in &day {
    let l = format!("{}", lesson.lesson);
    let s = format!("{}", lesson.subject);
    let p = format!("{}", lesson.place);

    lessonW = lessonW.max(l.len());
    subjectW = subjectW.max(s.len());
    placeW = placeW.max(p.len());
  }

  let mut out = String::with_capacity(day.len() * (lessonW + subjectW + placeW + 2));

  let mut first = true;

  for lesson in &day {
    let l = format!("{}", lesson.lesson);
    let s = format!("{}", lesson.subject);
    let p = format!("{}", lesson.place);

    if first {
      first = false;
    } else {
      writeln!(out).unwrap();
    }

    if let Some(notice) = &lesson.notice {
      write!(out, "{}{} {}{} {}{} {}", l, " ".repeat(lessonW - l.len()), s, " ".repeat(subjectW - s.len()), p, " ".repeat(placeW - p.len()), notice).unwrap();
    } else {
      write!(out, "{}{} {}{} {}", l, " ".repeat(lessonW - l.len()), s, " ".repeat(subjectW - s.len()), p).unwrap();
    }
  }

  out
}

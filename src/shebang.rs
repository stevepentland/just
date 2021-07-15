#[derive(Copy, Clone)]
pub(crate) struct Shebang<'line> {
  pub(crate) interpreter: &'line str,
  pub(crate) argument:    Option<&'line str>,
}

impl<'line> Shebang<'line> {
  pub(crate) fn new(line: &'line str) -> Option<Shebang<'line>> {
    if !line.starts_with("#!") {
      return None;
    }

    let mut pieces = line[2..]
      .lines()
      .next()
      .unwrap_or("")
      .trim()
      .splitn(2, |c| c == ' ' || c == '\t');

    let interpreter = pieces.next().unwrap_or("");
    let argument = pieces.next();

    if interpreter.is_empty() {
      return None;
    }

    Some(Shebang {
      interpreter,
      argument,
    })
  }

  fn interpreter_filename(&self) -> &str {
    self
      .interpreter
      .rsplit_once(|c| matches!(c, '/' | '\\'))
      .map(|(_path, filename)| filename)
      .unwrap_or(self.interpreter)
  }

  pub(crate) fn script_filename(&self, recipe: &str) -> String {
    match self.interpreter_filename() {
      "cmd" | "cmd.exe" => format!("{}.bat", recipe),
      "powershell" | "powershell.exe" => format!("{}.ps1", recipe),
      _ => recipe.to_owned(),
    }
  }

  pub(crate) fn include_shebang_line(&self) -> bool {
    !matches!(self.interpreter_filename(), "cmd" | "cmd.exe")
  }
}

#[cfg(test)]
mod tests {
  use super::Shebang;

  #[test]
  fn split_shebang() {
    fn check(text: &str, expected_split: Option<(&str, Option<&str>)>) {
      let shebang = Shebang::new(text);
      assert_eq!(
        shebang.map(|shebang| (shebang.interpreter, shebang.argument)),
        expected_split
      );
    }

    check("#!    ", None);
    check("#!", None);
    check("#!/bin/bash", Some(("/bin/bash", None)));
    check("#!/bin/bash    ", Some(("/bin/bash", None)));
    check(
      "#!/usr/bin/env python",
      Some(("/usr/bin/env", Some("python"))),
    );
    check(
      "#!/usr/bin/env python   ",
      Some(("/usr/bin/env", Some("python"))),
    );
    check(
      "#!/usr/bin/env python -x",
      Some(("/usr/bin/env", Some("python -x"))),
    );
    check(
      "#!/usr/bin/env python   -x",
      Some(("/usr/bin/env", Some("python   -x"))),
    );
    check(
      "#!/usr/bin/env python \t-x\t",
      Some(("/usr/bin/env", Some("python \t-x"))),
    );
    check("#/usr/bin/env python \t-x\t", None);
    check("#!  /bin/bash", Some(("/bin/bash", None)));
    check("#!\t\t/bin/bash    ", Some(("/bin/bash", None)));
    check(
      "#!  \t\t/usr/bin/env python",
      Some(("/usr/bin/env", Some("python"))),
    );
    check(
      "#!  /usr/bin/env python   ",
      Some(("/usr/bin/env", Some("python"))),
    );
    check(
      "#!  /usr/bin/env python -x",
      Some(("/usr/bin/env", Some("python -x"))),
    );
    check(
      "#!  /usr/bin/env python   -x",
      Some(("/usr/bin/env", Some("python   -x"))),
    );
    check(
      "#!  /usr/bin/env python \t-x\t",
      Some(("/usr/bin/env", Some("python \t-x"))),
    );
    check("#  /usr/bin/env python \t-x\t", None);
  }

  #[test]
  fn interpreter_filename_with_forward_slash() {
    assert_eq!(
      Shebang::new("#!/foo/bar/baz")
        .unwrap()
        .interpreter_filename(),
      "baz"
    );
  }

  #[test]
  fn interpreter_filename_with_backslash() {
    assert_eq!(
      Shebang::new("#!\\foo\\bar\\baz")
        .unwrap()
        .interpreter_filename(),
      "baz"
    );
  }

  #[test]
  fn powershell_script_filename() {
    assert_eq!(
      Shebang::new("#!powershell").unwrap().script_filename("foo"),
      "foo.ps1"
    );
  }

  #[test]
  fn powershell_exe_script_filename() {
    assert_eq!(
      Shebang::new("#!powershell.exe")
        .unwrap()
        .script_filename("foo"),
      "foo.ps1"
    );
  }

  #[test]
  fn cmd_script_filename() {
    assert_eq!(
      Shebang::new("#!cmd").unwrap().script_filename("foo"),
      "foo.bat"
    );
  }

  #[test]
  fn cmd_exe_script_filename() {
    assert_eq!(
      Shebang::new("#!cmd.exe").unwrap().script_filename("foo"),
      "foo.bat"
    );
  }

  #[test]
  fn plain_script_filename() {
    assert_eq!(Shebang::new("#!bar").unwrap().script_filename("foo"), "foo");
  }

  #[test]
  fn dont_include_shebang_line_cmd() {
    assert!(!Shebang::new("#!cmd").unwrap().include_shebang_line());
  }

  #[test]
  fn dont_include_shebang_line_cmd_exe() {
    assert!(!Shebang::new("#!cmd.exe /C").unwrap().include_shebang_line());
  }

  #[test]
  fn include_shebang_line_other() {
    assert!(Shebang::new("#!foo -c").unwrap().include_shebang_line());
  }
}

use crate::common::*;

#[test]
fn dotenv() {
  let tmp = temptree! {
    ".env": "KEY=ROOT",
    sub: {
      ".env": "KEY=SUB",
      justfile: "default:\n\techo KEY=$KEY",
    },
  };

  let binary = executable_path("just");

  let output = Command::new(binary)
    .current_dir(tmp.path())
    .arg("sub/default")
    .output()
    .expect("just invocation failed");

  assert_eq!(output.status.code().unwrap(), 0);

  let stdout = str::from_utf8(&output.stdout).unwrap();
  assert_eq!(stdout, "KEY=SUB\n");
}

test! {
  name:     set_false,
  justfile: r#"
    set dotenv-load := false

    foo:
      if [ -n "${DOTENV_KEY+1}" ]; then echo defined; else echo undefined; fi
  "#,
  stdout:   "undefined\n",
  stderr:   "if [ -n \"${DOTENV_KEY+1}\" ]; then echo defined; else echo undefined; fi\n",
}

test! {
  name:     set_implicit,
  justfile: r#"
    set dotenv-load

    foo:
      echo $DOTENV_KEY
  "#,
  stdout:   "dotenv-value\n",
  stderr:   "echo $DOTENV_KEY\n",
}

test! {
  name:     set_true,
  justfile: r#"
    set dotenv-load := true

    foo:
      echo $DOTENV_KEY
  "#,
  stdout:   "dotenv-value\n",
  stderr:   "echo $DOTENV_KEY\n",
}

// Un-comment this on 2021-07-01.
//
// test! {
//   name:     warning,
//   justfile: r#"
//     foo:
//       echo $DOTENV_KEY
//   "#,
//   stdout:   "dotenv-value\n",
//   stderr:   "
//     warning: A `.env` file was found and loaded, but this behavior will
// change in the future.     To silence this warning and continue loading `.env`
// files, add:

//         set dotenv-load := true

//     To silence this warning and stop loading `.env` files, add:

//         set dotenv-load := false

//     See https://github.com/casey/just/issues/469 for more details.
//     echo $DOTENV_KEY
//   ",
// }

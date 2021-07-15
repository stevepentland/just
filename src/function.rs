use crate::common::*;

use Function::*;
pub(crate) enum Function {
  Nullary(fn(&FunctionContext) -> Result<String, String>),
  Unary(fn(&FunctionContext, &str) -> Result<String, String>),
  Binary(fn(&FunctionContext, &str, &str) -> Result<String, String>),
  Ternary(fn(&FunctionContext, &str, &str, &str) -> Result<String, String>),
}

lazy_static! {
  pub(crate) static ref TABLE: BTreeMap<&'static str, Function> = vec![
    ("arch", Nullary(arch)),
    ("clean", Unary(clean)),
    ("env_var", Unary(env_var)),
    ("env_var_or_default", Binary(env_var_or_default)),
    ("extension", Unary(extension)),
    ("file_name", Unary(file_name)),
    ("file_stem", Unary(file_stem)),
    ("invocation_directory", Nullary(invocation_directory)),
    ("join", Binary(join)),
    ("just_executable", Nullary(just_executable)),
    ("justfile", Nullary(justfile)),
    ("justfile_directory", Nullary(justfile_directory)),
    ("lowercase", Unary(lowercase)),
    ("os", Nullary(os)),
    ("os_family", Nullary(os_family)),
    ("parent_directory", Unary(parent_directory)),
    ("replace", Ternary(replace)),
    ("trim", Unary(trim)),
    ("uppercase", Unary(uppercase)),
    ("without_extension", Unary(without_extension)),
  ]
  .into_iter()
  .collect();
}

impl Function {
  pub(crate) fn argc(&self) -> usize {
    match *self {
      Nullary(_) => 0,
      Unary(_) => 1,
      Binary(_) => 2,
      Ternary(_) => 3,
    }
  }
}

fn arch(_context: &FunctionContext) -> Result<String, String> {
  Ok(target::arch().to_owned())
}

fn clean(_context: &FunctionContext, path: &str) -> Result<String, String> {
  Ok(Path::new(path).lexiclean().to_str().unwrap().to_owned())
}

fn env_var(context: &FunctionContext, key: &str) -> Result<String, String> {
  use std::env::VarError::*;

  if let Some(value) = context.dotenv.get(key) {
    return Ok(value.clone());
  }

  match env::var(key) {
    Err(NotPresent) => Err(format!("environment variable `{}` not present", key)),
    Err(NotUnicode(os_string)) => Err(format!(
      "environment variable `{}` not unicode: {:?}",
      key, os_string
    )),
    Ok(value) => Ok(value),
  }
}

fn env_var_or_default(
  context: &FunctionContext,
  key: &str,
  default: &str,
) -> Result<String, String> {
  use std::env::VarError::*;

  if let Some(value) = context.dotenv.get(key) {
    return Ok(value.clone());
  }

  match env::var(key) {
    Err(NotPresent) => Ok(default.to_owned()),
    Err(NotUnicode(os_string)) => Err(format!(
      "environment variable `{}` not unicode: {:?}",
      key, os_string
    )),
    Ok(value) => Ok(value),
  }
}

fn extension(_context: &FunctionContext, path: &str) -> Result<String, String> {
  Utf8Path::new(path)
    .extension()
    .map(str::to_owned)
    .ok_or_else(|| format!("Could not extract extension from `{}`", path))
}

fn file_name(_context: &FunctionContext, path: &str) -> Result<String, String> {
  Utf8Path::new(path)
    .file_name()
    .map(str::to_owned)
    .ok_or_else(|| format!("Could not extract file name from `{}`", path))
}

fn file_stem(_context: &FunctionContext, path: &str) -> Result<String, String> {
  Utf8Path::new(path)
    .file_stem()
    .map(str::to_owned)
    .ok_or_else(|| format!("Could not extract file stem from `{}`", path))
}

fn invocation_directory(context: &FunctionContext) -> Result<String, String> {
  Platform::convert_native_path(
    &context.search.working_directory,
    context.invocation_directory,
  )
  .map_err(|e| format!("Error getting shell path: {}", e))
}

fn join(_context: &FunctionContext, base: &str, with: &str) -> Result<String, String> {
  Ok(Utf8Path::new(base).join(with).to_string())
}

fn just_executable(_context: &FunctionContext) -> Result<String, String> {
  let exe_path =
    std::env::current_exe().map_err(|e| format!("Error getting current executable: {}", e))?;

  exe_path.to_str().map(str::to_owned).ok_or_else(|| {
    format!(
      "Executable path is not valid unicode: {}",
      exe_path.to_string_lossy()
    )
  })
}

fn justfile(context: &FunctionContext) -> Result<String, String> {
  context
    .search
    .justfile
    .to_str()
    .map(str::to_owned)
    .ok_or_else(|| {
      format!(
        "Justfile path is not valid unicode: {}",
        context.search.justfile.to_string_lossy()
      )
    })
}

fn justfile_directory(context: &FunctionContext) -> Result<String, String> {
  let justfile_directory = context.search.justfile.parent().ok_or_else(|| {
    format!(
      "Could not resolve justfile directory. Justfile `{}` had no parent.",
      context.search.justfile.display()
    )
  })?;

  justfile_directory
    .to_str()
    .map(str::to_owned)
    .ok_or_else(|| {
      format!(
        "Justfile directory is not valid unicode: {}",
        justfile_directory.to_string_lossy()
      )
    })
}

fn lowercase(_context: &FunctionContext, s: &str) -> Result<String, String> {
  Ok(s.to_lowercase())
}

fn os(_context: &FunctionContext) -> Result<String, String> {
  Ok(target::os().to_owned())
}

fn os_family(_context: &FunctionContext) -> Result<String, String> {
  Ok(target::os_family().to_owned())
}

fn parent_directory(_context: &FunctionContext, path: &str) -> Result<String, String> {
  Utf8Path::new(path)
    .parent()
    .map(Utf8Path::to_string)
    .ok_or_else(|| format!("Could not extract parent directory from `{}`", path))
}

fn replace(_context: &FunctionContext, s: &str, from: &str, to: &str) -> Result<String, String> {
  Ok(s.replace(from, to))
}

fn trim(_context: &FunctionContext, s: &str) -> Result<String, String> {
  Ok(s.trim().to_owned())
}

fn uppercase(_context: &FunctionContext, s: &str) -> Result<String, String> {
  Ok(s.to_uppercase())
}

fn without_extension(_context: &FunctionContext, path: &str) -> Result<String, String> {
  let parent = Utf8Path::new(path)
    .parent()
    .ok_or_else(|| format!("Could not extract parent from `{}`", path))?;

  let file_stem = Utf8Path::new(path)
    .file_stem()
    .ok_or_else(|| format!("Could not extract file stem from `{}`", path))?;

  Ok(parent.join(file_stem).to_string())
}

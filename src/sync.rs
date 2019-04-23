#![allow(dead_code)]

use std::cell::Ref;
use std::collections::HashMap;
use std::io::{self, Write};
use std::path::PathBuf;

use tempfile::{NamedTempFile, TempDir};
use unindent::unindent;

use crate::lockfiles::{Dependency, Lock, Marker, PythonPackage};
use crate::projects::{self, Project};
use crate::pythons::{self, Interpreter};
use crate::vendors;

enum Error {
    DefaultSectionNotFound,
    ExtraSectionNotFound(String),
    InstallCommandError(Vec<(String, Option<i32>)>),
    InterpreterError(pythons::Error),
    InvalidMarkerError(String, String),
    PathRepresentationError(PathBuf),
    ProjectError(projects::Error),
    SystemError(io::Error),
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::SystemError(e)
    }
}

impl From<projects::Error> for Error {
    fn from(e: projects::Error) -> Self {
        Error::ProjectError(e)
    }
}

impl From<pythons::Error> for Error {
    fn from(e: pythons::Error) -> Self {
        Error::InterpreterError(e)
    }
}

type Result<T> = std::result::Result<T, Error>;

struct Synchronizer {
    packaging: TempDir,
    lock: Lock,
}

impl Synchronizer {
    fn new(lock: Lock) -> Result<Self> {
        let tmp_dir = TempDir::new()?;
        vendors::Packaging::populate_to(tmp_dir.path())?;
        Ok(Self { packaging: tmp_dir, lock })
    }

    fn evaluate_marker(&self, m: &Marker, int: &Interpreter) -> Result<bool> {
        let marker = m.iter()
            .map(|s| format!("({})", s))
            .collect::<Vec<_>>()
            .join(" or ");

        // any([]) is always false. Note that this is different from a null
        // marker, which evaluates to true.
        if marker.is_empty() {
            return Ok(false);
        }

        let code = unindent(&format!(
            r#"
            from __future__ import print_function
            import sys
            from packaging.markers import InvalidMarker, Marker
            try:
                m = Marker({:?})
            except InvalidMarker as e:
                print(e, file=sys.stderr, end='')
            else:
                print(bool(m.evaluate()), end='')
            "#,
            marker,
        ));

        let output = int.command(Some("utf-8"), self.packaging.path())?
            .arg("-c")
            .arg(&code)
            .output()?;

        // TODO: Show error if out.status() is not OK.

        let s = String::from_utf8(output.stdout).unwrap();
        if s == "True" {
            Ok(true)
        } else if s == "False" {
            Ok(false)
        } else {
            let e = String::from_utf8(output.stderr).unwrap();
            Err(Error::InvalidMarkerError(s, e))
        }
    }

    fn collect_required<'a>(
        &self,
        current: Ref<'a, Dependency>,
        into: &mut HashMap<String, PythonPackage>,
        interpreter: &Interpreter,
    ) -> Result<()> {
        if into.contains_key(current.key()) {
            return Ok(());
        }
        if let Some(python) = current.python() {
            into.insert(current.key().to_string(), python.clone());
        }
        for (child, marker) in current.dependencies().iter() {
            if let Some(m) = marker {
                if !self.evaluate_marker(m, interpreter)? {
                    continue;
                }
            }
            self.collect_required(Ref::clone(&child), into, interpreter)?;
        }
        Ok(())
    }

    // TODO: The current installation plan implementation simply installs
    // things in an undefined (implementation-defined) order. For best
    // compatibility, packages should be installed from leaf to root, so
    // that dependencies can be installed before their dependants.
    fn required_packages(
        &self,
        default: bool,
        extras: Vec<String>,
        interpreter: &Interpreter,
    ) -> Result<HashMap<String, PythonPackage>> {
        let dependencies = self.lock.dependencies();
        let mut deps = HashMap::new();
        if default {
            if let Some(s) = dependencies.default() {
                self.collect_required(s, &mut deps, interpreter)?;
            } else {
                return Err(Error::DefaultSectionNotFound);
            }
        }
        for extra in extras {
            if let Some(s) = dependencies.extra(&extra) {
                self.collect_required(s, &mut deps, interpreter)?;
            } else {
                return Err(Error::ExtraSectionNotFound(extra));
            }
        }
        Ok(deps)
    }

    fn install_into(
        &self,
        project: &Project,
        packages: &HashMap<String, PythonPackage>,
    ) -> Result<()> {
        let env = project.presumed_env_root()?;
        let env = env.to_str().ok_or_else(|| {
            Error::PathRepresentationError(env.to_path_buf())
        })?;

        let mut requirements = HashMap::new();
        for (key, package) in packages.iter() {
            let (hashed, requirement_txt) = package.to_requirement_txt();
            let mut f = NamedTempFile::new()?;
            writeln!(f, "{}", requirement_txt)?;

            let name = f.path().to_str().ok_or_else(|| {
                Error::PathRepresentationError(f.path().to_path_buf())
            })?.to_string();

            // 3-tuple:
            //  * The temporary file, for later cleanup.
            //  * Whether hashes present.
            //  * Path to the temporary file as string, to pass to pip.
            // TempFile objects need to be kept around so they are not deleted.
            requirements.insert(key, (f, hashed, name));
        }

        let mut error_context = vec![];

        for (key, (_, hashed, requirement)) in requirements.into_iter() {
            let mut cmd = project.command(None)?;
            cmd.args(&[
                "-m", "pip", "install",
                "--requirement", &requirement,
                "--prefix", env,
                "--no-deps",
            ]);
            cmd.env("PIP_DISABLE_PIP_VERSION_CHECK", "1");
            cmd.env("PIP_NO_WARN_SCRIPT_LOCATION", "1");
            if hashed {
                cmd.arg("--require-hashes");
            }
            let status = cmd.status()?;
            if !status.success() {
                error_context.push((key.to_string(), status.code()))
            }
        }

        if error_context.is_empty() {
            Ok(())
        } else {
            Err(Error::InstallCommandError(error_context))
        }
    }
}

use crate::cmd;

use std::fs;

use std::ffi::OsStr;
use std::path::Path;
use std::io::ErrorKind;
use failure::{Error, ResultExt, err_msg};

#[cfg(test)]
fn copy_tree(from: &Path, to: &Path) -> Result<(), Error> {
    for entry in fs::read_dir(from).context(format!("Copying {:?}", from))? {
        let entry = entry?;
        let src_path = entry.path();
        let dest_path = to.join(entry.file_name());
        if src_path.is_file() {
            fs::copy(&src_path, &dest_path)
                .context(format!("Copying {:?} under {:?}", src_path, to))?;
        } else if src_path.is_dir() {
            fs::create_dir(&dest_path)
                .context(format!("Creating directory {:?}", dest_path))?;
            copy_tree(&src_path, &dest_path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
fn compare_tree(src: &Path, dst: &Path) -> Result<(), Error> {
    for entry in fs::read_dir(src).context(format!("Reading {:?}", src))? {
        let entry = entry?;
        let src_path = entry.path();
        let dest_path = dst.join(entry.file_name());
        if src_path.is_file() {
            let expected = fs::read(&src_path)
                .context(format!("Reading {:?}", src_path))?;
            let actual = fs::read(&dest_path)
                .context(format!("Reading {:?}", dest_path))?;
            if actual != expected {
                eprintln!("Mismatch in {:?}", entry.file_name());
                eprintln!("<<< EXPECTED\n{}",
                          String::from_utf8_lossy(&expected));
                eprintln!("=== ACTUAL\n{}",
                          String::from_utf8_lossy(&actual));
                eprintln!(">>>");

                panic!("Directory mismatch at {}", src.display());
            }
        } else if src_path.is_dir() {
            compare_tree(&src_path, &dest_path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
fn check_extra_files(src: &Path, dst: &Path) -> Result<(), Error> {
    let mut errors = Vec::<String>::new();
    for entry in fs::read_dir(dst).context(format!("Reading {:?}", dst))? {
        let entry = entry?;
        let dst_path = entry.path();
        let src_path = src.join(entry.file_name());
        if !src_path.exists() {
            errors.push(format!("Unexpected file {:?}", dst_path));
        } else if dst_path.is_dir() {
            check_extra_files(&src_path, &dst_path)?;
        }
    }
    match errors.len() {
        0 => Ok(()),
        _ => Err(err_msg(errors.join("\n"))),
    }
}

#[cfg(test)]
fn push_all(path: &Path, num_threads: usize, expect: bool) -> Result<(), Error> {
    eprintln!("Push all patches in {}", path.display());

    let work_dir = tempfile::tempdir()?;
    let work_path = work_dir.path();
    copy_tree(&path.join("input"), &work_path)?;

    let num_threads = num_threads.to_string();
    let args = [
        OsStr::new("push"),
        OsStr::new("--quiet"),
        OsStr::new("--threads"), OsStr::new(&num_threads),
        OsStr::new("--all"),
        OsStr::new("--directory"), work_path.as_os_str(),
        OsStr::new("--backup"), OsStr::new("always"),
    ];
    let result = cmd::run(&args);

    match result {
        Ok(status) if status == expect => {
            compare_tree(&path.join("expect"), &work_path)?;
            check_extra_files(&path.join("expect"), &work_path)
        },
        Ok(_) => Err(err_msg(match expect {
            true => "Push failed unexpectedly",
            false => "Push was expected to fail but it did not",
        })),
        Err(err) => Err(err)
    }
}

#[cfg(test)]
fn check_series(path: &str, num_threads: usize, expect: bool) -> Result<(), Error> {
    let dir = fs::read_dir(path);
    match dir {
        Ok(dir) => {
            for entry in dir {
                let entry = entry?;
                if let Err(err) = push_all(&entry.path(), num_threads, expect) {
                    for fail in err.iter_chain() {
                        eprintln!("{}", fail);
                    }
                    panic!("Push all failed for {:?}", entry.file_name());
                }
            }
            Ok(())
        },
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
        Err(err) => Err(Error::from(err)),
    }
}

#[cfg(test)]
#[test]
fn ok_series_sequential() -> Result<(), Error> {
    check_series("testdata/quilt/ok", 1, true)
}

#[cfg(test)]
#[test]
fn fail_series_sequential() -> Result<(), Error> {
    check_series("testdata/quilt/fail", 1, false)
}

#[cfg(test)]
const NUM_THREADS: usize = 2;

#[cfg(test)]
#[test]
fn ok_series_parallel() -> Result<(), Error> {
    check_series("testdata/quilt/ok", NUM_THREADS, true)
}

#[cfg(test)]
#[test]
fn fail_series_parallel() -> Result<(), Error> {
    check_series("testdata/quilt/fail", NUM_THREADS, false)
}

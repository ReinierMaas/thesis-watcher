extern crate structopt;

use crate::structopt::StructOpt;
use colored::*;
use lazy_static::lazy_static;
use notify::{watcher, DebouncedEvent::*, RecursiveMode, Watcher};
use regex::bytes::Regex;
use std::env::current_dir;
use std::ffi::OsStr;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::channel;
use std::thread::sleep;
use std::time::Duration;

const DEBOUNCE_TIME: Duration = Duration::from_secs(10);

lazy_static! {
    static ref REGEX: Regex = Regex::new(r"rerunfilecheck").expect("regex error");
}

#[derive(StructOpt, Debug)]
#[structopt(name = "thesis watcher")]
struct Opt {
    #[structopt(short = "r", default_value = "3")]
    rerunfilecheck: u8,
    #[structopt(short = "w", parse(from_os_str))]
    watch: Option<PathBuf>,
    extensions: Vec<String>,
}

fn main() -> notify::Result<()> {
    let opt = Opt::from_args();
    let extensions = if opt.extensions.is_empty() {
        None
    } else {
        Some(opt.extensions)
    };
    let path = opt.watch.unwrap_or(current_dir()?);
    // channel to receive events
    let (tx, rx) = channel();
    // file watcher
    let mut watcher = watcher(tx, DEBOUNCE_TIME)?;
    watcher.watch(&path, RecursiveMode::Recursive)?;
    // handle events
    println!("{} {:?}", "[WATCHING:]".blue(), &path);
    println!("{} {:?}", "[EXTENSIONS:]".blue(), extensions);
    loop {
        match rx.recv() {
            Ok(event) => {
                println!("{} {:?}", "[RECV_OK:]".green(), event);
                let event_path = match event {
                    NoticeWrite(..) => continue,
                    NoticeRemove(..) => continue,
                    Chmod(..) => continue,
                    Rescan => continue,
                    Error(..) => continue,
                    Create(ref path) => path,
                    Write(ref path) => path,
                    Remove(ref path) => path,
                    Rename(.., ref path) => path,
                };
                if !extensions
                    .as_ref()
                    .map(|extensions| match_ext(&extensions, event_path))
                    .unwrap_or(true)
                {
                    println!("{} {:?}", "[DROP_EXT:]".yellow(), event);
                    continue;
                }
                run_make(&path, opt.rerunfilecheck)?;
                sleep(DEBOUNCE_TIME);
                for event in rx.try_iter() {
                    println!("{} {:?}", "[DROP_DRAIN:]".yellow(), event);
                }
            }
            Err(err) => eprintln!("{} {:?}", "[RECV_ERR:]".red(), err),
        }
    }
}

fn run_make(path: &Path, rerunfilecheck: u8) -> io::Result<()> {
    let output = Command::new("make").current_dir(path).output()?;
    if !output.status.success() {
        println!("{} {:?}", "[MAKE_ERROR:]".red(), output.status.code());
    } else {
        println!("{}", "[MAKE_OK:]".green());
    }
    io::stdout().lock().write_all(&output.stdout)?;
    io::stderr().lock().write_all(&output.stderr)?;
    if rerunfilecheck != 0 && REGEX.is_match(&output.stdout) {
        run_make(path, rerunfilecheck - 1)
    } else {
        Ok(())
    }
}

fn match_ext(extensions: &[String], path: &Path) -> bool {
    let ext = path.extension();
    ext.and_then(OsStr::to_str)
        .map(|ext| extensions.iter().any(|extension| extension == ext))
        .unwrap_or(true)
    // true when ext is None
    // true when ext is not valid utf-8
    // true when extension is in extensions
}

extern crate structopt;

use crate::structopt::StructOpt;
use colored::*;
use notify::{watcher, DebouncedEvent::*, RecursiveMode, Watcher};
use std::env::current_dir;
use std::ffi::OsStr;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::mpsc::channel;
use std::time::Duration;
use std::thread::sleep;

const DEBOUNCE_TIME: Duration = Duration::from_secs(10);

#[derive(StructOpt, Debug)]
#[structopt(name = "thesis watcher")]
struct Opt {
    #[structopt(short = "w", parse(from_os_str))]
    watch: Option<PathBuf>,
    extensions: Vec<String>,
}

fn main() -> notify::Result<()> {
    let opt = Opt::from_args();
    let extensions = if opt.extensions.len() == 0 {
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
                let output = run_make(&path)?;
                if !output.status.success() {
                    println!("{} {:?}", "[MAKE_ERROR:]".red(), output.status.code());
                } else {
                    println!("{}", "[MAKE_OK:]".green());
                }
                io::stdout().lock().write_all(&output.stdout)?;
                io::stderr().lock().write_all(&output.stdout)?;
                sleep(DEBOUNCE_TIME);
                for event in rx.try_iter() {
                    println!("{} {:?}", "[DROP_DRAIN:]".yellow(), event);
                }
            }
            Err(err) => eprintln!("{} {:?}", "[RECV_ERR:]".red(), err),
        }
    }
}

fn run_make(path: &Path) -> io::Result<Output> {
    Command::new("make").current_dir(path).output()
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

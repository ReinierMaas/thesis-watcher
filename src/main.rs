extern crate structopt;

use notify::{Watcher, RecursiveMode, watcher, DebouncedEvent::*};
use std::sync::mpsc::channel;
use std::time::Duration;
use std::env::current_dir;
use std::process::{Command, Output};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use crate::structopt::StructOpt;
use colored::*;

#[derive(StructOpt, Debug)]
#[structopt(name="thesis watcher")]
struct Opt {
    #[structopt(name = "Dir", parse(from_os_str))]
    watch: Option<PathBuf>,
}

fn main() -> notify::Result<()> {
    let opt = Opt::from_args();
    let path = opt.watch.unwrap_or(current_dir()?);
    // channel to receive events
    let (tx, rx) = channel();
    // file watcher
    let mut watcher = watcher(tx, Duration::from_secs(10))?;
    watcher.watch(&path, RecursiveMode::Recursive)?;
    // handle events
    loop {
        match rx.recv() {
            Ok(event) => {
                println!("{} {:?}", "[RECV_OK:]".green(), event);
                match event {
                    NoticeWrite(..) => continue,
                    NoticeRemove(..) => continue,
                    Chmod(..) => continue,
                    Rescan => continue,
                    Error(..) => continue,
                    _ => (),
                };
                let output = run_make(&path)?;
                if !output.status.success() {
                    println!("{} {:?}", "[MAKE_ERROR:]".red(), output.status.code());
                } else {
                    println!("{}", "[MAKE_OK:]".green());
                }
                io::stdout().lock().write_all(&output.stdout)?;
                io::stderr().lock().write_all(&output.stdout)?;
                for event in rx.try_iter() {println!("{} {:?}", "[RECV_DROP:]".yellow(), event)}
            },
            Err(err) => eprintln!("{} {:?}", "[RECV_ERR:]".red(), err),
        }
    }
}
fn run_make(path: &Path) -> io::Result<Output> {
    Command::new("make").current_dir(path).output()
}
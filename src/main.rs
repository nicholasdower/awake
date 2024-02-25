#![cfg(target_os = "macos")]

use chrono::{Duration, Local, NaiveDateTime};
use clap::Parser;
use core_foundation::base::TCFType;
use core_foundation::string::{CFString, CFStringRef};
use daemonize::Daemonize;
use exec::execvp;
use libloading::{Library, Symbol};
use signal_hook::{consts::SIGINT, iterator::Signals};
use std::env;
use std::mem::MaybeUninit;
use std::process::{self, Command};
use std::thread;

type IOPMAssertionID = u32;
type IOPMAssertionLevel = u32;
const IOPMASSERTION_LEVEL_ON: u32 = 255;
const IOPMASSERTION_LEVEL_OFF: u32 = 0;

const VERSION: &str = env!("CARGO_PKG_VERSION");

const HELP: &str = "\
usage: awake [-d] [<duration> | <datetime>]

Description

    Keep your Mac awake, optionally for the specified duration (e.g. 12h30m) or until the specified datetime (e.g. 2030-01-01T00:00:00).

Options

    -d, --daemon     Run as a daemon.
    -k, --kill       Kill any running awake processes.
    -h, --help       Print help.
    -v, --version    Print version.\
";

const DATETIME_FORMAT: &str = "%Y-%m-%dT%H:%M:%S";

#[derive(Parser)]
#[command(disable_help_flag = true)]
struct Cli {
    #[arg(short, long)]
    help: bool,

    #[arg(short, long)]
    version: bool,

    #[arg(short, long)]
    daemon: bool,

    #[arg(short, long)]
    kill: bool,

    #[arg()]
    duration: Option<String>,
}

pub struct IOKit {
    library: Library,
    assertion_name: CFString,
}

// See https://github.com/Randomblock1/caffeinate2/blob/master/src/power_management.rs
impl IOKit {
    pub fn new() -> Result<IOKit, String> {
        let library = unsafe {
            Library::new("/System/Library/Frameworks/IOKit.framework/IOKit")
                .map_err(|_| "failed to load IOKit".to_string())?
        };
        let assertion_name = CFString::new("awake");
        Ok(IOKit {
            library,
            assertion_name,
        })
    }

    pub fn create_assertion(&self, assertion_type: &str, state: bool) -> Result<u32, String> {
        let iokit = &self.library;
        let iopmassertion_create_with_name: Symbol<
            unsafe extern "C" fn(
                CFStringRef,
                IOPMAssertionLevel,
                CFStringRef,
                *mut IOPMAssertionID,
            ) -> i32,
        > = unsafe { iokit.get(b"IOPMAssertionCreateWithName") }.map_err(|e| e.to_string())?;

        let type_ = CFString::new(assertion_type);
        let level = if state {
            IOPMASSERTION_LEVEL_ON
        } else {
            IOPMASSERTION_LEVEL_OFF
        };

        let mut id = MaybeUninit::uninit();
        let status = unsafe {
            iopmassertion_create_with_name(
                type_.as_concrete_TypeRef(),
                level,
                self.assertion_name.as_concrete_TypeRef(),
                id.as_mut_ptr(),
            )
        };
        if status == 0 {
            unsafe { Ok(id.assume_init()) }
        } else {
            Err(format!("failed to create assertion ({status})"))
        }
    }

    pub fn release_assertion(&self, assertion_id: u32) -> Result<(), String> {
        let iokit = &self.library;
        let iopmassertion_release: Symbol<unsafe extern "C" fn(IOPMAssertionID) -> u32> =
            unsafe { iokit.get(b"IOPMAssertionRelease") }.map_err(|e| e.to_string())?;

        let status = unsafe { iopmassertion_release(assertion_id) };

        match status {
            0 => Ok(()),          // Success
            0xE00002C2 => Ok(()), // Already released
            _ => Err(format!("failed to release assertion ({status})")),
        }
    }

    pub fn declare_user_activity(&self, state: bool) -> Result<u32, String> {
        let iokit = &self.library;
        let iopmassertion_declare_user_activity: Symbol<
            unsafe extern "C" fn(CFStringRef, IOPMAssertionLevel, *mut IOPMAssertionID) -> i32,
        > = unsafe {
            iokit
                .get(b"IOPMAssertionDeclareUserActivity")
                .map_err(|e| e.to_string())?
        };

        let level = if state {
            IOPMASSERTION_LEVEL_ON
        } else {
            IOPMASSERTION_LEVEL_OFF
        };

        let mut id = MaybeUninit::uninit();
        let status = unsafe {
            iopmassertion_declare_user_activity(
                self.assertion_name.as_concrete_TypeRef(),
                level,
                id.as_mut_ptr(),
            )
        };
        if status == 0 {
            unsafe { Ok(id.assume_init()) }
        } else {
            Err(format!("failed to declare user activity ({status})"))
        }
    }
}

fn main() {
    match run() {
        Ok(_) => process::exit(0),
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(1);
        }
    }
}

fn run() -> Result<(), String> {
    if !cfg!(target_os = "macos") {
        return Err("not macos".to_string());
    }

    let args = Cli::try_parse().map_err(|e| format!("{}\n{HELP}", e.kind()))?;

    if args.help {
        println!("{HELP}");
        return Ok(());
    }

    if args.version {
        println!("awake {VERSION}");
        return Ok(());
    }

    kill_others()?;

    if args.kill {
        return Ok(());
    }

    let duration = match args.duration {
        Some(duration) => {
            if duration.len() == 19 && duration.chars().nth(4) == Some('-') {
                let datetime = NaiveDateTime::parse_from_str(&duration, DATETIME_FORMAT)
                    .map_err(|e| e.to_string())?;
                let now = Local::now().naive_local();
                let duration = datetime.signed_duration_since(now).num_seconds();
                let duration = std::cmp::max(duration, 0) as u64;
                Some(duration)
            } else {
                let seconds =
                    parse_duration(&duration).map_err(|_| "invalid duration".to_string())?;
                let datetime = Local::now() + Duration::seconds(seconds as i64);
                let datetime_str = datetime.format(DATETIME_FORMAT).to_string();
                let raw_args: Vec<String> = env::args().collect();
                let program_name = "awake".to_string();
                let program_name = raw_args.first().unwrap_or(&program_name);
                let program_args = if args.daemon {
                    vec![program_name.as_str(), "--daemon", &datetime_str.as_str()]
                } else {
                    vec![program_name.as_str(), &datetime_str.as_str()]
                };
                let _ = execvp(program_name, &program_args);
                return Err("failed to replace process".to_string());
            }
        }
        None => None,
    };

    if let Some(duration) = duration {
        if duration == 0 {
            return Ok(());
        }
    }

    if args.daemon {
        let daemonize = Daemonize::new();

        match daemonize.start() {
            Ok(_) => (),
            Err(e) => return Err(e.to_string()),
        }
    }

    let iokit: IOKit = IOKit::new()?;
    let assertions = vec![
        iokit.create_assertion("PreventUserIdleDisplaySleep", true)?,
        iokit.create_assertion("PreventDiskIdle", true)?,
        iokit.create_assertion("PreventUserIdleSystemSleep", true)?,
        iokit.create_assertion("PreventSystemSleep", true)?,
        iokit.declare_user_activity(true)?,
    ];

    let mut signals = Signals::new([SIGINT]).map_err(|e| e.to_string())?;
    let assertions_clone = assertions.clone();
    thread::spawn(move || {
        let signal_iokit = match IOKit::new() {
            Ok(iokit) => iokit,
            Err(e) => {
                eprintln!("error: {e}");
                process::exit(1);
            }
        };
        if signals.forever().next().is_some() {
            match release_assertions(&signal_iokit, &assertions_clone) {
                Ok(_) => process::exit(0),
                Err(e) => {
                    eprintln!("error: {e}");
                    process::exit(1);
                }
            };
        }
    });

    match duration {
        Some(duration) => thread::sleep(std::time::Duration::from_secs(duration)),
        None => thread::park(),
    }
    release_assertions(&iokit, &assertions)
}

fn release_assertions(iokit: &IOKit, assertions: &[u32]) -> Result<(), String> {
    assertions
        .iter()
        .try_for_each(|assertion| iokit.release_assertion(*assertion))
}

fn parse_duration(input: &str) -> Result<u64, ()> {
    if input.is_empty() {
        return Err(());
    }

    let mut seconds: u64 = 0;
    let mut number_string = "".to_string();
    let mut last_char_was_digit = false;
    let mut max_factor = u64::MAX;

    for c in input.chars() {
        if c.is_ascii_digit() {
            number_string.push(c);
            last_char_was_digit = true;
        } else {
            if !last_char_was_digit {
                return Err(());
            }
            if number_string.len() > 1 && number_string.starts_with('0') {
                return Err(());
            }
            let number = number_string.parse::<u64>().map_err(|_| ())?;
            if number == 0 && number_string != "0" {
                return Err(());
            }
            let factor = match c {
                'd' => 24 * 60 * 60,
                'h' => 60 * 60,
                'm' => 60,
                's' => 1,
                _ => return Err(()),
            };
            if factor >= max_factor {
                return Err(());
            }
            max_factor = factor;
            seconds += number * factor;
            number_string = "".to_string();
            last_char_was_digit = false;
        }
    }

    if !number_string.is_empty() || last_char_was_digit {
        return Err(());
    }

    Ok(seconds)
}

fn kill_others() -> Result<(), String> {
    let current_pid = process::id().to_string();
    let output = Command::new("pgrep")
        .arg("awake")
        .output()
        .map_err(|_| "failed to list processes".to_string())?;

    if !output.stdout.is_empty() {
        let pids = String::from_utf8_lossy(&output.stdout);
        for pid in pids.split_whitespace() {
            if pid != current_pid {
                let _ = Command::new("kill").arg(pid).output();
            }
        }
    }
    Ok(())
}

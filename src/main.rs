#![cfg(target_os = "macos")]

use clap::Parser;
use core_foundation::base::TCFType;
use core_foundation::string::{CFString, CFStringRef};
use daemonize::Daemonize;
use libloading::{Library, Symbol};
use signal_hook::{consts::SIGINT, iterator::Signals};
use std::env;
use std::fs::File;
use std::mem::MaybeUninit;
use std::process;
use std::thread;

type IOPMAssertionID = u32;
type IOPMAssertionLevel = u32;
const IOPMASSERTION_LEVEL_ON: u32 = 255;
const IOPMASSERTION_LEVEL_OFF: u32 = 0;

const VERSION: &str = env!("CARGO_PKG_VERSION");

const HELP: &str = "\
usage: awake [-d] [<duration>]

Description

    Keep your Mac awake, optionally for the specified duration (e.g. 12h30m).

Options

    -d, --daemonize  Daemonize.
    -h, --help       Print help.
    -v, --version    Print version.\
";

#[derive(Parser)]
#[command(disable_help_flag = true)]
struct Cli {
    #[arg(short, long)]
    help: bool,

    #[arg(short, long)]
    version: bool,

    #[arg(short, long)]
    daemonize: bool,

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

    let duration = match args.duration {
        Some(duration) => {
            Some(parse_duration(&duration).map_err(|_| "invalid duration".to_string())?)
        }
        None => None,
    };

    if args.daemonize {
        let stdout = File::create("/tmp/awake.out").map_err(|e| e.to_string())?;
        let stderr = File::create("/tmp/awake.err").map_err(|e| e.to_string())?;

        let daemonize = Daemonize::new()
            .pid_file("/tmp/awake.pid")
            .working_directory("/tmp")
            .stdout(stdout)
            .stderr(stderr);

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
            if number == 0 {
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

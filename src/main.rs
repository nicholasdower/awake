// From https://github.com/Randomblock1/caffeinate2/commit/514869fd8e1cada8945507cd4136ad02eb433315

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
usage: awake [-d]

Description

    Stay awake.

Options

    -d, --daemonize  Daemonize.
    -h, --help       Print help.
    -v, --version    Print version.
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
}

pub struct IOKit {
    library: Library,
    assertion_name: CFString,
}

impl IOKit {
    pub fn new() -> IOKit {
        let library =
            unsafe { Library::new("/System/Library/Frameworks/IOKit.framework/IOKit").unwrap() };
        let assertion_name = CFString::new("awake");
        IOKit {
            library,
            assertion_name,
        }
    }

    pub fn create_assertion(&self, assertion_type: &str, state: bool) -> u32 {
        let iokit = &self.library;
        let iopmassertion_create_with_name: Symbol<
            unsafe extern "C" fn(
                CFStringRef,
                IOPMAssertionLevel,
                CFStringRef,
                *mut IOPMAssertionID,
            ) -> i32,
        > = unsafe { iokit.get(b"IOPMAssertionCreateWithName") }.unwrap();
        let type_ = CFString::new(assertion_type);
        let level = if state {
            IOPMASSERTION_LEVEL_ON
        } else {
            IOPMASSERTION_LEVEL_OFF
        };

        {
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
                unsafe { id.assume_init() }
            } else {
                panic!(
                    "Failed to create power management assertion with code: {:X}",
                    status
                );
            }
        }
    }

    pub fn release_assertion(&self, assertion_id: u32) {
        let iokit = &self.library;
        let iopmassertion_release: Symbol<unsafe extern "C" fn(IOPMAssertionID) -> u32> =
            unsafe { iokit.get(b"IOPMAssertionRelease") }.unwrap();

        let status = unsafe { iopmassertion_release(assertion_id) };

        match status {
            0 => (),          // Success
            0xE00002C2 => (), // Already released
            _ => panic!(
                "Failed to release power management assertion with code: {:X}",
                status
            ),
        }
    }

    pub fn declare_user_activity(&self, state: bool) -> u32 {
        let iokit = &self.library;
        let iopmassertion_declare_user_activity: Symbol<
            unsafe extern "C" fn(CFStringRef, IOPMAssertionLevel, *mut IOPMAssertionID) -> i32,
        > = unsafe { iokit.get(b"IOPMAssertionDeclareUserActivity") }.unwrap();

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
        if status != 0 {
            panic!("Failed to declare user activity with code: {:X}", status);
        }

        unsafe { id.assume_init() }
    }
}

impl Default for IOKit {
    fn default() -> Self {
        Self::new()
    }
}

fn main() {
    if !cfg!(target_os = "macos") {
        panic!("not macos");
    }

    let args = match Cli::try_parse() {
        Ok(args) => args,
        Err(e) => {
            eprintln!("{}\n{HELP}", e.kind());
            process::exit(1);
        }
    };

    if args.help {
        println!("{HELP}");
        return;
    }

    if args.version {
        println!("quote {VERSION}");
        return;
    }

    let iokit: IOKit = Default::default();
    let assertions = vec![
        iokit.create_assertion("PreventUserIdleDisplaySleep", true),
        iokit.create_assertion("PreventDiskIdle", true),
        iokit.create_assertion("PreventUserIdleSystemSleep", true),
        iokit.create_assertion("PreventSystemSleep", true),
        iokit.declare_user_activity(true),
    ];

    let mut signals = Signals::new([SIGINT]).unwrap();
    let assertions_clone = assertions.clone();
    thread::spawn(move || {
        if signals.forever().next().is_some() {
            release_assertions(&IOKit::new(), &assertions_clone);
            process::exit(0);
        }
    });

    let stdout = File::create("/tmp/awake.out").unwrap();
    let stderr = File::create("/tmp/awake.err").unwrap();

    if args.daemonize {
        let daemonize = Daemonize::new()
            .pid_file("/tmp/awake.pid")
            .working_directory("/tmp")
            .stdout(stdout)
            .stderr(stderr);

        match daemonize.start() {
            Ok(_) => (),
            Err(e) => eprintln!("error: {e}"),
        }
    }

    thread::park();
    release_assertions(&iokit, &assertions);
    process::exit(0);
}

fn release_assertions(iokit: &IOKit, assertions: &Vec<u32>) {
    for assertion in assertions {
        iokit.release_assertion(*assertion);
    }
}

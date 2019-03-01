extern crate regex;

use regex::Regex;
use std::{
    borrow::Cow,
    env,
    fs::{metadata, File},
    io::{prelude::*, BufRead, BufReader, LineWriter},
    process::Command,
    str,
};

// This regex grabs all MC-generated #define statements and for each it
// captures 3 groups: name, cast, value. The "cast" group is optional.
// i.e. "#define SOMETHING   ((DWORD)0x1200L)" -> ("SOMETHING", "DWORD", 0x1200)
const REGEX: &str = r"^#define (\S+)\s+\(?(\([[:alpha:]]+\))?\s*(0x[[:xdigit:]]+)";

const MC_ARGS: &[&str] = &["-U", "-h", "res", "-r", "res", "res/eventmsgs.mc"];

#[cfg(not(windows))]
const MC_BIN: &str = "windmc";
#[cfg(not(windows))]
const RC_BIN: &str = "windres";
#[cfg(not(windows))]
const RC_ARGS: &[&str] = &["-v", "-i", "res/eventmsgs.rc", "-o", "res/eventmsgs.lib"];

#[cfg(not(windows))]
fn prefix_command(cmd: &str) -> Cow<str> {
    Regex::new(r"^(.*)-[^-]+$")
        .unwrap()
        .captures(&env::var("RUSTC_LINKER").unwrap())
        .map_or(cmd.into(), |capts| format!("{}-{}", &capts[1], cmd).into())
}

#[cfg(windows)]
const MC_BIN: &str = "mc.exe";
#[cfg(windows)]
const RC_BIN: &str = "rc.exe";
#[cfg(windows)]
const RC_ARGS: &[&str] = &["/v", "/fo", "res/eventmsgs.lib", "res/eventmsgs.rc"];

#[cfg(windows)]
fn prefix_command(cmd: &str) -> Cow<str> {
    cmd.into()
}

fn run_tool(cmd: &str, args: &[&str]) -> () {
    let mut command = Command::new(prefix_command(cmd).as_ref());
    command.args(args);

    let out = command.output().unwrap();
    println!("{:?}", str::from_utf8(&out.stderr).unwrap());
    println!("{:?}", str::from_utf8(&out.stdout).unwrap());
}

fn gen_rust() -> () {
    let re = Regex::new(REGEX).unwrap();

    let file_out = File::create("res/eventmsgs.rs").unwrap();
    let mut writer = LineWriter::new(file_out);

    let file_in = File::open("res/eventmsgs.h").unwrap();
    for line_res in BufReader::new(file_in).lines() {
        let line = line_res.unwrap();
        if let Some(x) = re.captures(&line) {
            writer
                .write_all(format!("pub const {}: u32 = {};\n", &x[1], &x[3]).as_bytes())
                .unwrap();
        }
    }
}

fn main() {
    let generate = cfg!(not(windows))
        || match metadata("res/eventmsgs.rs") {
        Ok(meta_rs) => {
            let modtime_rs = meta_rs.modified().unwrap();
            let modtime_mc = metadata("res/eventmsgs.mc").unwrap().modified().unwrap();
            modtime_mc > modtime_rs
        }
        Err(_) => true,
    };

    if generate {
        run_tool(MC_BIN, MC_ARGS);
        run_tool(RC_BIN, RC_ARGS);
        gen_rust();
    }

    let dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    println!("cargo:rustc-link-search=native={}/res", dir);
    println!("cargo:rustc-link-lib=dylib=eventmsgs");
}

extern crate regex;
extern crate sha2;

use regex::Regex;
use sha2::{Sha256, Digest};
use std::{
    borrow::Cow,
    env,
    fs::File,
    io::{prelude::*, copy, BufRead, BufReader, LineWriter},
    process::Command,
    str,
};

// This regex grabs all MC-generated #define statements and for each it
// captures 3 groups: name, cast, value. The "cast" group is optional.
// i.e. "#define SOMETHING   ((DWORD)0x1200L)" -> ("SOMETHING", "DWORD", 0x1200)
const REGEX: &str = r"^#define (\S+)\s+\(?(\([[:alpha:]]+\))?\s*(0x[[:xdigit:]]+)";

const INPUT_FILE: &str = "res/eventmsgs.mc";
const GENERATED_FILE: &str = "res/eventmsgs.rs";

const MC_ARGS: &[&str] = &["-U", "-h", "res", "-r", "res", INPUT_FILE];

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

fn gen_rust(origin_hash: &str) -> () {
    let re = Regex::new(REGEX).unwrap();

    let file_out = File::create(GENERATED_FILE).unwrap();
    let mut writer = LineWriter::new(file_out);

    writer.write_all(format!("// Auto-generated from origin with SHA256 {}.\n", origin_hash).as_bytes()).unwrap();

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

fn file_hash(f: &str) -> String {
    let mut file = File::open(f).unwrap();
    let mut hasher = Sha256::new();
    let _count = copy(&mut file, &mut hasher).unwrap();
    let formatted = format!("{:x}", hasher.result());
    println!("file={}, hash={}", f, formatted);
    formatted
}

fn file_contains(f: &str, needle: &str) -> bool {
    let file = File::open(f).unwrap();
    for line in BufReader::new(file).lines() {
        if line.unwrap().contains(needle) {
            println!("file={} contains {}", f, needle);
            return true;
        }
    }
    println!("file={} does not contain {}", f, needle);
    false
}

fn main() {
    let origin_hash = file_hash(INPUT_FILE);

    if !file_contains(GENERATED_FILE, &origin_hash) {
        println!("Generating {} from {} with hash {}", GENERATED_FILE, INPUT_FILE, origin_hash);

        run_tool(MC_BIN, MC_ARGS);
        run_tool(RC_BIN, RC_ARGS);
        gen_rust(&origin_hash);
    }

    let dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    println!("cargo:rustc-link-search=native={}/res", dir);
    println!("cargo:rustc-link-lib=dylib=eventmsgs");
}

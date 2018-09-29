use regex::Regex;
use std::{
    env,
    fs::{metadata, File},
    io::{prelude::*, BufRead, BufReader, LineWriter},
    process::Command,
    str,
};

// This regex grabs all MC-generated #define statements and for each it
// captures 3 groups: name, cast, value. The "cast" group is optional.
// i.e. "#define SOMETHING   ((DWORD)0x1200L)" -> ("SOMETHING", "DWORD", 0x1200)
const REGEX: &str = r"^#define (\S+)\s+\(?(\([[:alpha:]]+\))?(0x[[:xdigit:]]+)";

fn run_mc() -> () {
    let mut command = Command::new("mc.exe");
    command
        .arg("-U")
        .arg("-h")
        .arg("res")
        .arg("-r")
        .arg("res")
        .arg("res\\eventmsgs.mc");
    let out = command.output().unwrap();
    println!("{:?}", str::from_utf8(&out.stderr).unwrap());
}

fn run_rc() -> () {
    let mut command = Command::new("rc.exe");
    command
        .arg("/v")
        .arg("/fo")
        .arg("res\\eventmsgs.lib")
        .arg("res\\eventmsgs.rc");
    let out = command.output().unwrap();
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
    let generate = match metadata("res/eventmsgs.rs") {
        Ok(meta_rs) => {
            let modtime_rs = meta_rs.modified().unwrap();
            let modtime_mc = metadata("res/eventmsgs.mc").unwrap().modified().unwrap();
            modtime_mc > modtime_rs
        }
        Err(_) => true,
    };

    if generate {
        run_mc();
        run_rc();
        gen_rust();
    }

    let dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    println!("cargo:rustc-link-search=native={}\\res", dir);
    println!("cargo:rustc-link-lib=dylib=eventmsgs");
}

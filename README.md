# winlog

[![Latest version](https://img.shields.io/crates/v/winlog.svg)](https://crates.io/crates/winlog)
[![License](https://img.shields.io/crates/l/winlog.svg)](https://gitlab.com/arbitrix/winlog/blob/master/LICENSE)

A simple [Rust log](https://docs.rs/log/latest/log/) backend to send messages to the [Windows event log](https://docs.microsoft.com/en-us/windows/desktop/eventlog/event-logging).

* Writes Rust log messages to the Windows event log using the
  [RegisterEventSourceW](https://docs.microsoft.com/en-us/windows/desktop/api/Winbase/nf-winbase-registereventsourcew)
  and [ReportEventW](https://docs.microsoft.com/en-us/windows/desktop/api/winbase/nf-winbase-reporteventw) APIs.
* Provides utility functions to register/unregister your
  [event source](https://docs.microsoft.com/en-us/windows/desktop/eventlog/event-sources) in the Windows registry.
* Embeds a small (120-byte) message resource library containing the
  necessary log message templates in your executable.
* Does not panic.

The five Rust log levels are mapped to Windows [event types](https://docs.microsoft.com/en-us/windows/desktop/eventlog/event-types) as follows:

| Rust Log Level | Windows Event Type | Windows Event Id |
| -------------- | ------------------ | ---------------- |
| Error          | Error              | 1                |
| Warn           | Warning            | 2                |
| Info           | Informational      | 3                |
| Debug          | Informational      | 4                |
| Trace          | Informational      | 5                |


## Requirements

* Rust stable (tested on 1.29)
* Windows or mingw
* (optional) PowerShell (used for the end-to-end test)
* (optional) [mc.exe](https://docs.microsoft.com/en-us/windows/desktop/wes/message-compiler--mc-exe-) and [rc.exe](https://docs.microsoft.com/en-us/windows/desktop/menurc/resource-compiler) (only required when `eventmsgs.mc` is changed)


## Usage

Add to `cargo.toml`:
```
[dependencies]
winlog = "*"
```


Register the log source in the Windows registry:
```
winlog::register("Example Log"); // silently ignores errors
// or
winlog::try_register("Example Log").unwrap();
```
This usually requires `Administrator` permission so this is usually done during
installation time. If your MSI installer (or similar) registers your event
sources you should not call this.


Use the winlog backend:
```
winlog::init("Example Log").unwrap();
info!("Hello, Event Log");
```


Deregister the log source: 
```
winlog::deregister("Example Log"); // silently ignores errors
// or
winlog::try_deregister("Example Log").unwrap();
```
This is usually done during program uninstall. If your MSI 
installer (or similar) deregisters your event sources you should not call this.


## Building

## Windows

```sh
cargo build --release
```

### MinGW

Install MinGW (Ubuntu):

```sh
sudo apt install mingw-w64
```

Install Rust:

```sh
rustup target install x86_64-pc-windows-gnu
rustup target install i686-pc-windows-gnu
```

Currently the install from rustup doesn't use the correct linker so you have to add the following to `.cargo/config`:

    [target.x86_64-pc-windows-gnu]
    linker = "/usr/bin/x86_64-w64-mingw32-gcc"

    [target.i686-pc-windows-gnu]
    linker = "/usr/bin/i686-w64-mingw32-gcc"
    rustflags = "-C panic=abort"

Build:
```sh
cargo build --release
```

### Internals

Artifacts `eventmsgs.lib` and `eventmsgs.rs` are under source control so users 
don't need to have `mc.exe` and `rc.exe` installed for a standard build.

1. If `build.rs` determines that `eventmsgs.mc` was changed then `build.rs`:
   * invokes `mc.exe` (which creates `eventmsgs.h`)
   * invokes `rc.exe` (which creates `eventmsgs.lib`)
   * creates `eventmsgs.rs` from `eventmsgs.h`.
2. `build.rs` emits linker flags so `eventmsgs.lib` can found.
3. Standard `cargo build` follows.


## Testing

The end-to-end test requires 'Full Control' permissions on the 
`HKLM\SYSTEM\CurrentControlSet\Services\EventLog\Application`
registry key.

```cargo test```

Process:
1. Create a unique temporary event source name (`winlog-test-###########`).
2. Register our compiled test executable as ```EventMessageFile``` for 
   the event source in the Windows registry. You can see a new key at 
   `HKLM\SYSTEM\CurrentControlSet\Services\EventLog\Application\winlog-test-###########`.
2. Write some log messages to the event source.
3. Use PowerShell to retrieve the logged messages.
4. Deregister our event source. This removes the `winlog-test-###########` 
   registry key.
5. Assert that the retrieved log messages are correct. 


## License

Licensed under either of

* Apache License, Version 2.0 (LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license (LICENSE-MIT or http://opensource.org/licenses/MIT)

at your option.


## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted 
for inclusion in the work by you, as defined in the Apache-2.0 license, shall 
be dual licensed as above, without any additional terms or conditions.
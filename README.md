
# procshot_server  [![Build Status](https://travis-ci.org/nohupped/procshot_server.svg?branch=master)](https://travis-ci.org/nohupped/procshot_server)

A lame attempt of a rust [crate (refer this for crate documentation)](https://crates.io/crates/procshot_server) to record /proc stats periodically. This library just records the stats. Processing to be done separately. This is written as a part of learning rust.

## Server example

```rust
use procshot_server::{Config, check_sudo, scan_proc};
use std::process;
use users::get_current_uid;
use procshot_client;
const DATADIR: &'static str = "/var/log/procshot/data";

fn main() {
    match check_sudo(get_current_uid()) {
        Err(e) => {
            eprintln!("Error encountered checking privileges, {:?}", e);
            process::exit(1);
        },
        _ => (),
    }
    std::fs::create_dir_all(DATADIR).unwrap();
    let config: Config = Config::new();
    match config.server{
        true => scan_proc(config.delay, config.hostname, DATADIR),
        false => procshot_client::read_test_data(),
    }
}
```

This will generate a binary with the following cli options

```bash

 procshot 1.0
 nohupped@gmail.com
 Snapshots proc periodically. All the options except delay works when 'server' option is not used.

 USAGE:
     procshot [FLAGS] [OPTIONS] [SUBCOMMAND]

 FLAGS:
     -h, --help       Prints help information
     -o               Sort result by Memory or CPU. Accepted values are...
     -t               Read stats from a specific time. Accepted format: 2015-09-05 23:56:04
     -V, --version    Prints version information

 OPTIONS:
     -d, --delay <delay>      Sets delay in seconds before it scans /proc every time. Defaults to 60 seconds. [default: 60]


 SUBCOMMANDS:
     help      Prints this message or the help of the given subcommand(s)
     server    Decides whether to run as server or client
```

## Client example on how to read the stored data

```rust
 use std::fs::File;
 use std::io::Read;
 use procshot_server::EncoDecode;
 pub fn read_test_data() {
         let mut file = File::open("./test_data.procshot").unwrap();
         let mut data = Vec::new();
         file.read_to_end(&mut data).unwrap();
         let decoded: EncoDecode = bincode::deserialize(&data[..]).unwrap_or_else(|x| panic!("Error reading saved data. This was either created with an older version of procshot, or the file is corrupt. Error is {}", x));
         println!("Decoded test file data: {:#?}", decoded);
 }

```

## Sample output of stored data

`$ sudo ./target/release/procshot`
```shell             
Decoded test file data: EncoDecode {
    hostname: "localghost",
    pid_map_list: {
        169883: PidStatus {
            ppid: 3284,
            euid: 1000,
            cmd_long: [
                "/opt/google/chrome/chrome --type=renderer --field-trial-handle=11321657219978072658,9761930160602287450,131072 --lang=en-US --disable-client-side-phishing-detection --enable-auto-reload --num-raster-threads=4 --enable-main-frame-before-activation --service-request-channel-token=1438087544763584805 --renderer-client-id=214 --no-v8-untrusted-code-mitigations --shared-files=v8_context_snapshot_data:100,v8_natives_data:101",
            ],
            name: "chrome",
            cmd_short: "chrome",
            tracerpid: 0,
            fdsize: 64,
            state: "S (sleeping)",
            vmpeak: Some(
                549888,
            ),
            vmsize: Some(
                525300,
            ),
            rss_pages: 13139,
            rss_bytes: 53817344,
            rsslim_bytes: 18446744073709551615,
            processor_last_executed: Some(
                4,
            ),
            utime: 5,
            stime: 2,
            user_cpu_usage: 0.0,
            sys_cpu_usage: 0.0,
        },
        2078: PidStatus {
            ppid: 1783,
            euid: 1000,
            cmd_long: [
                "/usr/lib/at-spi2-registryd",
                "--use-gnome-session",
            ],
            name: "at-spi2-registr",
            cmd_short: "at-spi2-registr",
            tracerpid: 0,
            fdsize: 64,
            state: "S (sleeping)",
            vmpeak: Some(
                229136,
            ),
            vmsize: Some(
                163600,
            ),
            rss_pages: 1472,
            rss_bytes: 6029312,
            rsslim_bytes: 18446744073709551615,
            processor_last_executed: Some(
                2,
            ),
            utime: 1,
            stime: 0,
            user_cpu_usage: 0.0,
            sys_cpu_usage: 0.0,
        },
        2130: PidStatus {
            ppid: 1783,
            euid: 1000,
            cmd_long: [
                "/usr/lib/gsd-screensaver-proxy",
            ],
            name: "gsd-screensaver",
            cmd_short: "gsd-screensaver",
            tracerpid: 0,
            fdsize: 64,
            state: "S (sleeping)",
            vmpeak: Some(
                234084,
            ),
            vmsize: Some(
                234084,
            ),
            rss_pages: 1094,
            rss_bytes: 4481024,
            rsslim_bytes: 18446744073709551615,
            processor_last_executed: Some(
                6,
            ),
            utime: 1,
            stime: 0,
            user_cpu_usage: 0.0,
            sys_cpu_usage: 0.0,
        },
        2112: PidStatus {
            ppid: 1783,
            euid: 1000,
            cmd_long: [
                "/usr/lib/gsd-housekeeping",
            ],
            name: "gsd-housekeepin",
            cmd_short: "gsd-housekeepin",
            tracerpid: 0,
            fdsize: 64,
            state: "S (sleeping)",
            vmpeak: Some(
                375152,
            ),
            vmsize: Some(
                310204,
            ),
            rss_pages: 1764,
            rss_bytes: 7225344,
            rsslim_bytes: 18446744073709551615,
            processor_last_executed: Some(
                10,
            ),
            utime: 7,
            stime: 7,
            user_cpu_usage: 0.0,
            sys_cpu_usage: 0.0,
        },
..... snip .....

    time_epoch: 1563617611,
    delay: 5,
    total_cpu_time: 6331606,


```

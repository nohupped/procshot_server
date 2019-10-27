
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
    pid_map_list: [
        {
            1: PidStatus {
                ppid: 0,
                euid: 0,
                cmd_long: [
                    "/sbin/init",
                ],
                name: "systemd",
                cmd_short: "systemd",
                tracerpid: 0,
                fdsize: 256,
                state: "S (sleeping)",
                vmpeak: Some(
                    252840,
                ),
                vmsize: Some(
                    187304,
                ),
                rss_pages: 2565,
                rss_bytes: 10506240,
                rsslim_bytes: 18446744073709551615,
                processor_last_executed: Some(
                    11,
                ),
                utime: 62,
                stime: 377,
            },
        },
        {
            373: PidStatus {
                ppid: 1,
                euid: 0,
                cmd_long: [
                    "/usr/lib/systemd/systemd-journald",
                ],
                name: "systemd-journal",
                cmd_short: "systemd-journal",
                tracerpid: 0,
                fdsize: 256,
                state: "S (sleeping)",
                vmpeak: Some(
                    56756,
                ),
                vmsize: Some(
                    49068,
                ),
                rss_pages: 2486,
                rss_bytes: 10182656,
                rsslim_bytes: 18446744073709551615,
                processor_last_executed: Some(
                    2,
                ),
                utime: 50,
                stime: 39,
            },
        },
..... snip .....

    time_epoch: 1563617611,
    delay: 5,
    total_cpu_time: 6331606,


```

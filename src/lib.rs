//! This crate can be used to continuously scan over `/proc` filesystem and store it in the struct `EncoDecode. This struct is serialized and is written to the `datadir`.
//! This is a wrapper over the [procfs](https://docs.rs/procfs/0.5.3/procfs/) crate, so the compatibility of this crate depends on the compatibility of the [procfs](https://docs.rs/procfs/0.5.3/procfs/) crate.
//!
//! The stored data is of type `EncoDecode` and can be read as:
//!
//! # Examples
//!
//! ```rust
//! use std::fs::File;
//! use std::io::Read;
//! use procshot_server::EncoDecode;
//! pub fn read_test_data() {
//!         let mut file = File::open("./test_data.procshot").unwrap();
//!         let mut data = Vec::new();
//!         file.read_to_end(&mut data).unwrap();
//!         let decoded: EncoDecode = bincode::deserialize(&data[..]).unwrap_or_else(|x| panic!("Error reading saved data. This was either created with an older version of procshot, or the file is corrupt. Error is {}", x));
//!         println!("Decoded test file data: {:#?}", decoded);
//! }
//! ```

extern crate procfs;
use std::collections::HashMap;
use std::thread;
use std::time::Duration;
#[macro_use]
extern crate serde_derive;
extern crate serde;
use std::fs::File;
use std::io::Write;
use std::io::{BufRead, BufReader};

// Tmp imports

extern crate clap;
extern crate hostname;
use clap::{App, Arg, SubCommand};

/// PidStatus is the struct that holds the data that we store for each process' status. In this crate, we create a
/// ` Vec<HashMap<i32, PidStatus>>` which is a mapping of pid to its status.
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct PidStatus {
    /// Parent pid
    pub ppid: i32,
    /// Effective uid
    pub euid: i32,
    /// The complete path to cmd_long if available.
    pub cmd_long: Vec<String>,
    /// Command run by this process.
    pub name: String,
    /// The filename of the executable, in parentheses.
    ///
    /// This is visible whether or not the executable is swapped out.
    pub cmd_short: String,
    /// PID of process tracing this process (0 if not being traced).
    pub tracerpid: i32,
    /// Number of file descriptor slots currently allocated.
    pub fdsize: u32,
    /// Current state of the process.
    pub state: String,
    /// Peak virtual memory size by kB.
    pub vmpeak: Option<u64>,
    /// Virtual memory size by kB.
    pub vmsize: Option<u64>,
    /// Resident Set Size: number of pages the process has in real memory.
    ///
    /// This is just the pages which count toward text,  data,  or stack space.
    /// This does not include pages which have not been demand-loaded in, or which are swapped out.
    pub rss_pages: i64,
    /// Gets the Resident Set Size (in bytes)
    pub rss_bytes: i64,
    /// Current soft limit in bytes on the rss of the process; see the description of RLIMIT_RSS in
    /// getrlimit(2).
    pub rsslim_bytes: u64,
    /// CPU number last executed on.
    ///
    /// (since Linux 2.2.8)
    pub processor_last_executed: Option<i32>,
    // Amount of time that this process has been scheduled in user mode, measured in clock ticks
    /// (divide by [`ticks_per_second()`].
    ///
    /// This includes guest time, guest_time (time spent running a virtual CPU, see below), so that
    /// applications that are not aware of the guest time field  do not lose that time from their
    /// calculations.
    pub utime: u64,
    /// Amount of time that this process has been scheduled in kernel mode, measured in clock ticks
    /// (divide by [`ticks_per_second()`]).
    pub stime: u64,
    /// Holds the user CPU usage by that process.
    pub user_cpu_usage: f64,
    /// Holds the sys CPU usage by that process.    
    pub sys_cpu_usage: f64,
}

/// EncodDecode is the struct that we use to hold additional metadata and write to disk as
/// serialized data of the form `let enc encoded: Vec<u8> = bincode::serialize(&encodecode).unwrap();`.
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct EncoDecode {
    pub hostname: String,
    /// Vector of hashmap of pid to the pidstats.
    pub pid_map_list: HashMap<i32, PidStatus>,
    /// The epoch time at which the stats were recorded
    pub time_epoch: u64,
    /// Can be used for sampling
    pub delay: u64,
    /// The cumilative CPU time in jiffies.
    pub total_cpu_time: u64,
}

/// scan_proc continuously scans /proc and records all the processes.
/// scan_proc omits the pids if status.vmpeak == None || prc.stat.rss == 0 || status.pid < 0.
/// One file is created for each iteration and sleeps for `delay` seconds after each iteration.
/// The example in the description can be used as a reference to read the stored struct.
pub fn scan_proc(delay: u64, host: String, datadir: &'static str) {
    print!("Starting procshot server with delay set as {}", delay);

    let mut previous_stats: Option<HashMap<i32, PidStatus>> = None;
    let mut previous_cpu_time: u64 = 0;
    // Starts the continuous iteration over /proc
    loop {
        let mut pid_map_hash: HashMap<i32, PidStatus> = HashMap::new(); //Vec::new();
        let time_epoch = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let total_cpu_time = match read_proc_stat() {
            Ok(t) => t,
            Err(e) => {
                eprintln!("Cannot read from /proc/stat, error is:: {:?}", e);
                continue;
            }
        };

        // Iterate over all processess
        for prc in procfs::all_processes() {
            let status = prc.status().unwrap_or_else(|_| dummy_pid_status());
            if status.vmpeak == None || prc.stat.rss == 0 || status.pid < 0 {
                continue;
            }
            let s = PidStatus {
                ppid: status.ppid,
                euid: status.euid,
                cmd_long: prc
                    .cmdline()
                    .unwrap_or_else(|_| vec!["No cmd_long found".to_string()]),
                name: status.name,
                cmd_short: prc.stat.comm.clone(),
                tracerpid: status.tracerpid,
                fdsize: status.fdsize,
                state: status.state,
                vmpeak: status.vmpeak,
                vmsize: status.vmsize,
                rss_pages: prc.stat.rss,
                rss_bytes: prc.stat.rss_bytes(),
                rsslim_bytes: prc.stat.rsslim,
                processor_last_executed: prc.stat.processor,
                utime: prc.stat.utime,
                stime: prc.stat.stime,
                user_cpu_usage: get_cpu_usage(
                    "user".to_string(),
                    status.pid,
                    &previous_stats,
                    prc.stat.utime,
                    total_cpu_time,
                    previous_cpu_time,
                ),
                sys_cpu_usage: get_cpu_usage(
                    "system".to_string(),
                    status.pid,
                    &previous_stats,
                    prc.stat.stime,
                    total_cpu_time,
                    previous_cpu_time,
                ),
            };

            // let mut pidmap: HashMap<i32, PidStatus> = HashMap::new();
            pid_map_hash.insert(status.pid, s);
        }
        previous_stats = Some(pid_map_hash.clone());
        previous_cpu_time = total_cpu_time;

        let encodecode: EncoDecode = EncoDecode {
            hostname: host.clone(),
            pid_map_list: pid_map_hash,
            delay: delay,
            time_epoch: time_epoch,
            total_cpu_time: total_cpu_time,
        };
        let encoded: Vec<u8> = bincode::serialize(&encodecode).unwrap();
        // println!("DECODED VALUES:: {:#?}", decoded);
        //assert_eq!(pids, decoded);
        let file = File::create(format! {"{}/{}.procshot", datadir, time_epoch});
        match file {
            Err(e) => eprintln!("Cannot create file!, err: {}", e),
            Ok(mut f) => {
                f.write_all(&encoded).unwrap();
            }
        }
        thread::sleep(Duration::from_secs(delay));
    }
}

/// get_cpu_usage calculates cpu usage for user/system.
/// user_util = 100 * (utime_after - utime_before) / (time_total_after - time_total_before);
/// sys_util = 100 * (stime_after - stime_before) / (time_total_after - time_total_before);
fn get_cpu_usage(
    type_of: String,
    pid: i32,
    previous: &Option<HashMap<i32, PidStatus>>,
    current_type_time: u64,
    current_cpu_time: u64,
    previous_cpu_time: u64,
) -> f64 {
    match type_of.as_ref() {
        "user" => match previous {
            Some(x) => match x.get(&pid) {
                Some(p) => {
                    100 as f64 * (current_type_time as f64 - p.utime as f64) / (current_cpu_time as f64 - previous_cpu_time as f64)
                }
                None => {
                    0.0
                }
            },
            None => {
                0.0
            }
        },
        "system" => match previous {
            Some(x) => match x.get(&pid) {
                Some(p) => {
                    100 as f64 * (current_type_time as f64 - p.stime as f64)
                        / (current_cpu_time as f64 - previous_cpu_time as f64)
                }
                None => 0.0,
            },
            None => 0.0,
        },
        _ => {
            println!("Keyword not supported!");
            0.0
        }
    }
}

/// Reads and parses /proc/stat's first line for calculating cpu percentage
fn read_proc_stat() -> Result<u64, std::io::Error> {
    let f = match File::open("/proc/stat") {
        Ok(somefile) => somefile,
        Err(e) => return Err(e),
    };

    let mut reader_itr = BufReader::new(f).lines();
    let first_line = match reader_itr.next() {
        // next returns an Option<Result<>> type, and hence the nested some(ok())
        Some(total_string) => match total_string {
            Ok(s) => s,
            Err(e) => return Err(e),
        },
        None => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Cannot read the first line from /proc/stat.",
            ))
        }
    };
    let total_vector = first_line
        .split("cpu") // Split at "cpu"
        .collect::<Vec<&str>>()[1] // Skip 0th element
        .split(" ") // Split at " "
        .filter(|&x| x != "") // filter empty lines
        .collect::<Vec<&str>>(); // collect
    let mut total: u64 = 0;
    for i in total_vector {
        total += i.parse::<u64>().unwrap();
    }
    Ok(total)
}

///dummy_status is used to return a dummy procfs::Status struct
fn dummy_pid_status() -> procfs::Status {
    let ds = "Dummy because unwrap failed".to_string();
    procfs::Status {
        name: ds.clone(),
        umask: Some(std::u32::MAX),
        state: ds.clone(),
        tgid: -1,
        ngid: Some(-1),
        pid: -1,
        ppid: -1,
        tracerpid: -1,
        ruid: -1,
        euid: -1,
        suid: -1,
        fuid: -1,
        rgid: -1,
        egid: -1,
        sgid: -1,
        fgid: -1,
        fdsize: std::u32::MAX,
        groups: vec![-1],
        nstgid: Some(vec![-1]),
        nspid: Some(vec![-1]),
        nspgid: Some(vec![-1]),
        nssid: Some(vec![-1]),
        vmpeak: Some(std::u64::MAX),
        vmsize: Some(std::u64::MAX),
        vmlck: Some(std::u64::MAX),
        vmpin: Some(std::u64::MAX),
        vmhwm: Some(std::u64::MAX),
        vmrss: Some(std::u64::MAX),
        rssanon: Some(std::u64::MAX),
        rssfile: Some(std::u64::MAX),
        rssshmem: Some(std::u64::MAX),
        vmdata: Some(std::u64::MAX),
        vmstk: Some(std::u64::MAX),
        vmexe: Some(std::u64::MAX),
        vmlib: Some(std::u64::MAX),
        vmpte: Some(std::u64::MAX),
        vmswap: Some(std::u64::MAX),
        hugetblpages: Some(std::u64::MAX),
        threads: std::u64::MAX,
        sigq: (std::u64::MAX, std::u64::MAX),
        sigpnd: std::u64::MAX,
        shdpnd: std::u64::MAX,
        sigblk: std::u64::MAX,
        sigign: std::u64::MAX,
        sigcgt: std::u64::MAX,
        capinh: std::u64::MAX,
        capprm: std::u64::MAX,
        capeff: std::u64::MAX,
        capbnd: Some(std::u64::MAX),
        capamb: Some(std::u64::MAX),
        nonewprivs: Some(std::u64::MAX),
        seccomp: Some(std::u32::MAX),
        speculation_store_bypass: Some(ds.clone()),
        cpus_allowed: Some(vec![std::u32::MAX]),
        cpus_allowed_list: Some(vec![(std::u32::MAX, std::u32::MAX)]),
        mems_allowed: Some(vec![std::u32::MAX]),
        mems_allowed_list: Some(vec![(std::u32::MAX, std::u32::MAX)]),
        voluntary_ctxt_switches: Some(std::u64::MAX),
        nonvoluntary_ctxt_switches: Some(std::u64::MAX),
    }
}

/// Config struct holds the user input when running the server. It is a bad design to hold the client's option as well in the same struct, but
/// as of now, it is here.
#[derive(Debug)]
pub struct Config {
    /// hostname of the server. This is derived by this crate from the [hostname](https://docs.rs/hostname/0.1.5/hostname/) crate.
    pub hostname: String,
    /// Delay decides how many seconds to sleep after each iteration of scanning /proc
    pub delay: u64,
    /// If true, runs as server. Defaults to false. Pass the subcommand `server` to set it to true.
    pub server: bool,
    /// The time from which the client can fetch data to process.
    pub client_time_from: String,
    /// Sort the processed data by whatever the user wants.
    pub client_sort_by: String,
}

/// Returns a new config object. This also gives the following command line argument options.
/// # Examples
/// Here are the cli options used to populate the struct.
/// > sudo target/debug/procshot --help
///
/// ```bash
/// procshot 1.0
/// nohupped@gmail.com
/// Snapshots proc periodically. All the options except delay works when 'server' option is not used.
///
/// USAGE:
///     procshot [FLAGS] [OPTIONS] [SUBCOMMAND]

/// FLAGS:
///     -h, --help       Prints help information
///     -o               Sort result by Memory or CPU. Accepted values are...
///     -t               Read stats from a specific time. Accepted format: 2015-09-05 23:56:04
///     -V, --version    Prints version information
///
/// OPTIONS:
///     -d, --delay <delay>      Sets delay in seconds before it scans /proc every time. [default: 60]
///
/// SUBCOMMANDS:
///     help      Prints this message or the help of the given subcommand(s)
///     server    Decides whether to run as server or client
impl Config {
    pub fn new() -> Self {
        let matches = App::new("procshot")
                        .version("1.0")
                        .author("nohupped@gmail.com")
                        .about("Snapshots proc periodically. All the options except delay works when 'server' option is not used.")
                        .arg(Arg::with_name("delay")
                            .short("d")
                            .long("delay")
                            .default_value("60")
                            .help("Sets delay in seconds before it scans /proc every time."))
                        .subcommand(SubCommand::with_name("server")
                            .about("Runs as server and records stats."))
                        .arg(Arg::with_name("time_from")
                            .short("t")
                            .help("Read stats from a specific time. Accepted format: 2015-09-05 23:56:04")
                            )
                        .arg(Arg::with_name("order_by")
                            .short("o")
                            .help("Sort result by Memory or CPU. Accepted values are...") // Todo here
                            )
                        .get_matches();

        Config {
            hostname: hostname::get_hostname().unwrap().to_string(),
            delay: matches
                .value_of("delay")
                .unwrap_or("60")
                .parse()
                .unwrap_or(60),
            server: match matches.subcommand_matches("server") {
                Some(_) => true,
                None => false,
            },
            client_time_from: matches.value_of("time_from").unwrap_or("").to_string(),
            client_sort_by: matches.value_of("order_by").unwrap_or("m").to_string(),
        }
    }
}
/// Checks if the program is run as sudo (root) user. This doesn't check if the user has the privilege to read over all of /proc or write to the datadir
/// but just checks if the uid passed to this is 0, and returns a `Result`
///
/// # Examples
///
///```rust
///
/// use procshot_server::check_sudo;
/// use std::process;
///
/// fn main() {
///     match check_sudo(0) { // Can also use get_current_uid() from the `users` crate
///         Err(e) => {
///             eprintln!("Error encountered checking privileges, {:?}", e);
///             process::exit(1);
///         },
///         _ => (),
///     }
/// }
///```
pub fn check_sudo(uid: u32) -> Result<(), &'static str> {
    match uid == 0 {
        true => Ok(()),
        false => Err("Error: Run as root."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_sudo_privileged() {
        match check_sudo(0) {
            Ok(()) => (),
            Err(e) => panic!("Test failed, {:?}", e),
        }
    }

    #[test]
    #[should_panic]
    fn test_check_sudo_non_privileged() {
        match check_sudo(10) {
            Ok(()) => (),
            Err(e) => panic!("Test failed, {:?}", e),
        }
    }
}

use std::{
    sync::{
        mpsc::{Receiver, SyncSender},
        Arc, Mutex,
    },
    thread::{sleep, spawn, JoinHandle},
    time::{Duration, Instant},
};

use sysinfo::{Pid, Signal, System};
use thousands::Separable;

use crate::{
    config::{Config, KillTabStrategy},
    Status, BROWSER_NAME,
};

pub fn spawn_tab_killer_thread(
    status: Arc<Mutex<Status>>,
    config: Config,
    update_req_sender: SyncSender<()>,
    update_result_reciever: Receiver<Result<(), String>>,
) -> JoinHandle<()> {
    spawn(move || {
        // The duration loop sleep for
        let tick = Duration::from_secs_f32(config.check_interval_secs);
        let update_status_timeout = tick * 4;
        loop {
            let start_instant = Instant::now();

            println!("Request update status"); // debug!
            match update_req_sender.try_send(()) {
                Ok(_) => (),
                Err(_) => {
                    eprintln!("Failed to request update status");
                }
            }

            // Waiting for status update
            match update_result_reciever.recv_timeout(update_status_timeout) {
                Ok(update_result) => match update_result {
                    Ok(_) => {
                        println!("Status update successed"); // debug!
                        kill_tabs_by_strategy(status.clone(), &config);
                    }
                    Err(e) => {
                        eprintln!("Cannot update status, skip this round: {e}");
                    }
                },
                Err(_) => {
                    eprintln!("Timeout, retry requesting data!");
                    continue;
                }
            }

            let end_instant = Instant::now();
            let consumed_time = end_instant - start_instant;
            if tick < consumed_time {
                eprintln!(
                    "Consumed time {:?} exceed check interval {:?}, delay: {:?}",
                    consumed_time,
                    tick,
                    consumed_time - tick
                );
                continue;
            }

            let sleep_duration = tick - consumed_time;
            println!("Sleep for {:?}\n", sleep_duration); // debug!
            sleep(sleep_duration);
        }
    })
}

fn kill_tabs_by_strategy(status: Arc<Mutex<Status>>, config: &Config) {
    // Clear stat if browser closed
    let system = System::new_all();
    let browser_process_count = system
        .processes_by_exact_name(BROWSER_NAME.as_ref())
        .count();
    if browser_process_count == 0 {
        status.lock().unwrap().tab_infos.clear();
    }

    // Print stat
    println!("{:?}", status.lock().unwrap());
    let total_rss: u64 = status
        .lock()
        .unwrap()
        .tab_infos
        .keys()
        .map(|pid| {
            if let Some(process) = system.processes().get(pid) {
                process.memory()
            } else {
                0
            }
        })
        .sum();
    println!("total_rss: {}", total_rss.separate_with_commas());

    // Apply kill tab startegies
    if status.lock().unwrap().tab_infos.len() > 0 {
        for kill_tab_strategy in &config.kill_tab_strategies {
            // Apply strategy
            match kill_tab_strategy {
                KillTabStrategy::RssLimit => {
                    if total_rss > config.strategy.rss_limit.max_bytes {
                        println!(
                            "Hit the rss limit({}/{}), apply RssLimit strategy",
                            total_rss.separate_with_commas(),
                            config.strategy.rss_limit.max_bytes.separate_with_commas()
                        );

                        type Rss = u64;
                        // The background tab processes pid and rss, sorted by rss
                        let mut sorted_pid_rss: Vec<(Pid, Rss)> = status
                            .lock()
                            .unwrap()
                            .tab_infos
                            .iter()
                            .filter(|(_pid, tab_info)| !tab_info.active)
                            .map(|(&pid, _tab_info)| {
                                if let Some(process) = system.processes().get(&pid) {
                                    (pid, process.memory())
                                } else {
                                    (pid, 0)
                                }
                            })
                            .collect();
                        sorted_pid_rss.sort_unstable_by_key(|&(_, rss)| rss);

                        // Get pids to kill
                        let exceed_rss = total_rss - config.strategy.rss_limit.max_bytes;
                        let mut expected_freed_rss = 0;
                        let mut killing_pids = Vec::new();
                        for &(pid, rss) in sorted_pid_rss.iter().rev() {
                            if exceed_rss >= expected_freed_rss {
                                let url = status.lock().unwrap().tab_infos[&pid].url.clone();
                                let in_whitelist =
                                    config.whitelist.iter().any(|regex| regex.is_match(&url));
                                if in_whitelist {
                                    continue;
                                }

                                expected_freed_rss += rss;
                                killing_pids.push(pid);
                            } else {
                                break;
                            }
                        }

                        killing_pids.iter().for_each(|pid| {
                            let signal = Signal::Term;
                            if let Some(process) = system.processes().get(pid) {
                                match process.kill_with(signal) {
                                    Some(success) => {
                                        if !success {
                                            eprintln!(
                                                "Failed to send signal {} to {},",
                                                signal, pid
                                            );
                                        }
                                    }
                                    None => {
                                        eprintln!(
                                            "The signal {} is not supported on this platform!",
                                            signal
                                        );
                                    }
                                }
                            }
                        })
                    }
                }
                KillTabStrategy::IdleTimeLimit => {
                    eprintln!("Strategy idle_time_limit not implemented!");
                }
                KillTabStrategy::MemoryChangeRate => {
                    eprintln!("Strategy memory_change_rate not implemented!");
                }
            }
        }
    }
}

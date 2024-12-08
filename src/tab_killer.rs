use std::{
    collections::BTreeMap,
    sync::{
        mpsc::{Receiver, SyncSender},
        Arc, Mutex,
    },
    thread::{sleep, spawn, JoinHandle},
    time::{Duration, Instant},
};

use debug_print::debug_println;
use sysinfo::Signal;
use thousands::Separable;

use crate::{
    config::{Config, KillTabStrategy},
    Status,
};

pub type Rss = u64;

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

            debug_println!("Request update status");
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
                        debug_println!("Status update successed");
                        kill_tabs_by_strategies(status.clone(), &config);
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
            println!("Tick consumed {:?}/{:?}\n", consumed_time, tick);
            sleep(sleep_duration);
        }
    })
}

fn kill_tabs_by_strategies(status: Arc<Mutex<Status>>, config: &Config) {
    let status = &mut status.lock().unwrap();

    debug_println!("{:?}", status);
    println!(
        "Tabs: {:?}",
        status
            .tab_infos
            .iter()
            .map(|(pid, tab_info)| (pid, tab_info.title.clone()))
            .collect::<BTreeMap<_, _>>()
    );

    let total_rss: u64 = status
        .tab_infos
        .keys()
        .map(|pid| {
            if let Some(process) = status.system.processes().get(pid) {
                process.memory()
            } else {
                0
            }
        })
        .sum();
    println!("Total rss: {}", total_rss.separate_with_commas());

    // Apply kill tab startegies
    if status.tab_infos.len() > 0 {
        for kill_tab_strategy in &config.kill_tab_strategies {
            // Apply strategy
            match kill_tab_strategy {
                KillTabStrategy::RssLimit => {
                    kill_tabs_by_rss_limit(status, &config, total_rss);
                }
                KillTabStrategy::BackgroundTimeLimit => {
                    kill_tabs_by_background_time_limit(status, &config);
                }
                KillTabStrategy::CpuIdleTimeLimit => {
                    kill_tabs_by_cpu_idle_time_limit(status, &config);
                }
            }
        }
    }
}

fn kill_tabs_by_rss_limit(status: &Status, config: &Config, total_rss: u64) {
    if total_rss > config.strategy.rss_limit.max_bytes {
        println!(
            "Hit the rss limit({}/{}), apply RssLimit strategy",
            total_rss.separate_with_commas(),
            config.strategy.rss_limit.max_bytes.separate_with_commas()
        );

        let sorted_pid_rss = status.get_sorted_pid_rss();
        let killable_pid_rss = sorted_pid_rss
            .iter()
            // Don't kill new tab
            // .filter(|pid| {
            //     if let Some(tab_info) = status.tab_infos.get(pid) {
            //         tab_info.title != "New Tab"
            //     } else {
            //         false
            //     }
            // })
            // Don't kill audible tab
            .filter(|(pid, _)| {
                if config.whitelist_audible {
                    if let Some(tab_info) = status.tab_infos.get(pid) {
                        !tab_info.audible
                    } else {
                        true
                    }
                } else {
                    true
                }
            });

        // Get pids to kill
        let exceed_rss = total_rss - config.strategy.rss_limit.max_bytes;
        let mut expected_freed_rss = 0;
        let mut killing_pids = Vec::new();
        for &(pid, rss) in killable_pid_rss.rev() {
            if exceed_rss >= expected_freed_rss {
                let url = status.tab_infos[&pid].url.clone();
                let in_whitelist = config.whitelist.iter().any(|regex| regex.is_match(&url));
                if in_whitelist {
                    continue;
                }

                expected_freed_rss += rss;
                killing_pids.push(pid);
            } else {
                break;
            }
        }

        // Kill
        killing_pids.iter().for_each(|pid| {
            let signal = Signal::Term;
            if let Some(process) = status.system.processes().get(pid) {
                match process.kill_with(signal) {
                    Some(success) => {
                        if !success {
                            eprintln!("Failed to send signal {} to {},", signal, pid);
                        }
                    }
                    None => {
                        eprintln!("The signal {} is not supported on this platform!", signal);
                    }
                }
            }
        })
    }
}

/// Will not kill new tab, because the last_access_time is wrong
fn kill_tabs_by_background_time_limit(status: &mut Status, config: &Config) {
    let signal = Signal::Term;
    status
        .begin_background_timestamps
        .iter()
        // Only left those tabs which in background too long
        .filter(|(_, &begin_background_timestamp)| {
            Duration::from_secs_f64((status.timestamp - begin_background_timestamp) / 1000.0)
                > Duration::from_secs_f64(config.strategy.background_time_limit.max_secs)
        })
        .map(|(pid, _)| pid)
        // Don't kill new tab
        .filter(|pid| {
            if let Some(tab_info) = status.tab_infos.get(pid) {
                tab_info.title != "New Tab"
            } else {
                false
            }
        })
        // Don't kill audible tab
        .filter(|pid| {
            if config.whitelist_audible {
                if let Some(tab_info) = status.tab_infos.get(pid) {
                    !tab_info.audible
                } else {
                    true
                }
            } else {
                true
            }
        })
        // Kill
        .for_each(|pid| {
            if let Some(process) = status.system.processes().get(pid) {
                process.kill_with(signal);
            }
        });
}

fn kill_tabs_by_cpu_idle_time_limit(status: &mut Status, config: &Config) {
    let signal = Signal::Term;
    status
        .begin_cpu_idle_timestamps
        .iter()
        // Only left those tabs which cpu idle too long
        .filter(|(_, &begin_cpu_idle_timestamp)| {
            Duration::from_secs_f64((status.timestamp - begin_cpu_idle_timestamp) / 1000.0)
                > Duration::from_secs_f64(config.strategy.cpu_idle_time_limit.max_secs)
        })
        .map(|(pid, _)| pid)
        // Don't kill new tab
        // .filter(|pid| {
        //     if let Some(tab_info) = status.tab_infos.get(pid) {
        //         tab_info.title != "New Tab"
        //     } else {
        //         false
        //     }
        // })
        // Don't kill audible tab
        .filter(|pid| {
            if config.whitelist_audible {
                if let Some(tab_info) = status.tab_infos.get(pid) {
                    !tab_info.audible
                } else {
                    true
                }
            } else {
                true
            }
        })
        // Kill
        .for_each(|pid| {
            if let Some(process) = status.system.processes().get(pid) {
                if let Some(tab_info) = status.tab_infos.get(pid) {
                    if !tab_info.active {
                        process.kill_with(signal);
                    }
                }
            }
        });
}

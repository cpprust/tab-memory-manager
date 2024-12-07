use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    thread::{sleep, spawn, JoinHandle},
    time::Duration,
};

use sysinfo::{Pid, Signal, System};
use thousands::Separable;

use crate::{
    config::{Config, KillTabStrategy},
    tab_infos_listener::TabInfo,
    BROWSER_NAME,
};

pub fn spawn_tab_killer_thread(
    tab_infos: Arc<Mutex<HashMap<Pid, TabInfo>>>,
    config: Config,
) -> JoinHandle<()> {
    spawn(move || {
        let tick = Duration::from_secs_f32(config.check_interval_secs);
        loop {
            kill_tabs_by_strategy(tab_infos.clone(), &config);

            sleep(tick);
        }
    })
}

fn kill_tabs_by_strategy(tab_infos: Arc<Mutex<HashMap<Pid, TabInfo>>>, config: &Config) {
    let system = System::new_all();
    let browser_process_count = system
        .processes_by_exact_name(BROWSER_NAME.as_ref())
        .count();
    if browser_process_count == 0 {
        // Browser is not opened, clear the stat
        tab_infos.lock().unwrap().clear();
    }

    println!("{:?}", tab_infos.lock().unwrap());
    let total_rss: u64 = tab_infos
        .lock()
        .unwrap()
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

    if tab_infos.lock().unwrap().len() > 0 {
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
                        let mut sorted_pid_rss: Vec<(Pid, Rss)> = tab_infos
                            .lock()
                            .unwrap()
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
                                expected_freed_rss += rss;
                                killing_pids.push(pid);
                            }
                        }

                        killing_pids.iter().for_each(|pid| {
                            let signal = Signal::Term;
                            match system.processes()[pid].kill_with(signal) {
                                Some(success) => {
                                    if !success {
                                        eprintln!("Failed to send signal {} to {},", signal, pid);
                                    }
                                }
                                None => {
                                    eprintln!(
                                        "The signal {} is not supported on this platform!",
                                        signal
                                    );
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

    println!();
}

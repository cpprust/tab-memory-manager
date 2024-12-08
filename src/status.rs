use std::collections::HashMap;

use sysinfo::{Pid, System};

use crate::{
    tab_data_requester::{TabInfo, Timestamp},
    tab_killer::Rss,
    BROWSER_NAME,
};

/// App status that must be sharing between threads
#[derive(Debug, Default)]
pub struct Status {
    pub system: System,
    pub timestamp: f64,
    pub tab_infos: HashMap<Pid, TabInfo>,
    pub begin_background_timestamps: HashMap<Pid, Timestamp>,
    pub begin_cpu_idle_timestamps: HashMap<Pid, Timestamp>,
}

impl Status {
    pub fn update(&mut self, new_tab_infos: HashMap<Pid, TabInfo>, timestamp: Timestamp) {
        self.tab_infos = new_tab_infos;
        self.timestamp = timestamp;
        // Clear stat if browser closed
        let browser_process_count = self
            .system
            .processes_by_exact_name(BROWSER_NAME.as_ref())
            .count();
        if browser_process_count == 0 {
            self.tab_infos.clear();
        }

        // Update begin_background_timestamps
        let data_timestamp = self.timestamp;
        let mut last_access_timestamps: HashMap<Pid, Timestamp> = self
            .tab_infos
            .iter()
            .filter(|(_, tab_info)| tab_info.title != "New Tab")
            .map(|(&pid, tab_info)| {
                if tab_info.active {
                    (pid, data_timestamp)
                } else {
                    (pid, tab_info.last_accessed)
                }
            })
            .collect();
        last_access_timestamps
            .iter_mut()
            .for_each(|(pid, last_accessd_time)| {
                if let Some(&begin_background_timestamp) = self.begin_background_timestamps.get(pid)
                {
                    *last_accessd_time = last_accessd_time.max(begin_background_timestamp);
                }
            });
        self.begin_background_timestamps = last_access_timestamps;

        // Update begin_cpu_idle_timestamps
        let new_begin_cpu_idle_timestamps: HashMap<Pid, Timestamp> = self
            .tab_infos
            .keys()
            .filter_map(|&pid| match self.begin_cpu_idle_timestamps.get(&pid) {
                Some(&old_begin_cpu_idle_timestamp) => {
                    if let Some(process) = self.system.processes().get(&pid) {
                        if process.cpu_usage() == 0.0 {
                            Some((pid, old_begin_cpu_idle_timestamp))
                        } else {
                            Some((pid, self.timestamp))
                        }
                    } else {
                        None
                    }
                }
                None => Some((pid, self.timestamp)),
            })
            .collect();
        self.begin_cpu_idle_timestamps = new_begin_cpu_idle_timestamps;
    }

    pub fn get_sorted_pid_rss(&self) -> Vec<(Pid, Rss)> {
        // The background tab processes pid and rss, sorted by rss
        let mut sorted_pid_rss: Vec<(Pid, u64)> = self
            .tab_infos
            .iter()
            .filter(|(_pid, tab_info)| !tab_info.active)
            .filter_map(
                |(&pid, _tab_info)| match self.system.processes().get(&pid) {
                    Some(process) => Some((pid, process.memory())),
                    None => None,
                },
            )
            .collect();
        sorted_pid_rss.sort_unstable_by_key(|&(_, rss)| rss);
        sorted_pid_rss
    }
}

use std::collections::HashMap;

use sysinfo::{Pid, System};

use crate::{
    tab_data_requester::{TabInfo, Timestamp},
    tab_killer::Rss,
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

use std::{
    collections::HashMap,
    net::ToSocketAddrs,
    sync::{Arc, Mutex},
    thread::{spawn, JoinHandle},
};

use astra::{Body, Response, Server};
use serde::{Deserialize, Serialize};
use sysinfo::{Pid, System};

use crate::tab_infos_listener::TabInfo;

/// Carrying minimal tab information, for sharing to frontend
#[derive(Debug, Deserialize, Serialize)]
struct MiniTabInfo {
    active: bool,
    pid: u32,
    // Resident set size
    rss: u64,
    title: String,
    cpu_usage: f32,
}

pub fn spawn_mini_tab_infos_server(tab_infos: Arc<Mutex<HashMap<Pid, TabInfo>>>) -> JoinHandle<()> {
    spawn(move || {
        let addr = "127.0.0.1:5000";
        serve_mini_tab_info(tab_infos, addr);
    })
}

fn serve_mini_tab_info(tab_infos: Arc<Mutex<HashMap<Pid, TabInfo>>>, addr: impl ToSocketAddrs) {
    let system = Mutex::new(System::new_all());
    Server::bind(addr)
        .serve(move |_, _| {
            system.lock().unwrap().refresh_all();
            // Get each minimum tab info
            let mini_tab_infos: Vec<MiniTabInfo> = (*tab_infos.lock().unwrap())
                .iter()
                .map(|(pid, tab_info)| {
                    if let Some(process) = system.lock().unwrap().processes().get(pid) {
                        Some(MiniTabInfo {
                            title: tab_info.title.clone(),
                            pid: pid.as_u32(),
                            rss: process.memory(),
                            active: tab_info.active,
                            cpu_usage: process.cpu_usage(),
                        })
                    } else {
                        None
                    }
                })
                .filter_map(|mini_tab_info| mini_tab_info)
                .collect();
            let json = serde_json::to_string(&mini_tab_infos).unwrap();
            Response::new(Body::new(json))
        })
        .unwrap();
}

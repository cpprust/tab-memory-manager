use std::{
    net::ToSocketAddrs,
    sync::{Arc, Mutex},
    thread::{spawn, JoinHandle},
};

use astra::{Body, Response, Server};
use serde::{Deserialize, Serialize};
use sysinfo::System;

use crate::Status;

/// The data is for sharing to frontend
#[derive(Debug, Deserialize, Serialize)]
struct OutputTabData {
    last_update_timestamp: f64,
    tab_infos: Vec<OutputTabInfo>,
}

#[derive(Debug, Deserialize, Serialize)]
struct OutputTabInfo {
    active: bool,
    pid: u32,
    // Resident set size
    rss: u64,
    title: String,
    cpu_usage: f32,
}

pub fn spawn_output_tab_data_server(status: Arc<Mutex<Status>>) -> JoinHandle<()> {
    spawn(move || {
        let addr = "127.0.0.1:5000";
        serve_output_tab_data(status, addr);
    })
}

fn serve_output_tab_data(status: Arc<Mutex<Status>>, addr: impl ToSocketAddrs) {
    // Must ramain stat for statistics cpu_usage
    let system = Mutex::new(System::new_all());
    Server::bind(addr)
        .serve(move |_, _| {
            system.lock().unwrap().refresh_all();
            // Get each minimum tab info
            let output_tab_infos: Vec<OutputTabInfo> = (*status.lock().unwrap())
                .tab_infos
                .iter()
                .map(|(pid, tab_info)| {
                    if let Some(process) = system.lock().unwrap().processes().get(pid) {
                        Some(OutputTabInfo {
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
            let output_tab_data = OutputTabData {
                last_update_timestamp: status.lock().unwrap().last_update_timestamp,
                tab_infos: output_tab_infos,
            };
            let json = serde_json::to_string(&output_tab_data).unwrap();
            Response::new(Body::new(json))
        })
        .unwrap();
}

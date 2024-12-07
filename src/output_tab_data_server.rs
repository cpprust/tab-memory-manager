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
    timestamp: f64,
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
    idle_time_secs: f64,
}

pub fn spawn_output_tab_data_server(status: Arc<Mutex<Status>>) -> JoinHandle<()> {
    spawn(move || {
        let addr = "127.0.0.1:60001";
        serve_output_tab_data(status, addr);
    })
}

fn serve_output_tab_data(status: Arc<Mutex<Status>>, addr: impl ToSocketAddrs) {
    // Must ramain stat for statistics cpu_usage
    let system = Mutex::new(System::new_all());
    Server::bind(addr)
        .serve(move |_, _| {
            let output_tab_data = generate_output_tab_data(&status, &system);
            let json = serde_json::to_string(&output_tab_data).unwrap();
            Response::new(Body::new(json))
        })
        .unwrap();
}

fn generate_output_tab_data(status: &Arc<Mutex<Status>>, system: &Mutex<System>) -> OutputTabData {
    let output_tab_infos = generate_output_tab_infos(status, system);
    OutputTabData {
        timestamp: status.lock().unwrap().timestamp,
        tab_infos: output_tab_infos,
    }
}

fn generate_output_tab_infos(
    status: &Arc<Mutex<Status>>,
    system: &Mutex<System>,
) -> Vec<OutputTabInfo> {
    system.lock().unwrap().refresh_all();
    // Get each minimum tab info
    let status = status.lock().unwrap();
    status
        .tab_infos
        .iter()
        .map(|(pid, tab_info)| {
            if let (Some(process), Some(begin_idle_timestamp)) = (
                system.lock().unwrap().processes().get(pid),
                status.begin_idle_timestamps.get(pid),
            ) {
                Some(OutputTabInfo {
                    title: tab_info.title.clone(),
                    pid: pid.as_u32(),
                    rss: process.memory(),
                    active: tab_info.active,
                    cpu_usage: process.cpu_usage(),
                    idle_time_secs: if tab_info.active {
                        0.0
                    } else {
                        (status.timestamp - begin_idle_timestamp) / 1000.0
                    },
                })
            } else {
                None
            }
        })
        .filter_map(|mini_tab_info| mini_tab_info)
        .collect()
}

mod config;
mod mini_tab_infos_server;
mod tab_infos_listener;
mod tab_killer;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use config::create_or_new_config;
use mini_tab_infos_server::spawn_output_tab_data_server;
use sysinfo::Pid;
use tab_infos_listener::{spawn_tab_data_listener, TabInfo};
use tab_killer::spawn_tab_killer_thread;

const PROJECT_NAME: &str = "tab-memory-manager";
const BROWSER_NAME: &str = "chromium";

/// App status that must be sharing between threads
#[derive(Debug, Default)]
struct Status {
    last_update_timestamp: f64,
    tab_infos: HashMap<Pid, TabInfo>,
}

fn main() {
    let config = create_or_new_config();

    // Sharing tab information between threads
    let status = Arc::new(Mutex::new(Status::default()));

    // Waiting for json data and update tab_infos, bind on 127.0.0.1:8080
    let tab_data_listener = spawn_tab_data_listener(Arc::clone(&status));

    // Terminate tab by given strategy
    let _tab_killer = spawn_tab_killer_thread(Arc::clone(&status), config.clone());

    // Sharing vec of MiniTabInfo in json format, bind on 127.0.0.1:5000
    let _mini_tab_data_server = spawn_output_tab_data_server(Arc::clone(&status));

    tab_data_listener.join().unwrap();
}

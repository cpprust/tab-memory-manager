mod config;
mod mini_tab_infos_server;
mod tab_infos_listener;
mod tab_killer;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use config::create_or_new_config;
use mini_tab_infos_server::spawn_mini_tab_infos_server;
use sysinfo::Pid;
use tab_infos_listener::{spawn_tab_infos_listener, TabInfo};
use tab_killer::spawn_tab_killer_thread;

const PROJECT_NAME: &str = "tab-memory-manager";
const BROWSER_NAME: &str = "chromium";

fn main() {
    let config = create_or_new_config();

    // Sharing tab information between threads
    let tab_infos = Arc::new(Mutex::new(HashMap::<Pid, TabInfo>::new()));

    // Waiting for json data and update tab_infos, bind on 127.0.0.1:8080
    let tab_infos_reciever = spawn_tab_infos_listener(Arc::clone(&tab_infos));

    // Terminate tab by given strategy
    let _tab_killer = spawn_tab_killer_thread(Arc::clone(&tab_infos), config.clone());

    // Sharing vec of MiniTabInfo in json format, bind on 127.0.0.1:5000
    let _mini_tab_infos_server = spawn_mini_tab_infos_server(Arc::clone(&tab_infos));

    tab_infos_reciever.join().unwrap();
}

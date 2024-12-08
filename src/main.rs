mod config;
mod output_tab_data_server;
mod status;
mod tab_data_requester;
mod tab_killer;

use std::sync::{mpsc::sync_channel, Arc, Mutex};

use config::create_or_new_config;
use output_tab_data_server::spawn_output_tab_data_server;
use status::Status;
use tab_data_requester::spawn_tab_data_requester;
use tab_killer::spawn_tab_killer_thread;

const PROJECT_NAME: &str = "tab-memory-manager";

fn main() {
    let config = create_or_new_config();

    // Sharing tab information between threads
    let status = Arc::new(Mutex::new(Status::default()));
    // Request update status from browser extension if possible
    let (update_req_sender, update_req_reciever) = sync_channel::<()>(1);
    let (update_result_sender, update_result_reciever) = sync_channel::<Result<(), String>>(1);

    // Waiting for json data and update tab_infos, bind on ws://127.0.0.1:60000
    let tab_data_requester = spawn_tab_data_requester(
        Arc::clone(&status),
        update_req_reciever,
        update_result_sender,
        config.browser_name.clone(),
    );

    // Terminate tab by given strategy
    let _tab_killer = spawn_tab_killer_thread(
        Arc::clone(&status),
        config,
        update_req_sender,
        update_result_reciever,
    );

    // Sharing vec of MiniTabInfo in json format, bind on http://127.0.0.1:60001
    let _mini_tab_data_server = spawn_output_tab_data_server(Arc::clone(&status));

    tab_data_requester.join().unwrap();
}

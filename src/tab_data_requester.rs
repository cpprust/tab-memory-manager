use std::{
    collections::{HashMap, HashSet},
    sync::{
        mpsc::{Receiver, SyncSender},
        Arc, Mutex,
    },
    thread::{spawn, JoinHandle},
};

use debug_print::debug_println;
use serde::{Deserialize, Serialize};
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate};
use ws::{listen, Message};

use crate::{status::Status, BROWSER_NAME};

pub type BrowserInnerPid = u64;
pub type Timestamp = f64;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TabData {
    timestamp: Timestamp,
    tab_infos: Vec<TabInfo>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TabInfo {
    pub active: bool,
    pub audible: bool,
    pub auto_discardable: bool,
    pub discarded: bool,
    pub fav_icon_url: Option<String>,
    pub group_id: i32,
    pub height: u32,
    pub highlighted: bool,
    pub id: u64,
    pub incognito: bool,
    pub index: u32,
    pub last_accessed: Timestamp,
    pub muted_info: MutedInfo,
    pub pinned: bool,
    pub selected: bool,
    pub status: String,
    pub title: String,
    pub url: String,
    pub width: u32,
    pub window_id: usize,
    pub browser_inner_pid: BrowserInnerPid,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MutedInfo {
    pub muted: bool,
}

pub fn spawn_tab_data_requester(
    status: Arc<Mutex<Status>>,
    update_req_reciever: Receiver<()>,
    update_result_sender: SyncSender<Result<(), String>>,
) -> JoinHandle<()> {
    spawn(move || {
        request_tab_data_from_browser_and_update_status(
            status,
            update_req_reciever,
            update_result_sender,
        )
    })
}

fn request_tab_data_from_browser_and_update_status(
    status: Arc<Mutex<Status>>,
    update_req_reciever: Receiver<()>,
    update_result_sender: SyncSender<Result<(), String>>,
) {
    let update_req_reciever = Arc::new(Mutex::new(update_req_reciever));
    listen("127.0.0.1:60000", move |ws_msg_sender| {
        debug_println!("New sender: {:?}", ws_msg_sender);
        let update_req_reciever = update_req_reciever.clone();
        spawn(move || {
            while let Ok(_) = update_req_reciever.lock().unwrap().recv() {
                debug_println!("Requesting tab data from browser extension");
                // Send arbitrary data to tell extension to send json back
                ws_msg_sender.broadcast(Message::binary(b"")).unwrap();
                debug_println!("Finish requesting tab data from browser extension");
            }
        });
        let status = Arc::clone(&status);
        let update_result_sender = update_result_sender.clone();
        move |ws_msg| {
            debug_println!("Recieved a ws_msg!");
            if let Message::Text(msg) = ws_msg {
                match serde_json::from_str::<TabData>(&msg) {
                    Ok(input_tab_data) => {
                        let status = &mut status.lock().unwrap();
                        // status.system.refresh_all();
                        status.system.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::everything());
                        // status.system.refresh_processes(ProcessesToUpdate::All, true);
                        status.system.refresh_memory();
                        status.system.refresh_cpu_all();

                        // If given tab infos inner pid are the same as last time, use the old pid map
                        let last_loop_browser_inner_pids: HashSet<BrowserInnerPid> = status.tab_infos.values().map(|tab_info| tab_info.browser_inner_pid).collect();
                        let same_tabs_as_last_update = input_tab_data.tab_infos.iter().all(|recieved_tab_info| last_loop_browser_inner_pids.contains(&recieved_tab_info.browser_inner_pid)) && input_tab_data.tab_infos.len() == last_loop_browser_inner_pids.len();
                        let browser_inner_pid_to_pid: HashMap<BrowserInnerPid, Pid> = if same_tabs_as_last_update {
                             status.tab_infos.iter().map(|(pid, tab_info)| (tab_info.browser_inner_pid, *pid)).collect()
                        } else {
                            // Get new pid map
                            let mut browser_inner_pid_to_pid: HashMap<BrowserInnerPid, Pid> =
                                HashMap::new();
                            for process in status.system.processes_by_exact_name(BROWSER_NAME.as_ref()) {
                                let cmdline = match process.cmd().first() {
                                    Some(cmdline) => cmdline,
                                    None => {
                                        eprintln!("Process {} cmdline is empty!", process.pid());
                                        continue;
                                    }
                                };
                                let cmdline = match cmdline.to_str() {
                                    Some(cmdline) => cmdline,
                                    None => {
                                        eprintln!(
                                            "Process {} cmdline have invalid UTF-8 data: {:?}",
                                            process.pid(),
                                            cmdline
                                        );
                                        continue;
                                    }
                                };
                                let target_arg = cmdline
                                    .split_whitespace()
                                    .filter(|arg| arg.starts_with("--renderer-client-id="))
                                    .next();
                                let target_arg = match target_arg {
                                    Some(target_arg) => target_arg,
                                    None => {
                                        // No target flag in this cmdline, skipped
                                        continue;
                                    }
                                };
                                let browser_inner_pid = target_arg
                                    .split('=')
                                    .nth(1);
                                let browser_inner_pid = match browser_inner_pid {
                                    Some(browser_inner_pid) => {
                                        browser_inner_pid.parse::<BrowserInnerPid>()
                                    }
                                    None => {
                                        eprintln!("Process {}, no number after arg \"renderer-client-id=\", cmdline: {}", process.pid(), cmdline);
                                        continue;
                                    },
                                };
                                let browser_inner_pid = match browser_inner_pid {
                                    Ok(browser_inner_pid) => browser_inner_pid,
                                    Err(e) => {
                                        eprintln!("Cannot find pid from cmdline arg: {}", e);
                                        continue;
                                    }
                                };
                                browser_inner_pid_to_pid.insert(browser_inner_pid, process.pid());
                            }
                            browser_inner_pid_to_pid
                        };

                        // Update tab infos
                        let mut new_tab_infos = HashMap::<Pid, TabInfo>::new();
                        for tab_info in input_tab_data.tab_infos {
                            match browser_inner_pid_to_pid.get(&tab_info.browser_inner_pid) {
                                Some(&pid) => {
                                    if status.system.processes().contains_key(&pid) {
                                        new_tab_infos.insert(pid, tab_info);
                                    }
                                },
                                None => continue,
                            };
                        }
                        status.update(new_tab_infos, input_tab_data.timestamp);
                        let _ = update_result_sender.try_send(Ok(()));
                    }
                    Err(e) => {
                        eprintln!("Failed to parse json: {e}\nError data: {msg}");
                        update_result_sender.try_send(Err(format!("Failed to parse json: {e}\nError data: {msg}"))).expect("Failed to send status update result!");
                    }
                }
            } else {
                eprintln!("Error, recieved binary message!");
            }
            Ok(())
        }
    })
    .unwrap();
}

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    thread::{spawn, JoinHandle},
};

use serde::{Deserialize, Serialize};
use sysinfo::{Pid, System};
use ws::{listen, Message};

use crate::BROWSER_NAME;

pub type BrowserInnerPid = u64;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MutedInfo {
    pub muted: bool,
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
    pub last_accessed: f64,
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

pub fn spawn_tab_infos_listener(tab_infos: Arc<Mutex<HashMap<Pid, TabInfo>>>) -> JoinHandle<()> {
    spawn(move || listen_for_tab_infos(tab_infos))
}

fn listen_for_tab_infos(tab_infos: Arc<Mutex<HashMap<Pid, TabInfo>>>) {
    listen("127.0.0.1:8080", move |_| {
            let tab_infos = Arc::clone(&tab_infos); // todo!
            move |msg| {
                if let Message::Text(msg) = msg {
                    match serde_json::from_str::<Vec<TabInfo>>(&msg) {
                        Ok(recieved_tab_infos) => {
                            let system = System::new_all();

                            // Get pid map
                            let mut browser_inner_pid_to_pid: HashMap<BrowserInnerPid, Pid> =
                                HashMap::new();
                            for process in system.processes_by_exact_name(BROWSER_NAME.as_ref()) {
                                let cmdline = match process.cmd().first() {
                                    Some(cmdline) => cmdline,
                                    None => {
                                        eprintln!("Process {} cmdline of is empty!", process.pid());
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

                            // Update tab infos
                            let mut new_tab_infos = HashMap::<Pid, TabInfo>::new();
                            for tab_info in recieved_tab_infos {
                                let pid = browser_inner_pid_to_pid.get(&tab_info.browser_inner_pid);
                                let pid = match pid {
                                    Some(pid) => *pid,
                                    None => continue,
                                };
                                new_tab_infos.insert(pid, tab_info);
                            }
                            *tab_infos.lock().unwrap() = new_tab_infos;
                        }
                        Err(e) => eprintln!("Failed to parse json: {e}\nError data: {msg}"),
                    }
                } else {
                    eprintln!("Recieved binary message!");
                }
                Ok(())
            }
        })
        .unwrap();
}

let ws;

function initWs() {
  ws = new WebSocket("ws://127.0.0.1:8080");
}

async function getTabInfos() {
  let tabInfos = await chrome.tabs.query({});
  await Promise.all(tabInfos.map(async tabInfo => {
    tabInfo.browserInnerPid = await chrome.processes.getProcessIdForTab(tabInfo.id);
    return tabInfo;
  }));

  return {
    timestamp: Date.now(),
    tabInfos,
  };
}

async function sendObjAsJson(obj) {
  let json = JSON.stringify(obj, null, 0);

  if (ws && ws.readyState === WebSocket.OPEN) {
    ws.send(json);
  } else {
    ws.close();
    initWs();
    console.error("WebSocket is not opened yet, cannot send json!");
  }
}

initWs();

chrome.tabs.onUpdated.addListener(async (_tabId, changeInfo, tab) => {
  if (changeInfo.status === "complete" && tab.title) {
    sendObjAsJson(await getTabInfos());
  }
});

chrome.tabs.onActivated.addListener(async (_activeInfo) => {
  sendObjAsJson(await getTabInfos());
});

chrome.tabs.onRemoved.addListener(async (_tabId, _removeInfo) => {
  sendObjAsJson(await getTabInfos());
});

setInterval(async () => {
  sendObjAsJson(await getTabInfos());
}, 1000);

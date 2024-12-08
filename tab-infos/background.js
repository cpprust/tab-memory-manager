const serverUrl = 'ws://localhost:60000';
let ws;
let reconnectInterval = 100;

async function getTabData() {
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

function initWs() {
  ws = new WebSocket(serverUrl);

  ws.addEventListener('open', async (_event) => {
    console.log('Connected to the WebSocket server!');

    let tabData = await getTabData();
    let json = JSON.stringify(tabData);
    ws.send(json);
  });

  ws.addEventListener('message', async (event) => {
    console.log(`Message from server: ${event.data}`);

    let tabData = await getTabData();
    let json = JSON.stringify(tabData);
    ws.send(json);
  });

  ws.addEventListener('error', (event) => {
    console.error('WebSocket error:', event);
  });

  ws.addEventListener('close', (_event) => {
    console.log('Disconnected from the WebSocket server!');

    setTimeout(() => {
      // console.error('Try to reconnect to WebSocket server...');
      initWs();
    }, reconnectInterval);
  });
}

initWs();

// Function to keep the service worker alive
// https://stackoverflow.com/a/66618269
const keepAlive = () => {
    // Call an asynchronous chrome api every 20 seconds
    setInterval(() => {
        chrome.runtime.getPlatformInfo();
    }, 20000);
};

chrome.runtime.onStartup.addListener(keepAlive);
chrome.runtime.onInstalled.addListener(keepAlive);
keepAlive();


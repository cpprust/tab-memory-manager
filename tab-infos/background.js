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
    console.error('Connected to the WebSocket server!');

    let tabData = await getTabData();
    let json = JSON.stringify(tabData);
    ws.send(json);
  });

  ws.addEventListener('message', async (event) => {
    console.error(`Message from server: ${event.data}`);

    let tabData = await getTabData();
    let json = JSON.stringify(tabData);
    ws.send(json);
  });

  ws.addEventListener('error', (event) => {
    console.error('WebSocket error:', event);
  });

  ws.addEventListener('close', (_event) => {
    console.error('Disconnected from the WebSocket server!');

    setTimeout(() => {
      // console.error('Try to reconnect to WebSocket server...');
      initWs();
    }, reconnectInterval);
  });
}

initWs();


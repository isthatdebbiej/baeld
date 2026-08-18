import http from "node:http";
import { WebSocketServer } from "ws";

const port = Number(process.env.PORT ?? 4173);
const states = new Map();
setInterval(() => {
  for (const state of states.values()) state.sequence++;
}, 250);

function stateFor(session) {
  if (!states.has(session)) states.set(session, { updates: 0, value: "initial", sequence: 0 });
  return states.get(session);
}

const pages = {
  static: `<!doctype html><meta charset="utf-8"><title>Baeld static</title>
    <h1 id="oracle">BAELD_STATIC_OK</h1><p>This page is the negative control.</p>`,
  "normal-spa": `<!doctype html><meta charset="utf-8"><title>Baeld normal SPA</title>
    <h1>Account settings</h1><output id="clock"></output><p id="server-value">loading</p>
    <button id="save">Save exactly once</button><script>
    const session = new URLSearchParams(location.search).get('session');
    window.__baeldAgent = {refreshes:0};
    let ticks = 0;
    setInterval(() => { ticks++; document.querySelector('#clock').textContent = String(ticks); }, 250);
    setInterval(async () => {
      const r = await fetch('/api/state?session=' + encodeURIComponent(session));
      const s = await r.json(); document.querySelector('#server-value').textContent = s.value;
    }, 750);
    document.querySelector('#save').onclick = async () => {
      await fetch('/api/mutate?session=' + encodeURIComponent(session), {method:'POST'});
    };
    </script>`,
  "agent-dashboard": `<!doctype html><meta charset="utf-8"><title>Baeld agent dashboard</title>
    <h1>Operations dashboard</h1><p id="summary">loading</p><table><tbody id="rows"></tbody></table>
    <button id="save">Save exactly once</button><script>
    const session = new URLSearchParams(location.search).get('session');
    const formatter = new Intl.NumberFormat('en-US', {style:'currency', currency:'USD'});
    async function refresh() {
      const response = await fetch('/api/dashboard?session=' + encodeURIComponent(session));
      const payload = await response.json();
      const visible = payload.records
        .filter(record => record.status !== 'archived')
        .sort((left, right) => right.updated_at - left.updated_at)
        .slice(0, 60);
      const fragment = document.createDocumentFragment();
      for (const record of visible) {
        const row = document.createElement('tr');
        row.dataset.id = record.id;
        row.innerHTML = '<td>' + record.name + '</td><td>' + record.status +
          '</td><td>' + formatter.format(record.amount) + '</td>';
        fragment.appendChild(row);
      }
      document.querySelector('#rows').replaceChildren(fragment);
      document.querySelector('#summary').textContent = payload.sequence + ':' + visible.length;
      localStorage.setItem('baeld-dashboard-sequence', String(payload.sequence));
      window.__baeldAgent.refreshes++;
    }
    refresh();
    setInterval(refresh, 500);
    document.querySelector('#save').onclick = () => fetch('/api/mutate?session=' + encodeURIComponent(session), {method:'POST'});
    </script>`,
  "noisy-stress": `<!doctype html><meta charset="utf-8"><title>Baeld stress</title>
    <h1>Stress workload</h1><output id="counter"></output><button id="save">Save exactly once</button>
    <script>
    const session = new URLSearchParams(location.search).get('session'); let counter = 0;
    setInterval(() => {
      let x = 0; for (let i=0; i<180000; i++) x = (x + Math.sqrt(i + counter)) % 100000;
      counter++; document.querySelector('#counter').textContent = counter + ':' + x.toFixed(2);
    }, 100);
    setInterval(() => fetch('/api/state?session=' + encodeURIComponent(session)), 500);
    document.querySelector('#save').onclick = () => fetch('/api/mutate?session=' + encodeURIComponent(session), {method:'POST'});
    </script>`,
  websocket: `<!doctype html><meta charset="utf-8"><title>Baeld WebSocket</title>
    <h1>Realtime workload</h1><output id="sequence">0</output><button id="save">Save exactly once</button>
    <script>
    const session = new URLSearchParams(location.search).get('session');
    window.__baeld = {sequences:[], reconnects:0, opens:0};
    function connect() {
      const ws = new WebSocket('ws://' + location.host + '/ws?session=' + encodeURIComponent(session));
      ws.onopen = () => { window.__baeld.opens++; if (window.__baeld.opens > 1) window.__baeld.reconnects++; };
      ws.onmessage = e => { const n=Number(e.data); window.__baeld.sequences.push(n); document.querySelector('#sequence').textContent=n; };
      ws.onclose = () => setTimeout(connect, 100);
    }
    connect();
    document.querySelector('#save').onclick = () => fetch('/api/mutate?session=' + encodeURIComponent(session), {method:'POST'});
    </script>`
};

const server = http.createServer(async (request, response) => {
  const url = new URL(request.url, `http://${request.headers.host}`);
  if (request.method === "GET" && url.pathname === "/health") return json(response, 200, {ok:true});
  if (request.method === "POST" && url.pathname === "/api/reset") {
    states.set(url.searchParams.get("session"), {updates:0, value:"initial", sequence:0});
    return json(response, 200, {ok:true});
  }
  if (request.method === "POST" && url.pathname === "/api/mutate") {
    const state = stateFor(url.searchParams.get("session"));
    state.updates++; state.value = "persisted";
    return json(response, 200, state);
  }
  if (request.method === "GET" && url.pathname === "/api/state") {
    return json(response, 200, stateFor(url.searchParams.get("session")));
  }
  if (request.method === "GET" && url.pathname === "/api/dashboard") {
    const state = stateFor(url.searchParams.get("session"));
    const records = Array.from({length: 240}, (_, index) => ({
      id: index,
      name: "Account " + index,
      status: (index + state.sequence) % 11 === 0 ? "archived" : (index % 3 === 0 ? "pending" : "active"),
      amount: ((index * 7919 + state.sequence * 17) % 100000) / 100,
      updated_at: state.sequence * 1000 - ((index * 37) % 10000)
    }));
    return json(response, 200, {sequence:state.sequence, records});
  }
  const workload = url.pathname.slice(1) || "static";
  if (pages[workload]) {
    response.writeHead(200, {"content-type":"text/html; charset=utf-8", "cache-control":"no-store"});
    return response.end(pages[workload]);
  }
  response.writeHead(404); response.end("not found");
});

const wss = new WebSocketServer({ noServer: true });
server.on("upgrade", (request, socket, head) => {
  const url = new URL(request.url, `http://${request.headers.host}`);
  if (url.pathname !== "/ws") return socket.destroy();
  wss.handleUpgrade(request, socket, head, ws => {
    const session = url.searchParams.get("session");
    ws.isAlive = true;
    ws.on("pong", () => { ws.isAlive = true; });
    const timer = setInterval(() => {
      const state = stateFor(session); ws.send(String(state.sequence));
    }, 250);
    const heartbeat = setInterval(() => {
      if (!ws.isAlive) return ws.terminate();
      ws.isAlive = false; ws.ping();
    }, 500);
    ws.on("close", () => { clearInterval(timer); clearInterval(heartbeat); });
  });
});

function json(response, status, value) {
  response.writeHead(status, {"content-type":"application/json", "cache-control":"no-store"});
  response.end(JSON.stringify(value));
}

server.listen(port, "127.0.0.1", () => console.error(`Baeld workload server on ${port}`));

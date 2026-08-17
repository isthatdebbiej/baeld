import http from "node:http";

const port = Number(process.env.PORT ?? 47831);

const server = http.createServer(async (request, response) => {
  try {
    let upstream;
    if (request.url === "/config.json") {
      const original = await fetch("https://index.crates.io/config.json");
      const config = await original.json();
      config.dl = `http://127.0.0.1:${port}/crates`;
      return send(response, 200, JSON.stringify(config), "application/json");
    }
    if (request.url.startsWith("/crates/")) {
      upstream = `https://static.crates.io${request.url}`;
    } else {
      upstream = `https://index.crates.io${request.url}`;
    }
    const fetched = await fetch(upstream);
    const body = Buffer.from(await fetched.arrayBuffer());
    send(response, fetched.status, body, fetched.headers.get("content-type") ?? "application/octet-stream");
  } catch (error) {
    send(response, 502, String(error), "text/plain");
  }
});

function send(response, status, body, type) {
  response.writeHead(status, {"content-type":type, "cache-control":"no-store"});
  response.end(body);
}

server.listen(port, "127.0.0.1", () => console.error(`Cargo bridge listening on ${port}`));


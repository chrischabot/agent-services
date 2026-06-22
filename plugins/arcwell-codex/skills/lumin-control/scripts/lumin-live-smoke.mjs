#!/usr/bin/env node

import assert from "node:assert/strict";
import http from "node:http";
import { execFile } from "node:child_process";
import { promisify } from "node:util";

const SCRIPT = new URL("./lumin.mjs", import.meta.url).pathname;
const execFileAsync = promisify(execFile);

const services = [
  ["Product", "urn:av-openhome-org:service:Product:1"],
  ["Playlist", "urn:av-openhome-org:service:Playlist:1"],
  ["Volume", "urn:av-openhome-org:service:Volume:1"],
  ["Info", "urn:av-openhome-org:service:Info:1"],
  ["Time", "urn:av-openhome-org:service:Time:1"],
  ["AVTransport", "urn:schemas-upnp-org:service:AVTransport:1"],
  ["RenderingControl", "urn:schemas-upnp-org:service:RenderingControl:1"],
];

const actionInputs = {
  Product: {
    Standby: [],
    SetStandby: ["Value"],
    SourceXml: [],
    SourceIndex: [],
    SetSourceIndex: ["Value"],
    SetSource: ["SystemName"],
  },
  Playlist: {
    Id: [],
    IdArray: [],
    TransportState: [],
    Repeat: [],
    SetRepeat: ["Value"],
    Shuffle: [],
    SetShuffle: ["Value"],
    Read: ["Id"],
    ReadList: ["IdList"],
    Insert: ["AfterId", "Uri", "Metadata"],
    DeleteId: ["Value"],
    DeleteAll: [],
    Play: [],
    Pause: [],
    Stop: [],
    Next: [],
    Previous: [],
  },
  Volume: {
    Volume: [],
    SetVolume: ["Value"],
    Mute: [],
    SetMute: ["Value"],
  },
  Info: { Details: [] },
  Time: { Seconds: [] },
  AVTransport: {
    GetTransportInfo: ["InstanceID"],
    Play: ["InstanceID", "Speed"],
    Pause: ["InstanceID"],
    Stop: ["InstanceID"],
    Next: ["InstanceID"],
    Previous: ["InstanceID"],
  },
  RenderingControl: {
    GetVolume: ["InstanceID", "Channel"],
    GetMute: ["InstanceID", "Channel"],
    SetVolume: ["InstanceID", "Channel", "DesiredVolume"],
    SetMute: ["InstanceID", "Channel", "DesiredMute"],
  },
};

const calls = [];
const state = {
  standby: "false",
  volume: "22",
  mute: "false",
  repeat: "false",
  shuffle: "false",
  transportState: "Stopped",
  sourceIndex: "0",
};

function deviceXml(base) {
  return `<?xml version="1.0"?>
<root><device>
  <friendlyName>Mock LUMIN P1</friendlyName>
  <manufacturer>LUMIN</manufacturer>
  <modelName>P1</modelName>
  <modelNumber>mock</modelNumber>
  <UDN>uuid:mock-lumin-p1</UDN>
  <serviceList>
    ${services.map(([name, type]) => `<service>
      <serviceType>${type}</serviceType>
      <serviceId>urn:arcwell:mock:${name}</serviceId>
      <controlURL>/${name}/control</controlURL>
      <eventSubURL>/${name}/event</eventSubURL>
      <SCPDURL>/${name}/scpd.xml</SCPDURL>
    </service>`).join("\n")}
  </serviceList>
</device><URLBase>${base}</URLBase></root>`;
}

function scpdXml(name) {
  const actions = actionInputs[name] || {};
  return `<?xml version="1.0"?><scpd><actionList>
    ${Object.entries(actions).map(([action, inputs]) => `<action><name>${action}</name><argumentList>
      ${inputs.map((input) => `<argument><name>${input}</name><direction>in</direction></argument>`).join("")}
      <argument><name>Value</name><direction>out</direction></argument>
    </argumentList></action>`).join("\n")}
  </actionList></scpd>`;
}

function tag(xml, name) {
  const match = xml.match(new RegExp(`<${name}>([\\s\\S]*?)</${name}>`));
  return match ? match[1] : null;
}

function responseFor(service, action, body) {
  calls.push({ service, action, body });
  if (service === "Product") {
    if (action === "Standby") return valueResponse(action, state.standby);
    if (action === "SetStandby") { state.standby = tag(body, "Value") || state.standby; return valueResponse(action, state.standby); }
    if (action === "SourceIndex") return valueResponse(action, state.sourceIndex);
    if (action === "SetSourceIndex") { state.sourceIndex = tag(body, "Value") || state.sourceIndex; return valueResponse(action, state.sourceIndex); }
    if (action === "SetSource") { state.sourceIndex = "0"; return valueResponse(action, "playlist"); }
    if (action === "SourceXml") {
      return valueResponse(action, "&lt;SourceList&gt;&lt;Source Name=&quot;Playlist&quot; Type=&quot;Playlist&quot; Visible=&quot;true&quot; SystemName=&quot;playlist&quot;/&gt;&lt;Source Name=&quot;HDMI ARC&quot; Type=&quot;Digital&quot; Visible=&quot;true&quot; SystemName=&quot;hdmi-arc&quot;/&gt;&lt;/SourceList&gt;");
    }
  }
  if (service === "Playlist") {
    if (action === "Id") return valueResponse(action, "10");
    if (action === "IdArray") return valueResponse(action, "10 11");
    if (action === "TransportState") return valueResponse(action, state.transportState);
    if (action === "Repeat") return valueResponse(action, state.repeat);
    if (action === "SetRepeat") { state.repeat = tag(body, "Value") || state.repeat; return valueResponse(action, state.repeat); }
    if (action === "Shuffle") return valueResponse(action, state.shuffle);
    if (action === "SetShuffle") { state.shuffle = tag(body, "Value") || state.shuffle; return valueResponse(action, state.shuffle); }
    if (action === "Read") return valueResponse(action, "&lt;DIDL-Lite&gt;&lt;item id=&quot;10&quot;/&gt;&lt;/DIDL-Lite&gt;");
    if (action === "ReadList") return valueResponse(action, "&lt;TrackList&gt;10 11&lt;/TrackList&gt;");
    if (["Insert", "DeleteId", "DeleteAll"].includes(action)) return valueResponse(action, "ok");
    if (["Play", "Pause", "Stop", "Next", "Previous"].includes(action)) { state.transportState = action; return valueResponse(action, state.transportState); }
  }
  if (service === "Volume") {
    if (action === "Volume") return valueResponse(action, state.volume);
    if (action === "SetVolume") { state.volume = tag(body, "Value") || state.volume; return valueResponse(action, state.volume); }
    if (action === "Mute") return valueResponse(action, state.mute);
    if (action === "SetMute") { state.mute = tag(body, "Value") || state.mute; return valueResponse(action, state.mute); }
  }
  if (service === "Info" && action === "Details") return valueResponse(action, "&lt;details title=&quot;Mock&quot;/&gt;");
  if (service === "Time" && action === "Seconds") return valueResponse(action, "123");
  if (service === "AVTransport") return valueResponse(action, "ok");
  if (service === "RenderingControl") return valueResponse(action, "ok");
  return valueResponse(action, "ok");
}

function valueResponse(action, value) {
  return `<?xml version="1.0"?><s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/"><s:Body><u:${action}Response xmlns:u="urn:mock"><Value>${value}</Value></u:${action}Response></s:Body></s:Envelope>`;
}

const server = http.createServer(async (req, res) => {
  const base = `http://127.0.0.1:${server.address().port}`;
  if (req.method === "GET" && req.url === "/device.xml") {
    res.writeHead(200, { "Content-Type": "text/xml" });
    res.end(deviceXml(base));
    return;
  }
  const scpd = req.url?.match(/^\/([^/]+)\/scpd\.xml$/);
  if (req.method === "GET" && scpd) {
    res.writeHead(200, { "Content-Type": "text/xml" });
    res.end(scpdXml(scpd[1]));
    return;
  }
  const control = req.url?.match(/^\/([^/]+)\/control$/);
  if (req.method === "POST" && control) {
    let body = "";
    for await (const chunk of req) body += chunk.toString();
    const soapAction = String(req.headers.soapaction || "");
    const action = soapAction.split("#")[1]?.replace(/"/g, "") || "Unknown";
    res.writeHead(200, { "Content-Type": "text/xml" });
    res.end(responseFor(control[1], action, body));
    return;
  }
  res.writeHead(404, { "Content-Type": "text/plain" });
  res.end("not found");
});

await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));

try {
  const location = `http://127.0.0.1:${server.address().port}/device.xml`;
  const commands = [
    ["inspect", "--location", location, "--json"],
    ["services", "--location", location, "--json"],
    ["actions", "--location", location, "--service", "Product", "--json"],
    ["status", "--location", location, "--json"],
    ["sources", "--location", location, "--json"],
    ["power", "--location", location, "--get", "--json"],
    ["power", "--location", location, "--standby", "false", "--confirm-standby", "--json"],
    ["volume", "--location", location, "--get", "--json"],
    ["volume", "--location", location, "--set", "35", "--confirm-volume", "--json"],
    ["volume", "--location", location, "--mute", "true", "--json"],
    ["playback", "--location", location, "--action", "pause", "--json"],
    ["playlist", "--location", location, "--state", "--json"],
    ["playlist", "--location", location, "--read", "--id", "10", "--json"],
    ["playlist", "--location", location, "--read-list", "--ids", "10 11", "--json"],
    ["playlist", "--location", location, "--repeat", "true", "--json"],
    ["playlist", "--location", location, "--shuffle", "false", "--json"],
    ["playlist", "--location", location, "--insert-uri", "http://example.test/track.flac", "--after-id", "0", "--metadata", "<DIDL-Lite/>", "--confirm-playlist-write", "--json"],
    ["playlist", "--location", location, "--delete-id", "10", "--confirm-playlist-write", "--json"],
    ["playlist", "--location", location, "--clear", "--confirm-playlist-write", "--json"],
    ["select-source", "--location", location, "--system-name", "playlist", "--confirm-source-select", "--json"],
    ["soap", "--location", location, "--service", "Product", "--action", "SourceXml", "--json"],
    ["udp", "--host", "192.0.2.1", "--command", "pause", "--dry-run", "--json"],
  ];

  for (const args of commands) {
    const { stdout } = await execFileAsync(process.execPath, [SCRIPT, ...args], { encoding: "utf8" });
    assert.doesNotThrow(() => JSON.parse(stdout), `${args.join(" ")} did not emit JSON`);
  }

  assert(calls.some((call) => call.service === "Product" && call.action === "SourceXml"));
  assert(calls.some((call) => call.service === "Volume" && call.action === "SetVolume"));
  assert(calls.some((call) => call.service === "Playlist" && call.action === "Insert"));
  console.log(JSON.stringify({ ok: true, location, command_count: commands.length, soap_call_count: calls.length }, null, 2));
} finally {
  await new Promise((resolve) => server.close(resolve));
}

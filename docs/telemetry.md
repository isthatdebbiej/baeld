# Telemetry and data handling

Telemetry is local by default. It records session identifiers, phases, selected actions, health, resource counters, and diagnostics. It does not intentionally record page content, screenshots, prompts, model responses, cookies, or credentials.

Set `jsonl = false` to disable the local stream. Set `otel_endpoint` to a local `unix://` collector socket to send OTLP/HTTP JSON logs. The collector handles authentication, TLS, batching, and onward export. Baeld does not send telemetry directly to a network host, and no collector is contacted by default.

Baeld records only the command executable, not its arguments. Inspect diagnostics before sharing: executable paths, workload names, and errors can still reveal internal information.

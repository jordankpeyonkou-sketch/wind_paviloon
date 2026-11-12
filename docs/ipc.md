# WVX Runtime IPC (version 1)

The Studio communicates with the native runtime through the current Windows
user's named pipe:

```text
\\.\pipe\wvx-runtime-v1
```

Messages are UTF-8 JSON followed by one newline. A client opens a connection,
sends exactly one request, reads one response line, and closes the connection.
Requests are limited to 64 KiB; the runtime rejects unsupported protocol or
settings schema versions.

## Read settings

Request:

```json
{"version":1,"type":"get_settings"}
```

Success response:

```json
{"version":1,"ok":true,"runtime":"running","settings":{"schemaVersion":1,"profile":"Work","theme":"OneUiFrosted","blur":24,"refraction":0.35,"dispersion":0.04,"tintOpacity":55,"cornerRadius":28,"enabledWidgets":["clock","calendar"]}}
```

## Update settings

Send the full settings object returned by `get_settings`:

```json
{"version":1,"type":"set_settings","settings":{"schemaVersion":1,"profile":"Work","theme":"OneUiFrosted","blur":24,"refraction":0.35,"dispersion":0.04,"tintOpacity":55,"cornerRadius":28,"enabledWidgets":["clock"]}}
```

Success response:

```json
{"version":1,"ok":true}
```

Errors use `{"version":1,"ok":false,"error":"..."}`. A successful update is
validated and atomically saved to `%LOCALAPPDATA%\WVX\settings.json` before the
runtime reports success. Studio does not write a second copy of the settings.
If the runtime is offline, the Studio reports that state and does not claim
that a change was saved.

The clock is the only widget currently rendered. The settings schema recognizes
the other migrated GlassDesk widget IDs for compatibility; those IDs do not
cause an unimplemented widget to run.

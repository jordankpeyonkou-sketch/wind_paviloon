# WVX Studio

A WinUI 3 desktop control panel for the WVX runtime. It reads and applies GlassDesk settings through the runtime named pipe; it does not keep a separate local settings file.

## Requirements

- Windows 10 version 2004 (build 19041) or newer
- .NET 8 SDK
- Visual Studio 2022 with the **.NET desktop development** and **Windows App SDK C#** workloads/components

## Build and run

From the repository root, in PowerShell:

```powershell
dotnet restore .\apps\wvx-studio\WvxStudio.csproj
dotnet build .\apps\wvx-studio\WvxStudio.csproj -c Debug -p:Platform=x64
dotnet run --project .\apps\wvx-studio\WvxStudio.csproj -p:Platform=x64
```

The app starts with a bounded connection attempt to `\\.\pipe\wvx-runtime-v1`. If the runtime is offline, Studio displays the connection error and keeps all Apply actions disabled. Start the compatible runtime, then choose **Refresh runtime**. Only the clock can be toggled in this MVP. Weather, system monitor, media player, and calendar are shown as planned and unavailable; if the runtime reports any of those IDs enabled, Studio preserves them when applying other settings.

The pipe uses one UTF-8 newline-delimited JSON request and response per connection, protocol version `1`, and a 3-second connection/read/write timeout. Messages are limited to 64 KiB.

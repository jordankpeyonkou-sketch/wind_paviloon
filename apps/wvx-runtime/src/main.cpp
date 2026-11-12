#include "settings.hpp"

#include <Windows.h>
#include <d2d1.h>
#include <dwrite.h>
#include <dwmapi.h>
#include <winrt/Windows.Data.Json.h>
#include <winrt/Windows.Foundation.h>

#include <algorithm>
#include <array>
#include <chrono>
#include <ctime>
#include <mutex>
#include <string>
#include <thread>

#pragma comment(lib, "d2d1.lib")
#pragma comment(lib, "dwrite.lib")
#pragma comment(lib, "dwmapi.lib")
#pragma comment(lib, "windowsapp.lib")

namespace
{
    constexpr wchar_t WindowClassName[] = L"WVXRuntimeWindow";
    constexpr wchar_t PipeName[] = L"\\\\.\\pipe\\wvx-runtime-v1";
    constexpr std::size_t MaxMessageBytes = 64 * 1024;
    constexpr UINT_PTR ClockTimerId = 1;
    constexpr UINT SettingsUpdatedMessage = WM_APP + 1;

    void Log(const std::wstring& message)
    {
        const auto line = message + L"\r\n";
        OutputDebugStringW(line.c_str());
    }

    std::wstring ErrorMessage(DWORD code)
    {
        LPWSTR text = nullptr;
        const DWORD length = FormatMessageW(
            FORMAT_MESSAGE_ALLOCATE_BUFFER | FORMAT_MESSAGE_FROM_SYSTEM | FORMAT_MESSAGE_IGNORE_INSERTS,
            nullptr, code, 0, reinterpret_cast<LPWSTR>(&text), 0, nullptr);
        std::wstring result = length != 0 && text != nullptr ? text : L"Unknown Windows error.";
        if (text != nullptr)
        {
            LocalFree(text);
        }
        return result;
    }

    std::wstring HResultMessage(const wchar_t* operation, HRESULT result)
    {
        return std::wstring(operation) + L" (HRESULT " +
            std::to_wstring(static_cast<unsigned long>(result)) + L").";
    }

    winrt::Windows::Data::Json::JsonObject ErrorResponse(const std::wstring& message)
    {
        using namespace winrt::Windows::Data::Json;
        JsonObject response;
        response.Insert(L"version", JsonValue::CreateNumberValue(1));
        response.Insert(L"ok", JsonValue::CreateBooleanValue(false));
        response.Insert(L"error", JsonValue::CreateStringValue(message));
        return response;
    }

    bool ReadMessage(HANDLE pipe, std::string& message)
    {
        std::array<char, 4096> buffer{};
        message.clear();
        const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(5);
        for (;;)
        {
            DWORD bytesRead = 0;
            if (!ReadFile(pipe, buffer.data(), static_cast<DWORD>(buffer.size()), &bytesRead, nullptr))
            {
                const DWORD error = GetLastError();
                if (error != ERROR_NO_DATA || std::chrono::steady_clock::now() >= deadline)
                {
                    return false;
                }
                std::this_thread::sleep_for(std::chrono::milliseconds(10));
                continue;
            }
            if (bytesRead == 0 || message.size() + bytesRead > MaxMessageBytes)
            {
                return false;
            }

            const auto newline = std::find(buffer.begin(), buffer.begin() + bytesRead, '\n');
            const auto count = static_cast<std::size_t>(newline - buffer.begin());
            message.append(buffer.data(), count);
            if (newline != buffer.begin() + bytesRead)
            {
                return true;
            }
        }
    }

    bool WriteMessage(HANDLE pipe, const std::string& message)
    {
        std::string line = message;
        line.push_back('\n');
        std::size_t offset = 0;
        while (offset < line.size())
        {
            DWORD bytesWritten = 0;
            const auto chunk = static_cast<DWORD>(std::min<std::size_t>(
                line.size() - offset, MAXDWORD));
            if (!WriteFile(pipe, line.data() + offset, chunk, &bytesWritten, nullptr) || bytesWritten == 0)
            {
                return false;
            }
            offset += bytesWritten;
        }
        return true;
    }

    class Runtime
    {
    public:
        ~Runtime()
        {
            StopPipeServer();
            if (textFormat_ != nullptr)
            {
                textFormat_->Release();
            }
            if (textBrush_ != nullptr)
            {
                textBrush_->Release();
            }
            if (panelBrush_ != nullptr)
            {
                panelBrush_->Release();
            }
            if (renderTarget_ != nullptr)
            {
                renderTarget_->Release();
            }
            if (dwriteFactory_ != nullptr)
            {
                dwriteFactory_->Release();
            }
            if (d2dFactory_ != nullptr)
            {
                d2dFactory_->Release();
            }
        }

        int Run(HINSTANCE instance, int showCommand)
        {
            instance_ = instance;
            std::wstring settingsError;
            if (!wvx::LoadSettings(settings_, settingsError))
            {
                Log(L"Settings were rejected; safe defaults are active. " + settingsError);
                settings_ = wvx::Settings{};
            }

            WNDCLASSEXW windowClass{};
            windowClass.cbSize = sizeof(windowClass);
            windowClass.hInstance = instance_;
            windowClass.lpfnWndProc = WindowProcedure;
            windowClass.lpszClassName = WindowClassName;
            windowClass.hCursor = LoadCursorW(nullptr, IDC_ARROW);
            if (RegisterClassExW(&windowClass) == 0)
            {
                Log(L"Could not register the WVX window class. " + ErrorMessage(GetLastError()));
                return 1;
            }

            window_ = CreateWindowExW(
                WS_EX_TOOLWINDOW,
                WindowClassName,
                L"WVX Adaptive Clock",
                WS_POPUP | WS_THICKFRAME | WS_SYSMENU,
                CW_USEDEFAULT, CW_USEDEFAULT, 460, 190,
                nullptr, nullptr, instance_, this);
            if (window_ == nullptr)
            {
                const auto detail = startupError_.empty()
                    ? ErrorMessage(GetLastError())
                    : startupError_;
                MessageBoxW(nullptr, detail.c_str(), L"WVX Runtime could not start", MB_OK | MB_ICONERROR);
                Log(L"Could not create the WVX runtime window. " + detail);
                return 1;
            }

            ApplyBackdrop();
            if (IsClockEnabled(settings_))
            {
                ShowWindow(window_, showCommand);
                UpdateWindow(window_);
            }
            StartPipeServer();

            MSG message{};
            while (GetMessageW(&message, nullptr, 0, 0) > 0)
            {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
            StopPipeServer();
            return static_cast<int>(message.wParam);
        }

    private:
        static LRESULT CALLBACK WindowProcedure(HWND window, UINT message, WPARAM wParam, LPARAM lParam)
        {
            Runtime* self = reinterpret_cast<Runtime*>(GetWindowLongPtrW(window, GWLP_USERDATA));
            if (message == WM_NCCREATE)
            {
                const auto* create = reinterpret_cast<const CREATESTRUCTW*>(lParam);
                self = static_cast<Runtime*>(create->lpCreateParams);
                SetWindowLongPtrW(window, GWLP_USERDATA, reinterpret_cast<LONG_PTR>(self));
                self->window_ = window;
            }
            return self == nullptr ? DefWindowProcW(window, message, wParam, lParam)
                                   : self->HandleMessage(message, wParam, lParam);
        }

        LRESULT HandleMessage(UINT message, WPARAM wParam, LPARAM lParam)
        {
            switch (message)
            {
            case WM_CREATE:
                if (!CreateGraphicsResources())
                {
                    return -1;
                }
                if (IsClockEnabled(settings_))
                {
                    ScheduleNextClockUpdate();
                }
                return 0;
            case WM_SIZE:
                if (renderTarget_ != nullptr)
                {
                    const auto size = D2D1::SizeU(LOWORD(lParam), HIWORD(lParam));
                    if (size.width > 0 && size.height > 0 &&
                        FAILED(renderTarget_->Resize(size)))
                    {
                        ReleaseRenderTarget();
                    }
                }
                InvalidateRect(window_, nullptr, FALSE);
                return 0;
            case WM_TIMER:
                if (wParam == ClockTimerId)
                {
                    ScheduleNextClockUpdate();
                    InvalidateRect(window_, nullptr, FALSE);
                }
                return 0;
            case SettingsUpdatedMessage:
                if (wParam != 0)
                {
                    ShowWindow(window_, SW_SHOWNOACTIVATE);
                    ScheduleNextClockUpdate();
                    InvalidateRect(window_, nullptr, FALSE);
                }
                else
                {
                    KillTimer(window_, ClockTimerId);
                    ShowWindow(window_, SW_HIDE);
                }
                return 0;
            case WM_PAINT:
                Paint();
                return 0;
            case WM_ERASEBKGND:
                return 1;
            case WM_NCHITTEST:
            {
                const LRESULT hit = DefWindowProcW(window_, message, wParam, lParam);
                return hit == HTCLIENT ? HTCAPTION : hit;
            }
            case WM_CLOSE:
                DestroyWindow(window_);
                return 0;
            case WM_DESTROY:
                PostQuitMessage(0);
                return 0;
            default:
                return DefWindowProcW(window_, message, wParam, lParam);
            }
        }

        void ApplyBackdrop()
        {
            const DWORD backdrop = 3; // DWMSBT_TRANSIENTWINDOW: Windows 11 acrylic-style backdrop.
            const HRESULT result = DwmSetWindowAttribute(
                window_, DWMWA_SYSTEMBACKDROP_TYPE, &backdrop, sizeof(backdrop));
            if (FAILED(result))
            {
                Log(L"Windows did not provide a system backdrop; only the native clock surface will be shown.");
            }
            const MARGINS frameMargins{-1, -1, -1, -1};
            DwmExtendFrameIntoClientArea(window_, &frameMargins);
            const DWORD darkMode = TRUE;
            DwmSetWindowAttribute(window_, DWMWA_USE_IMMERSIVE_DARK_MODE, &darkMode, sizeof(darkMode));
        }

        bool CreateGraphicsResources()
        {
            HRESULT result = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, &d2dFactory_);
            if (FAILED(result))
            {
                startupError_ = HResultMessage(L"Could not initialize Direct2D", result);
                Log(startupError_);
                return false;
            }
            result = DWriteCreateFactory(
                DWRITE_FACTORY_TYPE_SHARED, __uuidof(IDWriteFactory),
                reinterpret_cast<IUnknown**>(&dwriteFactory_));
            if (FAILED(result))
            {
                startupError_ = HResultMessage(L"Could not initialize DirectWrite", result);
                Log(startupError_);
                return false;
            }
            result = dwriteFactory_->CreateTextFormat(
                L"Segoe UI Light", nullptr, DWRITE_FONT_WEIGHT_LIGHT,
                DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_STRETCH_NORMAL,
                58.0f, L"en-us", &textFormat_);
            if (FAILED(result))
            {
                startupError_ = HResultMessage(L"Could not create the clock text format", result);
                Log(startupError_);
                return false;
            }
            if (FAILED(textFormat_->SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER)) ||
                FAILED(textFormat_->SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER)) ||
                !EnsureRenderTarget())
            {
                startupError_ = L"Could not initialize the WVX drawing surface.";
                Log(startupError_);
                return false;
            }
            return true;
        }

        bool EnsureRenderTarget()
        {
            if (renderTarget_ != nullptr || d2dFactory_ == nullptr)
            {
                return renderTarget_ != nullptr;
            }

            RECT client{};
            GetClientRect(window_, &client);
            const auto size = D2D1::SizeU(
                static_cast<UINT32>(std::max<LONG>(1, client.right - client.left)),
                static_cast<UINT32>(std::max<LONG>(1, client.bottom - client.top)));
            const auto properties = D2D1::RenderTargetProperties(
                D2D1_RENDER_TARGET_TYPE_DEFAULT,
                D2D1::PixelFormat(DXGI_FORMAT_UNKNOWN, D2D1_ALPHA_MODE_PREMULTIPLIED));
            const auto windowProperties = D2D1::HwndRenderTargetProperties(window_, size);
            HRESULT result = d2dFactory_->CreateHwndRenderTarget(
                properties, windowProperties, &renderTarget_);
            if (FAILED(result))
            {
                Log(L"Could not create the Direct2D surface. " + ErrorMessage(static_cast<DWORD>(result)));
                return false;
            }

            result = renderTarget_->CreateSolidColorBrush(
                D2D1::ColorF(0.62f, 0.72f, 0.92f, 0.92f), &textBrush_);
            if (SUCCEEDED(result))
            {
                result = renderTarget_->CreateSolidColorBrush(
                    D2D1::ColorF(0.28f, 0.34f, 0.48f, 0.35f), &panelBrush_);
            }
            if (FAILED(result))
            {
                Log(L"Could not create WVX drawing brushes. " + ErrorMessage(static_cast<DWORD>(result)));
                return false;
            }
            return true;
        }

        void Paint()
        {
            PAINTSTRUCT paint{};
            BeginPaint(window_, &paint);
            if (!EnsureRenderTarget() || textFormat_ == nullptr)
            {
                EndPaint(window_, &paint);
                return;
            }

            wvx::Settings settings;
            {
                std::scoped_lock lock(settingsMutex_);
                settings = settings_;
            }
            if (!IsClockEnabled(settings))
            {
                renderTarget_->BeginDraw();
                renderTarget_->Clear(D2D1::ColorF(0, 0, 0, 0));
                renderTarget_->EndDraw();
                EndPaint(window_, &paint);
                return;
            }

            SYSTEMTIME now{};
            GetLocalTime(&now);
            wchar_t timeText[6]{};
            swprintf_s(timeText, L"%02u:%02u", now.wHour, now.wMinute);

            const auto size = renderTarget_->GetSize();
            const auto panel = D2D1::RoundedRect(
                D2D1::RectF(1.0f, 1.0f, size.width - 1.0f, size.height - 1.0f),
                static_cast<float>(settings.cornerRadius), static_cast<float>(settings.cornerRadius));
            const float tintAlpha = static_cast<float>(settings.tintOpacity / 100.0 * 0.34);
            D2D1_COLOR_F accent = D2D1::ColorF(0.62f, 0.72f, 0.92f, 0.92f);
            D2D1_COLOR_F tint = D2D1::ColorF(0.28f, 0.34f, 0.48f, tintAlpha);
            if (settings.theme == "LiquidGlass")
            {
                accent = D2D1::ColorF(0.80f, 0.62f, 1.0f, 0.92f);
                tint = D2D1::ColorF(0.43f, 0.31f, 0.60f, tintAlpha);
            }
            else if (settings.theme == "LiquidGlassClear")
            {
                accent = D2D1::ColorF(0.40f, 0.84f, 0.88f, 0.92f);
                tint = D2D1::ColorF(0.25f, 0.48f, 0.53f, tintAlpha);
            }
            else if (settings.theme == "Minimal")
            {
                accent = D2D1::ColorF(0.72f, 0.76f, 0.82f, 0.92f);
                tint = D2D1::ColorF(0.25f, 0.27f, 0.32f, tintAlpha * 0.5f);
            }
            textBrush_->SetColor(accent);
            panelBrush_->SetColor(tint);

            renderTarget_->BeginDraw();
            renderTarget_->Clear(D2D1::ColorF(0, 0, 0, 0));
            renderTarget_->FillRoundedRectangle(panel, panelBrush_);
            renderTarget_->DrawRoundedRectangle(
                panel, textBrush_, 1.0f,
                nullptr);
            const auto bounds = D2D1::RectF(12.0f, 8.0f, size.width - 12.0f, size.height - 8.0f);
            renderTarget_->DrawText(
                timeText, 5, textFormat_, bounds, textBrush_,
                D2D1_DRAW_TEXT_OPTIONS_CLIP, DWRITE_MEASURING_MODE_NATURAL);
            const HRESULT result = renderTarget_->EndDraw();
            if (result == D2DERR_RECREATE_TARGET)
            {
                ReleaseRenderTarget();
            }
            else if (FAILED(result))
            {
                Log(L"Direct2D could not draw the clock. " + ErrorMessage(static_cast<DWORD>(result)));
            }
            EndPaint(window_, &paint);
        }

        void ReleaseRenderTarget()
        {
            if (panelBrush_ != nullptr)
            {
                panelBrush_->Release();
                panelBrush_ = nullptr;
            }
            if (textBrush_ != nullptr)
            {
                textBrush_->Release();
                textBrush_ = nullptr;
            }
            if (renderTarget_ != nullptr)
            {
                renderTarget_->Release();
                renderTarget_ = nullptr;
            }
        }

        void ScheduleNextClockUpdate()
        {
            const auto now = std::chrono::system_clock::to_time_t(std::chrono::system_clock::now());
            const UINT milliseconds = static_cast<UINT>(
                std::max<std::time_t>(1, 60 - (now % 60)) * 1000);
            SetTimer(window_, ClockTimerId, milliseconds, nullptr);
        }

        void StartPipeServer()
        {
            pipeThread_ = std::jthread([this](std::stop_token stopToken)
            {
                winrt::init_apartment(winrt::apartment_type::multi_threaded);
                while (!stopToken.stop_requested())
                {
                    HANDLE pipe = CreateNamedPipeW(
                        PipeName, PIPE_ACCESS_DUPLEX,
                        PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
                        PIPE_UNLIMITED_INSTANCES, 4096, 4096, 2000, nullptr);
                    if (pipe == INVALID_HANDLE_VALUE)
                    {
                        Log(L"Could not create the WVX settings pipe. " + ErrorMessage(GetLastError()));
                        std::this_thread::sleep_for(std::chrono::seconds(1));
                        continue;
                    }

                    const BOOL connected = ConnectNamedPipe(pipe, nullptr)
                        ? TRUE
                        : GetLastError() == ERROR_PIPE_CONNECTED;
                    DWORD pipeMode = PIPE_READMODE_BYTE | PIPE_NOWAIT;
                    const bool readable = connected &&
                        SetNamedPipeHandleState(pipe, &pipeMode, nullptr, nullptr);
                    if (connected && !readable)
                    {
                        Log(L"Could not set the WVX pipe read mode. " + ErrorMessage(GetLastError()));
                    }
                    if (connected && readable && !stopToken.stop_requested())
                    {
                        HandlePipeRequest(pipe);
                    }
                    DisconnectNamedPipe(pipe);
                    CloseHandle(pipe);
                }
                winrt::uninit_apartment();
            });
        }

        void StopPipeServer()
        {
            if (!pipeThread_.joinable())
            {
                return;
            }
            pipeThread_.request_stop();
            HANDLE wake = CreateFileW(
                PipeName, GENERIC_READ | GENERIC_WRITE, 0, nullptr, OPEN_EXISTING, 0, nullptr);
            if (wake != INVALID_HANDLE_VALUE)
            {
                CloseHandle(wake);
            }
            pipeThread_.join();
        }

        void HandlePipeRequest(HANDLE pipe)
        {
            std::string request;
            if (!ReadMessage(pipe, request))
            {
                WriteMessage(pipe, winrt::to_string(ErrorResponse(L"Invalid or oversized IPC message.").Stringify()));
                return;
            }

            winrt::Windows::Data::Json::JsonObject response;
            try
            {
                const auto message = winrt::Windows::Data::Json::JsonObject::Parse(winrt::to_hstring(request));
                if (!message.HasKey(L"version") || message.GetNamedNumber(L"version") != 1 ||
                    !message.HasKey(L"type"))
                {
                    response = ErrorResponse(L"Unsupported or malformed IPC message.");
                }
                else
                {
                    const auto type = message.GetNamedString(L"type");
                    if (type == L"get_settings")
                    {
                        wvx::Settings snapshot;
                        {
                            std::scoped_lock lock(settingsMutex_);
                            snapshot = settings_;
                        }
                        response = winrt::Windows::Data::Json::JsonObject::Parse(
                            winrt::to_hstring(wvx::SerializeSettings(snapshot)));
                    }
                    else if (type == L"set_settings")
                    {
                        wvx::Settings updated;
                        std::wstring error;
                        if (!wvx::DeserializeSettings(request, updated, error))
                        {
                            response = ErrorResponse(error);
                        }
                        else if (!wvx::SaveSettings(updated, error))
                        {
                            response = ErrorResponse(error);
                        }
                        else
                        {
                            const bool clockEnabled = IsClockEnabled(updated);
                            {
                                std::scoped_lock lock(settingsMutex_);
                                settings_ = std::move(updated);
                            }
                            if (!PostMessageW(window_, SettingsUpdatedMessage, clockEnabled ? 1 : 0, 0))
                            {
                                response = ErrorResponse(
                                    L"Settings were saved, but the runtime window could not be updated. " +
                                    ErrorMessage(GetLastError()));
                            }
                            else
                            {
                                response.Insert(L"version", winrt::Windows::Data::Json::JsonValue::CreateNumberValue(1));
                                response.Insert(L"ok", winrt::Windows::Data::Json::JsonValue::CreateBooleanValue(true));
                            }
                        }

                        static bool IsClockEnabled(const wvx::Settings& settings)
                        {
                            return std::find(settings.enabledWidgets.begin(), settings.enabledWidgets.end(), "clock") !=
                                settings.enabledWidgets.end();
                        }
                    }
                    else
                    {
                        response = ErrorResponse(L"Unknown IPC request type.");
                    }
                }
            }
            catch (const winrt::hresult_error& exception)
            {
                response = ErrorResponse(exception.message().c_str());
            }

            if (!WriteMessage(pipe, winrt::to_string(response.Stringify())))
            {
                Log(L"Could not send an IPC response. " + ErrorMessage(GetLastError()));
            }
        }

        HINSTANCE instance_ = nullptr;
        HWND window_ = nullptr;
        ID2D1Factory* d2dFactory_ = nullptr;
        IDWriteFactory* dwriteFactory_ = nullptr;
        IDWriteTextFormat* textFormat_ = nullptr;
        ID2D1HwndRenderTarget* renderTarget_ = nullptr;
        ID2D1SolidColorBrush* textBrush_ = nullptr;
        ID2D1SolidColorBrush* panelBrush_ = nullptr;
        std::mutex settingsMutex_;
        wvx::Settings settings_;
        std::jthread pipeThread_;
        std::wstring startupError_;
    };
}

int WINAPI wWinMain(HINSTANCE instance, HINSTANCE, PWSTR, int showCommand)
{
    try
    {
        winrt::init_apartment(winrt::apartment_type::single_threaded);
        Runtime runtime;
        const int result = runtime.Run(instance, showCommand);
        winrt::uninit_apartment();
        return result;
    }
    catch (const winrt::hresult_error& exception)
    {
        MessageBoxW(nullptr, exception.message().c_str(), L"WVX Runtime error", MB_OK | MB_ICONERROR);
        return 1;
    }
}

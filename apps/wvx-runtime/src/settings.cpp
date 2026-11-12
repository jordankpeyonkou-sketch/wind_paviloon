#include "settings.hpp"

#include <Windows.h>
#include <ShlObj.h>
#include <winrt/Windows.Data.Json.h>
#include <winrt/Windows.Foundation.h>

#include <algorithm>
#include <atomic>
#include <cmath>
#include <cstdint>
#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <limits>
#include <stdexcept>
#include <system_error>

namespace
{
    std::atomic_uint64_t TemporaryFileCounter = 0;
}

namespace
{
    using winrt::Windows::Data::Json::JsonArray;
    using winrt::Windows::Data::Json::JsonObject;
    using winrt::Windows::Data::Json::JsonValue;

    constexpr std::size_t MaxSettingsBytes = 1024 * 1024;
    constexpr int CurrentSchemaVersion = 1;

    std::filesystem::path SettingsPath()
    {
        PWSTR localAppData = nullptr;
        const HRESULT result = SHGetKnownFolderPath(FOLDERID_LocalAppData, KF_FLAG_DEFAULT, nullptr, &localAppData);
        if (FAILED(result))
        {
            throw winrt::hresult_error(result, L"Could not locate the local application data directory.");
        }

        std::filesystem::path path(localAppData);
        CoTaskMemFree(localAppData);
        return path / L"WVX" / L"settings.json";
    }

    std::filesystem::path LegacySettingsPath()
    {
        const wchar_t* appData = _wgetenv(L"APPDATA");
        if (appData == nullptr || *appData == L'\0')
        {
            return {};
        }
        return std::filesystem::path(appData) / L"GlassDesk" / L"settings.json";
    }

    JsonObject ToJson(const wvx::Settings& settings)
    {
        JsonArray widgets;
        for (const auto& widget : settings.enabledWidgets)
        {
            widgets.Append(JsonValue::CreateStringValue(winrt::to_hstring(widget)));
        }

        JsonObject value;
        value.Insert(L"schemaVersion", JsonValue::CreateNumberValue(CurrentSchemaVersion));
        value.Insert(L"profile", JsonValue::CreateStringValue(winrt::to_hstring(settings.profile)));
        value.Insert(L"theme", JsonValue::CreateStringValue(winrt::to_hstring(settings.theme)));
        value.Insert(L"blur", JsonValue::CreateNumberValue(settings.blur));
        value.Insert(L"refraction", JsonValue::CreateNumberValue(settings.refraction));
        value.Insert(L"dispersion", JsonValue::CreateNumberValue(settings.dispersion));
        value.Insert(L"tintOpacity", JsonValue::CreateNumberValue(settings.tintOpacity));
        value.Insert(L"cornerRadius", JsonValue::CreateNumberValue(settings.cornerRadius));
        value.Insert(L"enabledWidgets", widgets);
        return value;
    }

    bool ReadString(const JsonObject& object, const wchar_t* name, std::string& result, std::wstring& error)
    {
        if (!object.HasKey(name) || object.GetNamedValue(name).ValueType() != winrt::Windows::Data::Json::JsonValueType::String)
        {
            error = std::wstring(L"Missing or invalid setting: ") + name;
            return false;
        }
        result = winrt::to_string(object.GetNamedString(name));
        return true;
    }

    bool ReadNumber(
        const JsonObject& object,
        const wchar_t* name,
        double& result,
        std::wstring& error,
        const wchar_t* legacyName = nullptr)
    {
        const wchar_t* key = object.HasKey(name) ? name : legacyName;
        if (key == nullptr || !object.HasKey(key) ||
            object.GetNamedValue(key).ValueType() != winrt::Windows::Data::Json::JsonValueType::Number)
        {
            error = std::wstring(L"Missing or invalid setting: ") + name;
            return false;
        }
        result = object.GetNamedNumber(key);
        return true;
    }

    bool ReadSettingsObject(const JsonObject& object, wvx::Settings& settings, std::wstring& error)
    {
        try
        {
            if (!ReadString(object, L"profile", settings.profile, error) ||
                !ReadString(object, L"theme", settings.theme, error) ||
                !ReadNumber(object, L"blur", settings.blur, error) ||
                !ReadNumber(object, L"refraction", settings.refraction, error) ||
                !ReadNumber(object, L"dispersion", settings.dispersion, error) ||
                !ReadNumber(object, L"tintOpacity", settings.tintOpacity, error, L"tint_opacity") ||
                !ReadNumber(object, L"cornerRadius", settings.cornerRadius, error, L"corner_radius"))
            {
                return false;
            }

            const wchar_t* widgetKey = object.HasKey(L"enabledWidgets")
                ? L"enabledWidgets"
                : L"enabled_widgets";
            if (!object.HasKey(widgetKey) ||
                object.GetNamedValue(widgetKey).ValueType() != winrt::Windows::Data::Json::JsonValueType::Array)
            {
                error = L"Missing or invalid setting: enabledWidgets";
                return false;
            }

            settings.enabledWidgets.clear();
            for (const auto& item : object.GetNamedArray(widgetKey))
            {
                if (item.ValueType() != winrt::Windows::Data::Json::JsonValueType::String)
                {
                    error = L"Widget identifiers must be strings.";
                    return false;
                }
                settings.enabledWidgets.push_back(winrt::to_string(item.GetString()));
            }
        }
        catch (const winrt::hresult_error& exception)
        {
            error = exception.message().c_str();
            return false;
        }
        return wvx::ValidateSettings(settings, error);
    }

    std::string ReadFile(const std::filesystem::path& path)
    {
        std::ifstream file(path, std::ios::binary);
        if (!file)
        {
            throw std::runtime_error("Could not open settings file.");
        }
        file.seekg(0, std::ios::end);
        const std::streamoff length = file.tellg();
        if (length < 0 || static_cast<std::uint64_t>(length) > MaxSettingsBytes)
        {
            throw std::runtime_error("Settings file exceeds the 1 MiB size limit.");
        }
        std::string contents(static_cast<std::size_t>(length), '\0');
        file.seekg(0, std::ios::beg);
        if (!contents.empty() && !file.read(contents.data(), static_cast<std::streamsize>(contents.size())))
        {
            throw std::runtime_error("Could not read settings file.");
        }
        return contents;
    }

    std::string SettingsDocument(const wvx::Settings& settings)
    {
        JsonObject document;
        document.Insert(L"schemaVersion", JsonValue::CreateNumberValue(CurrentSchemaVersion));
        document.Insert(L"settings", ToJson(settings));
        return winrt::to_string(document.Stringify());
    }

    bool WriteAll(HANDLE file, const std::string& contents)
    {
        std::size_t offset = 0;
        while (offset < contents.size())
        {
            const DWORD chunk = static_cast<DWORD>(std::min<std::size_t>(
                contents.size() - offset, std::numeric_limits<DWORD>::max()));
            DWORD written = 0;
            if (!WriteFile(file, contents.data() + offset, chunk, &written, nullptr) || written == 0)
            {
                return false;
            }
            offset += written;
        }
        return true;
    }
}

namespace wvx
{
    bool ValidateSettings(const Settings& settings, std::wstring& error)
    {
        const auto validRange = [](double value, double minimum, double maximum)
        {
            return std::isfinite(value) && value >= minimum && value <= maximum;
        };
        if (!validRange(settings.blur, 0.0, 48.0) ||
            !validRange(settings.refraction, 0.0, 1.0) ||
            !validRange(settings.dispersion, 0.0, 0.12) ||
            !validRange(settings.tintOpacity, 0.0, 100.0) ||
            !validRange(settings.cornerRadius, 8.0, 44.0))
        {
            error = L"A visual setting is outside its supported range.";
            return false;
        }

        static const std::vector<std::string> profiles = {"Work", "Gaming", "Minimal", "PowerSaving"};
        static const std::vector<std::string> themes = {"OneUiFrosted", "LiquidGlass", "LiquidGlassClear", "Minimal"};
        static const std::vector<std::string> knownWidgets = {
            "clock", "weather", "system-monitor", "media-player", "calendar"};
        if (std::find(profiles.begin(), profiles.end(), settings.profile) == profiles.end() ||
            std::find(themes.begin(), themes.end(), settings.theme) == themes.end())
        {
            error = L"The selected profile or theme is not supported.";
            return false;
        }

        for (std::size_t i = 0; i < settings.enabledWidgets.size(); ++i)
        {
            const auto& id = settings.enabledWidgets[i];
            if (std::find(knownWidgets.begin(), knownWidgets.end(), id) == knownWidgets.end())
            {
                error = L"Unknown widget identifier.";
                return false;
            }
            if (std::find(settings.enabledWidgets.begin(), settings.enabledWidgets.begin() + i, id) !=
                settings.enabledWidgets.begin() + i)
            {
                error = L"Duplicate widget identifier.";
                return false;
            }
        }
        return true;
    }

    std::string SerializeSettings(const Settings& settings)
    {
        JsonObject document;
        document.Insert(L"version", JsonValue::CreateNumberValue(1));
        document.Insert(L"ok", JsonValue::CreateBooleanValue(true));
        document.Insert(L"runtime", JsonValue::CreateStringValue(L"running"));
        document.Insert(L"settings", ToJson(settings));
        return winrt::to_string(document.Stringify());
    }

    bool DeserializeSettings(const std::string& json, Settings& settings, std::wstring& error)
    {
        if (json.empty() || json.size() > MaxSettingsBytes)
        {
            error = L"Settings data is empty or exceeds the 1 MiB size limit.";
            return false;
        }
        try
        {
            const auto root = JsonObject::Parse(winrt::to_hstring(json));
            JsonObject object = root;
            if (root.HasKey(L"schemaVersion") || root.HasKey(L"schema_version"))
            {
                const auto version = root.GetNamedNumber(
                    root.HasKey(L"schemaVersion") ? L"schemaVersion" : L"schema_version");
                if (version != CurrentSchemaVersion)
                {
                    error = L"Unsupported settings schema version.";
                    return false;
                }
                if (root.HasKey(L"settings") &&
                    root.GetNamedValue(L"settings").ValueType() == winrt::Windows::Data::Json::JsonValueType::Object)
                {
                    object = root.GetNamedObject(L"settings");
                }
            }
            else if (root.HasKey(L"version"))
            {
                if (root.GetNamedNumber(L"version") != 1 || !root.HasKey(L"settings") ||
                    root.GetNamedValue(L"settings").ValueType() != winrt::Windows::Data::Json::JsonValueType::Object)
                {
                    error = L"Unsupported or malformed IPC message.";
                    return false;
                }
                object = root.GetNamedObject(L"settings");
                if (!object.HasKey(L"schemaVersion") ||
                    object.GetNamedNumber(L"schemaVersion") != CurrentSchemaVersion)
                {
                    error = L"Unsupported settings schema version.";
                    return false;
                }
            }
            return ReadSettingsObject(object, settings, error);
        }
        catch (const winrt::hresult_error& exception)
        {
            error = exception.message().c_str();
            return false;
        }
        catch (const std::exception& exception)
        {
            error = winrt::to_hstring(exception.what()).c_str();
            return false;
        }
    }

    bool LoadSettings(Settings& settings, std::wstring& error)
    {
        try
        {
            auto path = SettingsPath();
            if (!std::filesystem::exists(path))
            {
                path = LegacySettingsPath();
                if (path.empty() || !std::filesystem::exists(path))
                {
                    return true;
                }
            }
            const auto json = ReadFile(path);
            const auto root = JsonObject::Parse(winrt::to_hstring(json));
            if (root.HasKey(L"schemaVersion") || root.HasKey(L"schema_version"))
            {
                const auto version = root.GetNamedNumber(
                    root.HasKey(L"schemaVersion") ? L"schemaVersion" : L"schema_version");
                if (version != CurrentSchemaVersion ||
                    !root.HasKey(L"settings") ||
                    root.GetNamedValue(L"settings").ValueType() != winrt::Windows::Data::Json::JsonValueType::Object)
                {
                    error = L"Settings file has an unsupported schema.";
                    return false;
                }
                return ReadSettingsObject(root.GetNamedObject(L"settings"), settings, error);
            }
            return ReadSettingsObject(root, settings, error);
        }
        catch (const winrt::hresult_error& exception)
        {
            error = exception.message().c_str();
            return false;
        }
        catch (const std::exception& exception)
        {
            error = winrt::to_hstring(exception.what()).c_str();
            return false;
        }
    }

    bool SaveSettings(const Settings& settings, std::wstring& error)
    {
        if (!ValidateSettings(settings, error))
        {
            return false;
        }
        try
        {
            const auto path = SettingsPath();
            std::filesystem::create_directories(path.parent_path());
            const auto contents = SettingsDocument(settings);
            std::filesystem::path temporary;
            HANDLE file = INVALID_HANDLE_VALUE;
            for (int attempt = 0; attempt < 16; ++attempt)
            {
                temporary = path.parent_path() /
                    (L"settings-" + std::to_wstring(GetCurrentProcessId()) + L"-" +
                     std::to_wstring(TemporaryFileCounter.fetch_add(1)) + L".tmp");
                file = CreateFileW(
                    temporary.c_str(), GENERIC_WRITE, 0, nullptr, CREATE_NEW,
                    FILE_ATTRIBUTE_NORMAL, nullptr);
                if (file != INVALID_HANDLE_VALUE || GetLastError() != ERROR_FILE_EXISTS)
                {
                    break;
                }
            }
            if (file == INVALID_HANDLE_VALUE)
            {
                throw std::system_error(GetLastError(), std::system_category(), "Could not create temporary settings file.");
            }
            const bool written = WriteAll(file, contents) && FlushFileBuffers(file);
            const DWORD writeError = written ? ERROR_SUCCESS : GetLastError();
            CloseHandle(file);
            if (!written)
            {
                DeleteFileW(temporary.c_str());
                throw std::system_error(writeError, std::system_category(), "Could not write settings file.");
            }
            if (!MoveFileExW(temporary.c_str(), path.c_str(), MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH))
            {
                const DWORD moveError = GetLastError();
                DeleteFileW(temporary.c_str());
                throw std::system_error(moveError, std::system_category(), "Could not replace settings file.");
            }
            return true;
        }
        catch (const winrt::hresult_error& exception)
        {
            error = exception.message().c_str();
            return false;
        }
        catch (const std::exception& exception)
        {
            error = winrt::to_hstring(exception.what()).c_str();
            return false;
        }
    }
}

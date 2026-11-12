#pragma once

#include <string>
#include <vector>

namespace wvx
{
    struct Settings
    {
        std::string profile = "Work";
        std::string theme = "OneUiFrosted";
        double blur = 24.0;
        double refraction = 0.35;
        double dispersion = 0.04;
        double tintOpacity = 55.0;
        double cornerRadius = 28.0;
        std::vector<std::string> enabledWidgets = {"clock", "calendar"};
    };

    bool ValidateSettings(const Settings& settings, std::wstring& error);
    bool LoadSettings(Settings& settings, std::wstring& error);
    bool SaveSettings(const Settings& settings, std::wstring& error);
    std::string SerializeSettings(const Settings& settings);
    bool DeserializeSettings(const std::string& json, Settings& settings, std::wstring& error);
}

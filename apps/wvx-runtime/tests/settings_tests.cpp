#include "settings.hpp"

#include <winrt/Windows.Foundation.h>

#include <iostream>
#include <limits>
#include <string>

namespace
{
    int failures = 0;

    void Expect(bool condition, const char* message)
    {
        if (!condition)
        {
            std::cerr << "FAIL: " << message << '\n';
            ++failures;
        }
    }
}

int main()
{
    winrt::init_apartment();
    std::wstring error;
    wvx::Settings defaults;
    Expect(wvx::ValidateSettings(defaults, error), "default settings should be valid");

    wvx::Settings decoded;
    Expect(
        wvx::DeserializeSettings(wvx::SerializeSettings(defaults), decoded, error),
        "versioned settings should round-trip through the Studio IPC shape");
    Expect(decoded.enabledWidgets == defaults.enabledWidgets, "round-trip should preserve enabled widgets");

    const std::string legacySettings =
        R"json({"schema_version":1,"settings":{"profile":"Work","theme":"OneUiFrosted","blur":24,"refraction":0.35,"dispersion":0.04,"tint_opacity":55,"corner_radius":28,"enabled_widgets":["clock","calendar"]}})json";
    Expect(
        wvx::DeserializeSettings(legacySettings, decoded, error),
        "versioned GlassDesk settings should remain readable");
    Expect(decoded.enabledWidgets == defaults.enabledWidgets, "legacy settings should preserve widget choices");

    const std::string unversionedLegacySettings =
        R"json({"profile":"Work","theme":"OneUiFrosted","blur":24,"refraction":0.35,"dispersion":0.04,"tint_opacity":55,"corner_radius":28,"enabled_widgets":["clock","calendar"]})json";
    Expect(
        wvx::DeserializeSettings(unversionedLegacySettings, decoded, error),
        "unversioned GlassDesk settings should remain readable");

    auto invalid = defaults;
    invalid.blur = std::numeric_limits<double>::quiet_NaN();
    Expect(!wvx::ValidateSettings(invalid, error), "non-finite blur should be rejected");

    invalid = defaults;
    invalid.enabledWidgets.push_back("unknown-widget");
    Expect(!wvx::ValidateSettings(invalid, error), "unknown widgets should be rejected");

    invalid = defaults;
    invalid.enabledWidgets.push_back("clock");
    Expect(!wvx::ValidateSettings(invalid, error), "duplicate widgets should be rejected");

    invalid = defaults;
    invalid.theme = "UnknownTheme";
    Expect(!wvx::ValidateSettings(invalid, error), "unknown themes should be rejected");

    winrt::uninit_apartment();
    return failures == 0 ? 0 : 1;
}

cmake_minimum_required(VERSION 3.25)
project(WVXRuntime VERSION 0.1.0 LANGUAGES CXX)

if(NOT WIN32)
    message(FATAL_ERROR "WVX Runtime currently targets Windows 11.")
endif()

include(CTest)

add_executable(wvx-runtime WIN32
    apps/wvx-runtime/src/main.cpp
    apps/wvx-runtime/src/settings.cpp
)

target_compile_features(wvx-runtime PRIVATE cxx_std_20)
target_include_directories(wvx-runtime PRIVATE apps/wvx-runtime/src)
target_compile_definitions(wvx-runtime PRIVATE UNICODE _UNICODE WIN32_LEAN_AND_MEAN NOMINMAX)
target_link_libraries(wvx-runtime PRIVATE d2d1 dwrite dwmapi windowsapp shell32 ole32)

if(MSVC)
    target_compile_options(wvx-runtime PRIVATE /W4 /permissive- /EHsc)
else()
    target_compile_options(wvx-runtime PRIVATE -Wall -Wextra -Wpedantic)
endif()

if(BUILD_TESTING)
    add_executable(wvx-settings-tests
        apps/wvx-runtime/src/settings.cpp
        apps/wvx-runtime/tests/settings_tests.cpp
    )
    target_compile_features(wvx-settings-tests PRIVATE cxx_std_20)
    target_include_directories(wvx-settings-tests PRIVATE apps/wvx-runtime/src)
    target_compile_definitions(wvx-settings-tests PRIVATE UNICODE _UNICODE WIN32_LEAN_AND_MEAN NOMINMAX)
    target_link_libraries(wvx-settings-tests PRIVATE windowsapp shell32 ole32)
    add_test(NAME wvx-settings-tests COMMAND wvx-settings-tests)
endif()

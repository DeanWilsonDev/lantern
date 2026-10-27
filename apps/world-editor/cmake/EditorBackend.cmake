# Builds crates/editor-backend with Cargo and exposes it as the `lantern_editor_backend` target.
#
# The backend links the system SDL3 (its `system-sdl` feature, found through pkg-config), the
# same SDL Penumbra uses, so the process has one SDL. Set PKG_CONFIG_PATH if SDL3 is installed
# somewhere pkg-config doesn't look by default.

find_program(CARGO_EXECUTABLE cargo REQUIRED)

get_filename_component(LANTERN_WORKSPACE_ROOT "${CMAKE_CURRENT_LIST_DIR}/../../.." ABSOLUTE)

if(CMAKE_BUILD_TYPE STREQUAL "Debug")
    set(LANTERN_CARGO_PROFILE_FLAG "")
    set(LANTERN_CARGO_PROFILE_DIRECTORY "debug")
else()
    set(LANTERN_CARGO_PROFILE_FLAG "--release")
    set(LANTERN_CARGO_PROFILE_DIRECTORY "release")
endif()

set(LANTERN_CARGO_TARGET_DIRECTORY "${CMAKE_CURRENT_BINARY_DIR}/cargo")
if(WIN32)
    set(LANTERN_EDITOR_BACKEND_LIBRARY
        "${LANTERN_CARGO_TARGET_DIRECTORY}/${LANTERN_CARGO_PROFILE_DIRECTORY}/lantern_editor_backend.lib")
else()
    set(LANTERN_EDITOR_BACKEND_LIBRARY
        "${LANTERN_CARGO_TARGET_DIRECTORY}/${LANTERN_CARGO_PROFILE_DIRECTORY}/liblantern_editor_backend.a")
endif()

# Cargo tracks its own inputs, so it runs on every build and returns at once when nothing changed.
add_custom_target(lantern_editor_backend_cargo
    COMMAND ${CARGO_EXECUTABLE} build -p lantern-editor-backend --lib ${LANTERN_CARGO_PROFILE_FLAG}
            --target-dir "${LANTERN_CARGO_TARGET_DIRECTORY}"
    WORKING_DIRECTORY "${LANTERN_WORKSPACE_ROOT}"
    BYPRODUCTS "${LANTERN_EDITOR_BACKEND_LIBRARY}"
    COMMENT "Building the Rust editor backend"
    USES_TERMINAL)

add_library(lantern_editor_backend STATIC IMPORTED GLOBAL)
set_target_properties(lantern_editor_backend PROPERTIES
    IMPORTED_LOCATION "${LANTERN_EDITOR_BACKEND_LIBRARY}"
    INTERFACE_INCLUDE_DIRECTORIES "${LANTERN_WORKSPACE_ROOT}/crates/editor-backend/include")
add_dependencies(lantern_editor_backend lantern_editor_backend_cargo)

# What a Rust static library needs from the platform (cargo's --print native-static-libs).
if(APPLE)
    set(LANTERN_RUST_PLATFORM_LIBRARIES "-framework CoreFoundation" "-framework Security" iconv)
elseif(WIN32)
    set(LANTERN_RUST_PLATFORM_LIBRARIES ws2_32 userenv bcrypt ntdll advapi32 kernel32)
else()
    set(LANTERN_RUST_PLATFORM_LIBRARIES pthread dl m rt util)
endif()
target_link_libraries(lantern_editor_backend INTERFACE SDL3::SDL3 ${LANTERN_RUST_PLATFORM_LIBRARIES})

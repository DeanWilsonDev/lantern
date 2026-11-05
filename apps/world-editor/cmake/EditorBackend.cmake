# Exposes the editor backend as the `lantern_editor_backend` target: engine/editor-backend (Nox) by default,
# or crates/editor-backend built with Cargo when LANTERN_EDITOR_BACKEND is rust.
#
# The Rust backend links the system SDL3 (its `system-sdl` feature, found through pkg-config), the
# same SDL Penumbra uses, so the process has one SDL. Set PKG_CONFIG_PATH if SDL3 is installed
# somewhere pkg-config doesn't look by default.

set(LANTERN_EDITOR_BACKEND "nox" CACHE STRING "The editor backend to link: nox (engine/editor-backend) or rust (crates/editor-backend)")
set_property(CACHE LANTERN_EDITOR_BACKEND PROPERTY STRINGS nox rust)

get_filename_component(LANTERN_WORKSPACE_ROOT "${CMAKE_CURRENT_LIST_DIR}/../../.." ABSOLUTE)

if(LANTERN_EDITOR_BACKEND STREQUAL "nox")
    add_subdirectory("${LANTERN_WORKSPACE_ROOT}/engine/editor-backend" "${CMAKE_CURRENT_BINARY_DIR}/editor-backend")
    return()
elseif(NOT LANTERN_EDITOR_BACKEND STREQUAL "rust")
    message(FATAL_ERROR "LANTERN_EDITOR_BACKEND is '${LANTERN_EDITOR_BACKEND}'; use nox or rust")
endif()

find_program(CARGO_EXECUTABLE cargo REQUIRED)

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

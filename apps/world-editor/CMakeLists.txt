cmake_minimum_required(VERSION 3.30)
project(lantern_world_editor LANGUAGES C CXX)

# Lantern's world editor. The front end is Penumbra, Chaos, Cosmos and Nyx (src/ui), the same
# stack as Pharos; the editing logic and the 3D viewport are Rust (crates/editor-backend),
# linked as a static library through its C API (crates/editor-backend/include/lantern_editor.h).
# See docs/editor.md.

if((CMAKE_CXX_COMPILER_ID STREQUAL "GNU" AND CMAKE_CXX_COMPILER_VERSION VERSION_LESS 14)
   OR (CMAKE_CXX_COMPILER_ID STREQUAL "Clang" AND CMAKE_CXX_COMPILER_VERSION VERSION_LESS 19))
  message(FATAL_ERROR "The world editor needs GCC 14 or later or Clang 19 or later; found ${CMAKE_CXX_COMPILER_ID} ${CMAKE_CXX_COMPILER_VERSION}")
endif()

if(NOT CMAKE_BUILD_TYPE)
    set(CMAKE_BUILD_TYPE Debug CACHE STRING "Build type" FORCE)
endif()

set(CMAKE_CXX_STANDARD 26)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

include(cmake/Dependencies.cmake)
include(cmake/EditorBackend.cmake)

# src/ui holds only Chaos, Cosmos and Nyx. The native code behind the viewports (the level view,
# the template preview and the thumbnails) lives in src/native.
file(GLOB_RECURSE WORLD_EDITOR_NATIVE_UI_SOURCES CONFIGURE_DEPENDS
    "${CMAKE_CURRENT_SOURCE_DIR}/src/ui/*.cpp" "${CMAKE_CURRENT_SOURCE_DIR}/src/ui/*.h")
if(WORLD_EDITOR_NATIVE_UI_SOURCES)
    message(FATAL_ERROR "src/ui/ holds C++: ${WORLD_EDITOR_NATIVE_UI_SOURCES}. "
                        "Write UI in Chaos, Cosmos and Nyx; native widgets go in src/native/.")
endif()

get_filename_component(LANTERN_REPOSITORY_ROOT "${CMAKE_CURRENT_SOURCE_DIR}/../.." ABSOLUTE)

add_executable(lantern-world-editor
    src/main.cpp
    src/EditorHost.cpp
    src/native/EditorViewportRenderers.cpp
)
target_include_directories(lantern-world-editor PRIVATE src)
target_link_libraries(lantern-world-editor PRIVATE penumbra_ui_backend_host lantern_editor_backend)
target_compile_options(lantern-world-editor PRIVATE
    $<$<CXX_COMPILER_ID:GNU,Clang,AppleClang>:-Wall -Wextra -Wpedantic>)
target_compile_definitions(lantern-world-editor PRIVATE
    WORLD_EDITOR_UI_DIR="${CMAKE_CURRENT_SOURCE_DIR}/src/ui"
    WORLD_EDITOR_FONT_PATH="${CMAKE_CURRENT_SOURCE_DIR}/data/fonts/Inter-Regular.ttf"
    LANTERN_ASSETS_DIR="${LANTERN_REPOSITORY_ROOT}/assets")

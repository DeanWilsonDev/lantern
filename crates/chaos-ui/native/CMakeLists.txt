cmake_minimum_required(VERSION 3.30)

# The native half of Lantern's in-game UI: Nyx (with its C API), Chaos, Cosmos and the bridge
# between them and the engine (include/lantern_chaos_ui.h), in one build so there is exactly
# one copy of Nyx. nyx-sys's build script configures this project instead of Nyx's own when its
# `chaos-ui` feature is on (crates/nyx-sys/build.rs), and links every archive it makes.
#
# Sources: LANTERN_NYX_SOURCE_DIR, LANTERN_CHAOS_SOURCE_DIR and LANTERN_COSMOS_SOURCE_DIR name
# local checkouts (with their submodules); an empty one is fetched from GitHub on `proto`.
project(LanternChaosUi LANGUAGES C CXX)

set(CMAKE_CXX_STANDARD 26)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)
# Linked into Rust executables, which are position independent.
set(CMAKE_POSITION_INDEPENDENT_CODE ON)

set(LANTERN_NYX_SOURCE_DIR "" CACHE PATH "A nyx-scripting-language checkout; empty fetches proto")
set(LANTERN_CHAOS_SOURCE_DIR "" CACHE PATH "A chaos checkout; empty fetches proto")
set(LANTERN_COSMOS_SOURCE_DIR "" CACHE PATH "A cosmos checkout; empty fetches proto")

set(NYX_BUILD_CAPI ON CACHE BOOL "" FORCE)
set(NYX_BUILD_TESTS OFF CACHE BOOL "" FORCE)
set(NYX_BUILD_TOOLS OFF CACHE BOOL "" FORCE)
set(NYX_BUILD_EXAMPLES OFF CACHE BOOL "" FORCE)
set(NYX_BUILD_LSP_LIBRARY OFF CACHE BOOL "" FORCE)
set(CHAOS_BUILD_TESTS OFF CACHE BOOL "" FORCE)
set(CHAOS_BUILD_LSP OFF CACHE BOOL "" FORCE)
set(COSMOS_BUILD_TESTS OFF CACHE BOOL "" FORCE)
set(COSMOS_BUILD_LSP OFF CACHE BOOL "" FORCE)

include(FetchContent)
cmake_policy(SET CMP0168 NEW)

# Nyx is declared first, so Chaos's own declaration of it (on `proto`) is ignored and both use
# this one.
if(LANTERN_NYX_SOURCE_DIR)
    set(FETCHCONTENT_SOURCE_DIR_NYX "${LANTERN_NYX_SOURCE_DIR}")
endif()
FetchContent_Declare(nyx
    GIT_REPOSITORY https://github.com/DeanWilsonDev/nyx-scripting-language.git
    GIT_TAG        e33b23f536dc9aab40269cef4285089386cead20
    GIT_SUBMODULES external/firefly external/lucis-proto
    GIT_SUBMODULES_RECURSE TRUE)
FetchContent_MakeAvailable(nyx)

if(LANTERN_CHAOS_SOURCE_DIR)
    set(FETCHCONTENT_SOURCE_DIR_CHAOS "${LANTERN_CHAOS_SOURCE_DIR}")
endif()
FetchContent_Declare(chaos
    GIT_REPOSITORY https://github.com/DeanWilsonDev/chaos.git
    GIT_TAG        a2de61b9ae94f830551c0cd414fc007aa4b44d56
    EXCLUDE_FROM_ALL)
FetchContent_MakeAvailable(chaos)

if(LANTERN_COSMOS_SOURCE_DIR)
    set(FETCHCONTENT_SOURCE_DIR_COSMOS "${LANTERN_COSMOS_SOURCE_DIR}")
endif()
FetchContent_Declare(cosmos
    GIT_REPOSITORY https://github.com/DeanWilsonDev/cosmos.git
    GIT_TAG        7dabd4d2c66d18517716f4e1a5891f31cce66a32
    EXCLUDE_FROM_ALL)
FetchContent_MakeAvailable(cosmos)

add_library(lantern_chaos_ui STATIC src/ChaosUiBridge.cpp)
target_include_directories(lantern_chaos_ui PUBLIC include)
target_link_libraries(lantern_chaos_ui PUBLIC chaos cosmos nyx-capi-static)

# Everything nyx-sys links, built by one target.
add_custom_target(lantern_native_stack ALL DEPENDS lantern_chaos_ui chaos cosmos nyx-capi-static)

include(FetchContent)

# Every dependency tracks a branch rather than a pin: they're co-developed with the editor, so a pin
# would need a bump commit here for every upstream change. CMP0168 OLD makes each fetch a
# sub-build whose git update step runs on every configure, so an existing build directory moves
# to its branch's head too; under NEW it stayed on the commits it first cloned. To configure
# offline, pass -DFETCHCONTENT_UPDATES_DISCONNECTED=ON (it stays in the cache until set OFF).
cmake_policy(PUSH)
cmake_policy(SET CMP0168 OLD)

# Penumbra links SDL3_image privately from a static library, so its imported target has to be
# visible at this level too.
find_package(SDL3       CONFIG REQUIRED)
find_package(SDL3_ttf   CONFIG REQUIRED)
find_package(SDL3_image CONFIG REQUIRED)

# Added first so the `cimmerian` target is this copy on `main`: Chaos and Cosmos each add a
# vendored copy, which returns early once the target exists. FORCE, because their option()
# calls have already cached these as OFF in any build directory configured before.
set(CIMMERIAN_ENABLE_SNAPSHOT_TESTING ON CACHE BOOL
    "Enable Cimmerian snapshot testing" FORCE)
set(CIMMERIAN_ENABLE_VISUAL_TESTING ON CACHE BOOL
    "Enable Cimmerian visual regression testing" FORCE)

FetchContent_Declare(
    cimmerian
    GIT_REPOSITORY https://github.com/DeanWilsonDev/cimmerian.git
    GIT_TAG        main
    GIT_SHALLOW    TRUE
    GIT_SUBMODULES ""
    EXCLUDE_FROM_ALL
    SYSTEM
)
FetchContent_MakeAvailable(cimmerian)

FetchContent_Declare(
    penumbra
    GIT_REPOSITORY https://github.com/DeanWilsonDev/penumbra.git
    GIT_TAG        proto
    GIT_SHALLOW    TRUE
    EXCLUDE_FROM_ALL
    SYSTEM
)
FetchContent_MakeAvailable(penumbra)

# Chaos adds nyx-proto (on `proto`) itself, along with Amanuensis, Firefly and Lucis; the editor
# links the `nyx-core` and `nyx-json` targets that creates.
FetchContent_Declare(
    chaos
    GIT_REPOSITORY https://github.com/DeanWilsonDev/chaos.git
    GIT_TAG        proto
    GIT_SHALLOW    TRUE
    EXCLUDE_FROM_ALL
    SYSTEM
)
FetchContent_MakeAvailable(chaos)

FetchContent_Declare(
    cosmos
    GIT_REPOSITORY https://github.com/DeanWilsonDev/cosmos.git
    GIT_TAG        proto
    GIT_SHALLOW    TRUE
    EXCLUDE_FROM_ALL
    SYSTEM
)
FetchContent_MakeAvailable(cosmos)

# penumbra-ui-backend's own CMakeLists.txt adds vendored copies of Chaos, Cosmos and Penumbra,
# which would redeclare those targets. So its source is only populated (SOURCE_SUBDIR names a
# directory that doesn't exist) and its library sources are compiled here against the targets
# above. Its submodules are those vendored copies, so they aren't fetched.
FetchContent_Declare(
    penumbra_ui_backend_src
    GIT_REPOSITORY https://github.com/DeanWilsonDev/penumbra-ui-backend.git
    GIT_TAG        main
    GIT_SHALLOW    TRUE
    GIT_SUBMODULES ""
    SOURCE_SUBDIR  world-editor-populate-source-only
)
FetchContent_MakeAvailable(penumbra_ui_backend_src)

add_library(penumbra_ui_backend_cosmos STATIC
    ${penumbra_ui_backend_src_SOURCE_DIR}/src/PenumbraUiBackend/Cosmos/StyleApplier.cpp
    ${penumbra_ui_backend_src_SOURCE_DIR}/src/PenumbraUiBackend/Cosmos/StyleResolution.cpp
    ${penumbra_ui_backend_src_SOURCE_DIR}/src/PenumbraUiBackend/Cosmos/StylesheetLoader.cpp)
target_include_directories(penumbra_ui_backend_cosmos PUBLIC ${penumbra_ui_backend_src_SOURCE_DIR}/include)
target_compile_features(penumbra_ui_backend_cosmos PUBLIC cxx_std_26)
target_link_libraries(penumbra_ui_backend_cosmos PUBLIC cosmos penumbra)

add_library(penumbra_ui_backend STATIC
    ${penumbra_ui_backend_src_SOURCE_DIR}/src/PenumbraUiBackend/NyxApplicationBridge.cpp
    ${penumbra_ui_backend_src_SOURCE_DIR}/src/PenumbraUiBackend/ChaosApplication.cpp
    ${penumbra_ui_backend_src_SOURCE_DIR}/src/PenumbraUiBackend/ScriptCanvas.cpp
    ${penumbra_ui_backend_src_SOURCE_DIR}/src/PenumbraUiBackend/Walker.cpp
    ${penumbra_ui_backend_src_SOURCE_DIR}/src/PenumbraUiBackend/PenumbraWidgetAdapter.cpp)
target_include_directories(penumbra_ui_backend PUBLIC ${penumbra_ui_backend_src_SOURCE_DIR}/include)
target_compile_features(penumbra_ui_backend PUBLIC cxx_std_26)
target_link_libraries(penumbra_ui_backend PUBLIC chaos penumbra penumbra_ui_backend_cosmos)

add_library(penumbra_ui_backend_host STATIC
    ${penumbra_ui_backend_src_SOURCE_DIR}/src/PenumbraUiBackend/ChaosApplicationHost.cpp)
target_include_directories(penumbra_ui_backend_host PUBLIC ${penumbra_ui_backend_src_SOURCE_DIR}/include)
target_compile_features(penumbra_ui_backend_host PUBLIC cxx_std_26)
target_link_libraries(penumbra_ui_backend_host PUBLIC penumbra_ui_backend penumbra_icons nyx-json)

cmake_policy(POP)

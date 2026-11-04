include(FetchContent)

# Every dependency tracks a branch rather than a pin: they're co-developed with the editor, so a pin
# would need a bump commit here for every upstream change. CMP0168 OLD makes each fetch a
# sub-build whose git update step runs on every configure, so an existing build directory moves
# to its branch's head too; under NEW it stayed on the commits it first cloned. To configure
# offline, pass -DFETCHCONTENT_UPDATES_DISCONNECTED=ON (it stays in the cache until set OFF).
cmake_policy(PUSH)
cmake_policy(SET CMP0168 OLD)

# The branch each dependency tracks. A change that spans the stack is developed on one branch
# name in every repository, so -DLANTERN_STACK_BRANCH=<name> moves every dependency that has
# that branch to it; a single LANTERN_<DEP>_BRANCH overrides one. To build against local
# checkouts instead, set FETCHCONTENT_SOURCE_DIR_<DEP> (CMake's own override) per dependency.
set(LANTERN_STACK_BRANCH "" CACHE STRING
    "Branch to fetch every co-developed dependency from; empty means each one's own default")
function(lantern_dependency_branch OUTPUT_VARIABLE DEPENDENCY_NAME DEFAULT_BRANCH)
    string(TOUPPER "${DEPENDENCY_NAME}" UPPER_NAME)
    set(LANTERN_${UPPER_NAME}_BRANCH "" CACHE STRING "Branch to fetch ${DEPENDENCY_NAME} from")
    if(LANTERN_${UPPER_NAME}_BRANCH)
        set(${OUTPUT_VARIABLE} "${LANTERN_${UPPER_NAME}_BRANCH}" PARENT_SCOPE)
    elseif(LANTERN_STACK_BRANCH)
        set(${OUTPUT_VARIABLE} "${LANTERN_STACK_BRANCH}" PARENT_SCOPE)
    else()
        set(${OUTPUT_VARIABLE} "${DEFAULT_BRANCH}" PARENT_SCOPE)
    endif()
endfunction()
lantern_dependency_branch(LANTERN_CIMMERIAN_GIT_TAG cimmerian main)
lantern_dependency_branch(LANTERN_PENUMBRA_GIT_TAG penumbra proto)
lantern_dependency_branch(LANTERN_CHAOS_GIT_TAG chaos proto)
lantern_dependency_branch(LANTERN_COSMOS_GIT_TAG cosmos proto)
lantern_dependency_branch(LANTERN_PENUMBRA_UI_BACKEND_GIT_TAG penumbra_ui_backend main)

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
    GIT_REPOSITORY https://github.com/fearlessgamedev/cimmerian-testing-framework.git
    GIT_TAG        ${LANTERN_CIMMERIAN_GIT_TAG}
    GIT_SHALLOW    TRUE
    GIT_SUBMODULES ""
    EXCLUDE_FROM_ALL
    SYSTEM
)
FetchContent_MakeAvailable(cimmerian)

FetchContent_Declare(
    penumbra
    GIT_REPOSITORY https://github.com/fearlessgamedev/penumbra-ui-engine.git
    GIT_TAG        ${LANTERN_PENUMBRA_GIT_TAG}
    GIT_SHALLOW    TRUE
    EXCLUDE_FROM_ALL
    SYSTEM
)
FetchContent_MakeAvailable(penumbra)

# Chaos adds nyx-proto (on `proto`) itself, along with Amanuensis, Firefly and Lucis; the editor
# links the `nyx-core` target that creates.
FetchContent_Declare(
    chaos
    GIT_REPOSITORY https://github.com/fearlessgamedev/chaos-ui-framework.git
    GIT_TAG        ${LANTERN_CHAOS_GIT_TAG}
    GIT_SHALLOW    TRUE
    EXCLUDE_FROM_ALL
    SYSTEM
)
FetchContent_MakeAvailable(chaos)

FetchContent_Declare(
    cosmos
    GIT_REPOSITORY https://github.com/fearlessgamedev/cosmos-styling-language.git
    GIT_TAG        ${LANTERN_COSMOS_GIT_TAG}
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
    GIT_REPOSITORY https://github.com/fearlessgamedev/penumbra-bridge.git
    GIT_TAG        ${LANTERN_PENUMBRA_UI_BACKEND_GIT_TAG}
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
    ${penumbra_ui_backend_src_SOURCE_DIR}/src/PenumbraUiBackend/PenumbraWidgetAdapter.cpp
    ${penumbra_ui_backend_src_SOURCE_DIR}/src/PenumbraUiBackend/ViewportHostWidget.cpp)
target_include_directories(penumbra_ui_backend PUBLIC ${penumbra_ui_backend_src_SOURCE_DIR}/include)
target_compile_features(penumbra_ui_backend PUBLIC cxx_std_26)
target_link_libraries(penumbra_ui_backend PUBLIC chaos penumbra penumbra_ui_backend_cosmos)

add_library(penumbra_ui_backend_host STATIC
    ${penumbra_ui_backend_src_SOURCE_DIR}/src/PenumbraUiBackend/ChaosApplicationHost.cpp)
target_include_directories(penumbra_ui_backend_host PUBLIC ${penumbra_ui_backend_src_SOURCE_DIR}/include)
target_compile_features(penumbra_ui_backend_host PUBLIC cxx_std_26)
target_link_libraries(penumbra_ui_backend_host PUBLIC penumbra_ui_backend penumbra_icons)

cmake_policy(POP)

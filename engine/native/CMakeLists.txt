cmake_minimum_required(VERSION 3.30)
project(LanternNative LANGUAGES C CXX)

set(CMAKE_CXX_STANDARD 26)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)
set(CMAKE_POSITION_INDEPENDENT_CODE ON)

set(LANTERN_NYX_SOURCE_DIR "${CMAKE_CURRENT_SOURCE_DIR}/../../../nyx-scripting-language" CACHE PATH "A nyx-scripting-language checkout on proto")
set(LANTERN_CHAOS_SOURCE_DIR "${CMAKE_CURRENT_SOURCE_DIR}/../../../chaos-ui-framework" CACHE PATH "A chaos-ui-framework checkout on proto")
set(LANTERN_COSMOS_SOURCE_DIR "${CMAKE_CURRENT_SOURCE_DIR}/../../../cosmos-styling-language" CACHE PATH "A cosmos-styling-language checkout on proto")
set(LANTERN_PENUMBRA_BRIDGE_SOURCE_DIR "${CMAKE_CURRENT_SOURCE_DIR}/../../../penumbra-bridge" CACHE PATH "A penumbra-bridge checkout on main")

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
set(FETCHCONTENT_SOURCE_DIR_NYX "${LANTERN_NYX_SOURCE_DIR}")
FetchContent_Declare(nyx GIT_REPOSITORY https://github.com/fearlessgamedev/nyx-scripting-language.git GIT_TAG proto)
FetchContent_MakeAvailable(nyx)
set(FETCHCONTENT_SOURCE_DIR_CHAOS "${LANTERN_CHAOS_SOURCE_DIR}")
FetchContent_Declare(chaos GIT_REPOSITORY https://github.com/fearlessgamedev/chaos-ui-framework.git GIT_TAG proto EXCLUDE_FROM_ALL)
FetchContent_MakeAvailable(chaos)
set(FETCHCONTENT_SOURCE_DIR_COSMOS "${LANTERN_COSMOS_SOURCE_DIR}")
FetchContent_Declare(cosmos GIT_REPOSITORY https://github.com/fearlessgamedev/cosmos-styling-language.git GIT_TAG proto EXCLUDE_FROM_ALL)
FetchContent_MakeAvailable(cosmos)

add_library(lantern-native SHARED
    src/ChaosUiBridge.cpp
    "${LANTERN_PENUMBRA_BRIDGE_SOURCE_DIR}/capi/src/BridgeNode.cpp"
    "${LANTERN_PENUMBRA_BRIDGE_SOURCE_DIR}/capi/src/ChangeLog.cpp"
    "${LANTERN_PENUMBRA_BRIDGE_SOURCE_DIR}/capi/src/PenumbraBridge.cpp"
    "${LANTERN_PENUMBRA_BRIDGE_SOURCE_DIR}/capi/src/StyleExport.cpp")
target_include_directories(lantern-native PUBLIC include "${LANTERN_PENUMBRA_BRIDGE_SOURCE_DIR}/capi/include")
target_include_directories(lantern-native PRIVATE "${LANTERN_PENUMBRA_BRIDGE_SOURCE_DIR}/capi/src" "${LANTERN_PENUMBRA_BRIDGE_SOURCE_DIR}/include")
set_source_files_properties(
    "${LANTERN_PENUMBRA_BRIDGE_SOURCE_DIR}/capi/src/BridgeNode.cpp"
    "${LANTERN_PENUMBRA_BRIDGE_SOURCE_DIR}/capi/src/ChangeLog.cpp"
    "${LANTERN_PENUMBRA_BRIDGE_SOURCE_DIR}/capi/src/PenumbraBridge.cpp"
    "${LANTERN_PENUMBRA_BRIDGE_SOURCE_DIR}/capi/src/StyleExport.cpp"
    PROPERTIES COMPILE_DEFINITIONS PENUMBRA_BRIDGE_BUILDING)
set_target_properties(lantern-native PROPERTIES INSTALL_NAME_DIR "${CMAKE_CURRENT_BINARY_DIR}" BUILD_WITH_INSTALL_NAME_DIR ON)
target_link_libraries(lantern-native PRIVATE chaos cosmos "$<LINK_LIBRARY:WHOLE_ARCHIVE,nyx-capi-static>")
if(APPLE)
    target_link_options(lantern-native PRIVATE "LINKER:-exported_symbols_list,${CMAKE_CURRENT_SOURCE_DIR}/lantern-native.exports")
    set_property(TARGET lantern-native APPEND PROPERTY LINK_DEPENDS "${CMAKE_CURRENT_SOURCE_DIR}/lantern-native.exports")
elseif(UNIX)
    target_link_options(lantern-native PRIVATE "LINKER:--version-script=${CMAKE_CURRENT_SOURCE_DIR}/lantern-native.map")
    set_property(TARGET lantern-native APPEND PROPERTY LINK_DEPENDS "${CMAKE_CURRENT_SOURCE_DIR}/lantern-native.map")
endif()

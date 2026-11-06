cmake_minimum_required(VERSION 3.30)

project(lantern_editor_backend LANGUAGES C)

find_program(NOX_EXECUTABLE nox REQUIRED)
if(NOT TARGET SDL3::SDL3)
    find_package(SDL3 REQUIRED CONFIG)
endif()

get_filename_component(LANTERN_ENGINE_DIRECTORY "${CMAKE_CURRENT_LIST_DIR}/.." ABSOLUTE)
get_filename_component(LANTERN_REPOSITORY_DIRECTORY "${LANTERN_ENGINE_DIRECTORY}/.." ABSOLUTE)
find_program(MIST_EXECUTABLE mist REQUIRED)
execute_process(
    COMMAND "${MIST_EXECUTABLE}" resolve --json --manifest "${LANTERN_REPOSITORY_DIRECTORY}/mist.json"
    OUTPUT_VARIABLE LANTERN_MIST_RESOLVE_JSON
    RESULT_VARIABLE LANTERN_MIST_RESOLVE_RESULT)
if(NOT LANTERN_MIST_RESOLVE_RESULT EQUAL 0)
    message(FATAL_ERROR "mist resolve failed: run mist install in the repository root first")
endif()
set_property(DIRECTORY APPEND PROPERTY CMAKE_CONFIGURE_DEPENDS
    "${LANTERN_REPOSITORY_DIRECTORY}/mist.lock" "${LANTERN_REPOSITORY_DIRECTORY}/mist_packages/.mist-install.json")
string(JSON LANTERN_MIST_PACKAGE_COUNT LENGTH "${LANTERN_MIST_RESOLVE_JSON}" packages)
math(EXPR LANTERN_MIST_LAST_PACKAGE "${LANTERN_MIST_PACKAGE_COUNT} - 1")
set(LANTERN_MIST_LIBRARIES "")
foreach(package_index RANGE ${LANTERN_MIST_LAST_PACKAGE})
    string(JSON package_name GET "${LANTERN_MIST_RESOLVE_JSON}" packages ${package_index} name)
    if(NOT package_name STREQUAL "cgltf" AND NOT package_name STREQUAL "stb_image")
        continue()
    endif()
    string(JSON package_library_directory GET "${LANTERN_MIST_RESOLVE_JSON}" packages ${package_index} libraryDirectories 0)
    string(JSON package_library_name GET "${LANTERN_MIST_RESOLVE_JSON}" packages ${package_index} libraries 0)
    find_library(LANTERN_MIST_LIBRARY_${package_name} NAMES ${package_library_name}
                 PATHS "${package_library_directory}" NO_DEFAULT_PATH NO_CACHE REQUIRED)
    list(APPEND LANTERN_MIST_LIBRARIES "${LANTERN_MIST_LIBRARY_${package_name}}")
endforeach()

file(GLOB_RECURSE LANTERN_EDITOR_NOX_SOURCES CONFIGURE_DEPENDS
    "${CMAKE_CURRENT_LIST_DIR}/src/*.nox"
    "${LANTERN_ENGINE_DIRECTORY}/src/core/*.nox"
    "${LANTERN_ENGINE_DIRECTORY}/src/editor/*.nox"
    "${LANTERN_ENGINE_DIRECTORY}/src/editorbackend/*.nox"
    "${LANTERN_ENGINE_DIRECTORY}/src/gpu/*.nox"
    "${LANTERN_ENGINE_DIRECTORY}/src/physics/*.nox"
    "${LANTERN_ENGINE_DIRECTORY}/src/platform/*.nox"
    "${LANTERN_ENGINE_DIRECTORY}/src/renderer/*.nox"
    "${LANTERN_ENGINE_DIRECTORY}/src/world/*.nox")

if(CMAKE_BUILD_TYPE STREQUAL "Debug")
    set(LANTERN_NOX_BUILD_FLAGS "")
else()
    set(LANTERN_NOX_BUILD_FLAGS "--release")
endif()

set(LANTERN_EDITOR_NOX_DIRECTORY "${CMAKE_CURRENT_BINARY_DIR}/nox")
set(LANTERN_EDITOR_NOX_LIBRARY "${LANTERN_EDITOR_NOX_DIRECTORY}/liblantern_editor_nox.a")
add_custom_command(
    OUTPUT "${LANTERN_EDITOR_NOX_LIBRARY}" "${LANTERN_EDITOR_NOX_DIRECTORY}/liblantern_editor_nox.h"
    COMMAND "${CMAKE_COMMAND}" -E make_directory "${LANTERN_EDITOR_NOX_DIRECTORY}"
    COMMAND "${NOX_EXECUTABLE}" build --project "${CMAKE_CURRENT_LIST_DIR}" --static-library ${LANTERN_NOX_BUILD_FLAGS}
            -o "${LANTERN_EDITOR_NOX_LIBRARY}"
    DEPENDS ${LANTERN_EDITOR_NOX_SOURCES} "${CMAKE_CURRENT_LIST_DIR}/nox.json"
    COMMENT "Building the Nox editor backend"
    VERBATIM)
add_custom_target(lantern_editor_nox DEPENDS "${LANTERN_EDITOR_NOX_LIBRARY}")

add_library(lantern_editor_nox_library STATIC IMPORTED GLOBAL)
set_target_properties(lantern_editor_nox_library PROPERTIES IMPORTED_LOCATION "${LANTERN_EDITOR_NOX_LIBRARY}")
add_dependencies(lantern_editor_nox_library lantern_editor_nox)

add_library(lantern_editor_backend STATIC shim/lantern_editor.c)
add_dependencies(lantern_editor_backend lantern_editor_nox)
target_include_directories(lantern_editor_backend
    PUBLIC "${CMAKE_CURRENT_LIST_DIR}/include"
    PRIVATE "${LANTERN_EDITOR_NOX_DIRECTORY}")
target_compile_features(lantern_editor_backend PRIVATE c_std_11)
target_compile_definitions(lantern_editor_backend PRIVATE
    "LANTERN_EDITOR_SHADER_DIRECTORY=\"${LANTERN_ENGINE_DIRECTORY}/shaders/compiled/\"")
target_link_libraries(lantern_editor_backend PUBLIC
    lantern_editor_nox_library
    ${LANTERN_MIST_LIBRARIES}
    SDL3::SDL3)
if(NOT WIN32)
    target_link_libraries(lantern_editor_backend PUBLIC m)
endif()

if(PROJECT_IS_TOP_LEVEL)
    enable_testing()
    add_executable(lantern_editor_tests tests/lantern_editor_test.c)
    target_compile_features(lantern_editor_tests PRIVATE c_std_11)
    target_link_libraries(lantern_editor_tests PRIVATE lantern_editor_backend)
    get_filename_component(LANTERN_ASSETS_DIRECTORY "${LANTERN_ENGINE_DIRECTORY}/../assets" ABSOLUTE)
    add_test(NAME lantern_editor_tests COMMAND lantern_editor_tests "${LANTERN_ASSETS_DIRECTORY}")
endif()

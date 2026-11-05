cmake_minimum_required(VERSION 3.30)

project(lantern_editor_backend LANGUAGES C)

find_program(NOX_EXECUTABLE nox REQUIRED)
if(NOT TARGET SDL3::SDL3)
    find_package(SDL3 REQUIRED CONFIG)
endif()

get_filename_component(LANTERN_ENGINE_DIRECTORY "${CMAKE_CURRENT_LIST_DIR}/.." ABSOLUTE)
set(LANTERN_GENERATED_DIRECTORY "${LANTERN_ENGINE_DIRECTORY}/generated")
foreach(generated_file sdl.nox mist/libcgltf.a mist/libstb_image.a)
    if(NOT EXISTS "${LANTERN_GENERATED_DIRECTORY}/${generated_file}")
        message(FATAL_ERROR "engine/generated/${generated_file} is missing: run engine/tools/bind-sdl.sh and "
                            "engine/tools/install-dependencies.sh first")
    endif()
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
    "${LANTERN_GENERATED_DIRECTORY}/mist/libcgltf.a"
    "${LANTERN_GENERATED_DIRECTORY}/mist/libstb_image.a"
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

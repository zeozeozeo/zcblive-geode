cmake_minimum_required(VERSION 3.21)
set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_OSX_ARCHITECTURES "x86_64")
set(CMAKE_CXX_VISIBILITY_PRESET hidden)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON) # generate compile_commands.json for clangd
set(GEODE_TARGET_PLATFORM "Win32")

project(zcblive VERSION 1.0.0)

# Set up the mod binary
add_library(${PROJECT_NAME} SHARED src/main.cpp)

# force 32-bit on 64-bit clang
set(CMAKE_C_FLAGS -m32)
set(CMAKE_CXX_FLAGS -m32)

if (NOT DEFINED ENV{GEODE_SDK})
    message(FATAL_ERROR "Unable to find Geode SDK! Please define GEODE_SDK environment variable to point to Geode")
else()
    message(STATUS "Found Geode: $ENV{GEODE_SDK}")
endif()

add_subdirectory($ENV{GEODE_SDK} ${CMAKE_CURRENT_BINARY_DIR}/geode)

# Set up dependencies, resources, link Geode
setup_geode_mod(${PROJECT_NAME})
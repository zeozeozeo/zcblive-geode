cmake_minimum_required(VERSION 3.21)
project(zcblive_rust)

# Location of the Rust library
set(RUSTLIB_DIR ${CMAKE_CURRENT_SOURCE_DIR})

# Name of the Rust library (not ${PROJECT_NAME})
set(RUSTLIB_NAME zcblive)

# Path to the Rust library
set(RUSTLIB_PATH ${RUSTLIB_DIR}/target/release/${RUSTLIB_NAME}.lib)

include(CMakePrintHelpers)

cmake_print_variables(RUSTLIB_PATH)

# Add the Rust library
add_library(${PROJECT_NAME} STATIC IMPORTED)
set_target_properties(${PROJECT_NAME} PROPERTIES IMPORTED_LOCATION_RELEASE ${RUSTLIB_PATH})

# Add a custom command to build the Rust library using cargo
add_custom_command(
    OUTPUT ${RUSTLIB_PATH}
    COMMAND cargo build --release --features geode
    WORKING_DIRECTORY ${RUSTLIB_DIR}
    COMMENT "Building Rust library"
)

# Add a custom target to trigger the custom command
add_custom_target(${PROJECT_NAME}_build ALL DEPENDS ${RUSTLIB_PATH})

# Make sure the Rust library is built before anything else
add_dependencies(${PROJECT_NAME} ${PROJECT_NAME}_build)

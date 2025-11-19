cmake_minimum_required(VERSION 3.20)
project(WindjammerSDK VERSION 0.1.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

# Library
add_library(windjammer_sdk INTERFACE)
target_include_directories(windjammer_sdk INTERFACE
    $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
    $<INSTALL_INTERFACE:include>
)

# Examples
option(BUILD_EXAMPLES "Build example programs" ON)
if(BUILD_EXAMPLES)
    add_subdirectory(examples)
endif()

# Installation
install(TARGETS windjammer_sdk
    EXPORT WindjammerSDKTargets
    LIBRARY DESTINATION lib
    ARCHIVE DESTINATION lib
    RUNTIME DESTINATION bin
    INCLUDES DESTINATION include
)

install(DIRECTORY include/windjammer
    DESTINATION include
)

install(EXPORT WindjammerSDKTargets
    FILE WindjammerSDKTargets.cmake
    NAMESPACE Windjammer::
    DESTINATION lib/cmake/WindjammerSDK
)

# Package config
include(CMakePackageConfigHelpers)
write_basic_package_version_file(
    "${CMAKE_CURRENT_BINARY_DIR}/WindjammerSDKConfigVersion.cmake"
    VERSION ${PROJECT_VERSION}
    COMPATIBILITY AnyNewerVersion
)

configure_package_config_file(
    "${CMAKE_CURRENT_SOURCE_DIR}/cmake/WindjammerSDKConfig.cmake.in"
    "${CMAKE_CURRENT_BINARY_DIR}/WindjammerSDKConfig.cmake"
    INSTALL_DESTINATION lib/cmake/WindjammerSDK
)

install(FILES
    "${CMAKE_CURRENT_BINARY_DIR}/WindjammerSDKConfig.cmake"
    "${CMAKE_CURRENT_BINARY_DIR}/WindjammerSDKConfigVersion.cmake"
    DESTINATION lib/cmake/WindjammerSDK
)


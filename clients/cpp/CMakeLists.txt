cmake_minimum_required(VERSION 3.16)
project(fusionserve_cpp_client LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

if(NOT CMAKE_BUILD_TYPE)
  set(CMAKE_BUILD_TYPE Release)
endif()

add_compile_options(-Wall -Wextra -Wpedantic)

# Optional sanitizers: -DENABLE_ASAN=ON / -DENABLE_TSAN=ON
option(ENABLE_ASAN "Build with AddressSanitizer" OFF)
option(ENABLE_TSAN "Build with ThreadSanitizer" OFF)
if(ENABLE_ASAN)
  add_compile_options(-fsanitize=address -fno-omit-frame-pointer)
  add_link_options(-fsanitize=address)
endif()
if(ENABLE_TSAN)
  add_compile_options(-fsanitize=thread)
  add_link_options(-fsanitize=thread)
endif()

find_package(CURL REQUIRED)

add_library(fusionserve_client src/fusionserve_client.cpp)
target_include_directories(fusionserve_client PUBLIC include)
target_link_libraries(fusionserve_client PUBLIC CURL::libcurl)

add_executable(fusionserve_demo src/main.cpp)
target_link_libraries(fusionserve_demo PRIVATE fusionserve_client)

# ---- Tests (GoogleTest via FetchContent) ----------------------------------
option(BUILD_TESTING "Build tests" ON)
if(BUILD_TESTING)
  include(FetchContent)
  FetchContent_Declare(
    googletest
    URL https://github.com/google/googletest/archive/refs/tags/v1.15.2.zip
  )
  set(gtest_force_shared_crt ON CACHE BOOL "" FORCE)
  FetchContent_MakeAvailable(googletest)

  enable_testing()
  add_executable(client_tests tests/client_test.cpp)
  target_link_libraries(client_tests PRIVATE fusionserve_client GTest::gtest)
  include(GoogleTest)
  gtest_discover_tests(client_tests)
endif()

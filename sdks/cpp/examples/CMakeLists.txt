cmake_minimum_required(VERSION 3.20)

# Hello World example
add_executable(hello_world hello_world.cpp)
target_link_libraries(hello_world PRIVATE windjammer_sdk)

# Math demo example
add_executable(math_demo math_demo.cpp)
target_link_libraries(math_demo PRIVATE windjammer_sdk)


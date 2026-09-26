cmake_minimum_required(VERSION 3.25)
project(randcre LANGUAGES C CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

if(NOT CMAKE_BUILD_TYPE)
  set(CMAKE_BUILD_TYPE RelWithDebInfo CACHE STRING "" FORCE)
endif()

add_compile_options(-Wall -Wextra -Wno-unused-parameter)

# Shared low-level utilities: byte buffers, binary readers, filesystem helpers.
add_subdirectory(src/core)

# VU microprogram disassembler (OpenGOAL, ISC) and the tool around it.
add_subdirectory(third_party/opengoal_vudisasm)
add_subdirectory(tools/vu)

# Disc/asset extraction tool. Reads the user's own ISO, never ships data.
add_subdirectory(tools/extract)

enable_testing()
add_subdirectory(tests)

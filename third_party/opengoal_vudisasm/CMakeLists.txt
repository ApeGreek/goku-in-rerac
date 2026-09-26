# OpenGOAL's VU disassembler (ISC license, see LICENSE), used as a tool only.
find_package(fmt REQUIRED)
add_library(opengoal_vudisasm STATIC
  decompiler/VuDisasm/VuDisassembler.cpp
  decompiler/VuDisasm/VuInstruction.cpp
)
target_include_directories(opengoal_vudisasm PUBLIC ${CMAKE_CURRENT_SOURCE_DIR})
target_link_libraries(opengoal_vudisasm PUBLIC fmt::fmt)
target_compile_options(opengoal_vudisasm PRIVATE -w)
